//! Small binary codecs: the file archive carried inside packages, and submitted copies.
//!
//! Little-endian integers, `u32` length prefixes. Decoding never panics and never trusts a
//! length: every read is bounded by the bytes actually present.

use mcq_model::{
    Answer, CodeTokens, LogEntry, LogKind, Submission, TimeExtension, TokenKind, TokenRun,
};

/// Why bytes could not be decoded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    /// The data ends before a field does.
    Truncated,
    /// Bytes are left after the last field.
    TrailingBytes,
    /// A text field is not UTF-8.
    NotUtf8,
    /// A version this build does not know.
    UnsupportedVersion(u8),
    /// A value breaks a rule of the format.
    Invalid(&'static str),
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecodeError::Truncated => f.write_str("the data is truncated"),
            DecodeError::TrailingBytes => f.write_str("unexpected bytes after the data"),
            DecodeError::NotUtf8 => f.write_str("a text field is not UTF-8"),
            DecodeError::UnsupportedVersion(v) => write!(f, "unsupported version {v}"),
            DecodeError::Invalid(what) => write!(f, "invalid data: {what}"),
        }
    }
}

impl std::error::Error for DecodeError {}

/// Appends fields to a byte buffer.
#[derive(Debug, Default)]
pub struct ByteWriter {
    buf: Vec<u8>,
}

impl ByteWriter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn u8(&mut self, v: u8) -> &mut Self {
        self.buf.push(v);
        self
    }

    pub fn u32(&mut self, v: u32) -> &mut Self {
        self.buf.extend_from_slice(&v.to_le_bytes());
        self
    }

    /// Raw bytes of a known, fixed size.
    pub fn fixed(&mut self, v: &[u8]) -> &mut Self {
        self.buf.extend_from_slice(v);
        self
    }

    /// Length-prefixed bytes.
    ///
    /// # Panics
    /// When `v` is 4 GiB or more, which no field of these formats can reach.
    pub fn bytes(&mut self, v: &[u8]) -> &mut Self {
        let len = u32::try_from(v.len()).expect("field under 4 GiB");
        self.u32(len).fixed(v)
    }

    pub fn str(&mut self, v: &str) -> &mut Self {
        self.bytes(v.as_bytes())
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.buf
    }

    pub fn into_vec(self) -> Vec<u8> {
        self.buf
    }
}

/// Reads fields from a byte slice, front to back.
#[derive(Debug)]
pub struct ByteReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> ByteReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    /// Bytes consumed so far.
    pub fn position(&self) -> usize {
        self.pos
    }

    pub fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }

    pub fn take(&mut self, n: usize) -> Result<&'a [u8], DecodeError> {
        if n > self.remaining() {
            return Err(DecodeError::Truncated);
        }
        let out = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(out)
    }

    pub fn fixed<const N: usize>(&mut self) -> Result<[u8; N], DecodeError> {
        let mut out = [0_u8; N];
        out.copy_from_slice(self.take(N)?);
        Ok(out)
    }

    pub fn u8(&mut self) -> Result<u8, DecodeError> {
        Ok(self.take(1)?[0])
    }

    pub fn u32(&mut self) -> Result<u32, DecodeError> {
        Ok(u32::from_le_bytes(self.fixed()?))
    }

    pub fn bytes(&mut self) -> Result<&'a [u8], DecodeError> {
        let len = self.u32()? as usize;
        self.take(len)
    }

    pub fn str(&mut self) -> Result<&'a str, DecodeError> {
        std::str::from_utf8(self.bytes()?).map_err(|_| DecodeError::NotUtf8)
    }

    /// A count of items that each take at least `min_item_size` bytes, checked against the data
    /// left so that a forged count cannot reserve huge memory.
    pub fn count(&mut self, min_item_size: usize) -> Result<usize, DecodeError> {
        let n = self.u32()? as usize;
        if n.saturating_mul(min_item_size.max(1)) > self.remaining() {
            return Err(DecodeError::Truncated);
        }
        Ok(n)
    }

    pub fn finish(&self) -> Result<(), DecodeError> {
        if self.remaining() == 0 {
            Ok(())
        } else {
            Err(DecodeError::TrailingBytes)
        }
    }
}

/// A file carried inside a package: the paper text or a media file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// Relative path with `/` separators, such as `paper.qcm.md` or `media/apple.png`.
    pub path: String,
    pub data: Vec<u8>,
}

const ARCHIVE_VERSION: u8 = 1;
const TOKENS_VERSION: u8 = 1;

/// Name of the highlighted code inside package archives.
pub const CODE_TOKENS_FILE: &str = "code.tokens";
/// Version 2 adds the declared time extension, version 3 the questions reported as unclear;
/// older copies still open, without them.
const SUBMISSION_VERSION: u8 = 3;

/// Encodes files in the given order.
pub fn encode_archive(entries: &[Entry]) -> Vec<u8> {
    let mut w = ByteWriter::new();
    w.u8(ARCHIVE_VERSION);
    w.u32(u32::try_from(entries.len()).expect("fewer than 4 billion files"));
    for entry in entries {
        w.str(&entry.path).bytes(&entry.data);
    }
    w.into_vec()
}

/// Decodes an archive, refusing unsafe or repeated paths.
pub fn decode_archive(data: &[u8]) -> Result<Vec<Entry>, DecodeError> {
    let mut r = ByteReader::new(data);
    let version = r.u8()?;
    if version != ARCHIVE_VERSION {
        return Err(DecodeError::UnsupportedVersion(version));
    }
    let count = r.count(8)?;
    let mut entries: Vec<Entry> = Vec::with_capacity(count);
    for _ in 0..count {
        let path = r.str()?;
        if !is_safe_path(path) {
            return Err(DecodeError::Invalid("unsafe file path"));
        }
        if entries.iter().any(|e| e.path == path) {
            return Err(DecodeError::Invalid("repeated file path"));
        }
        let data = r.bytes()?.to_vec();
        entries.push(Entry {
            path: path.to_string(),
            data,
        });
    }
    r.finish()?;
    Ok(entries)
}

/// Relative, `/`-separated, without empty, `.` or `..` parts, without `\` or `:`.
pub fn is_safe_path(path: &str) -> bool {
    !path.is_empty()
        && !path.contains(['\\', ':', '\0'])
        && path
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

pub fn encode_submission(s: &Submission) -> Vec<u8> {
    let mut w = ByteWriter::new();
    w.u8(SUBMISSION_VERSION)
        .fixed(&s.paper_id)
        .str(&s.student_name)
        .u32(s.elapsed_seconds)
        .u32(s.focus_losses)
        .u32(s.seconds_away)
        .u8(s.extension.code())
        .u32(u32::try_from(s.answers.len()).expect("fewer than 4 billion answers"));
    for answer in &s.answers {
        w.str(&answer.question_id)
            .u32(answer.seconds_spent)
            .u8(u8::from(answer.reported))
            .u32(u32::try_from(answer.selected.len()).expect("fewer than 4 billion options"));
        for id in &answer.selected {
            w.str(id);
        }
    }
    w.u32(u32::try_from(s.log.len()).expect("fewer than 4 billion events"));
    for entry in &s.log {
        w.u32(entry.seconds)
            .u8(entry.kind.code())
            .str(&entry.question_id)
            .str(&entry.option_id);
    }
    w.into_vec()
}

pub fn decode_submission(data: &[u8]) -> Result<Submission, DecodeError> {
    let mut r = ByteReader::new(data);
    let version = r.u8()?;
    if !(1..=SUBMISSION_VERSION).contains(&version) {
        return Err(DecodeError::UnsupportedVersion(version));
    }
    let paper_id = r.fixed()?;
    let student_name = r.str()?.to_string();
    let elapsed_seconds = r.u32()?;
    let focus_losses = r.u32()?;
    let seconds_away = r.u32()?;
    let extension = if version >= 2 {
        TimeExtension::from_code(r.u8()?).ok_or(DecodeError::Invalid("unknown time extension"))?
    } else {
        TimeExtension::None
    };
    let count = r.count(12)?;
    let mut answers = Vec::with_capacity(count);
    for _ in 0..count {
        let question_id = r.str()?.to_string();
        let seconds_spent = r.u32()?;
        let reported = if version >= 3 {
            match r.u8()? {
                0 => false,
                1 => true,
                _ => return Err(DecodeError::Invalid("reported flag")),
            }
        } else {
            false
        };
        let n = r.count(4)?;
        let mut selected = Vec::with_capacity(n);
        for _ in 0..n {
            selected.push(r.str()?.to_string());
        }
        answers.push(Answer {
            question_id,
            selected,
            seconds_spent,
            reported,
        });
    }
    let count = r.count(13)?;
    let mut log = Vec::with_capacity(count);
    for _ in 0..count {
        let seconds = r.u32()?;
        let kind = LogKind::from_code(r.u8()?).ok_or(DecodeError::Invalid("unknown event"))?;
        log.push(LogEntry {
            seconds,
            kind,
            question_id: r.str()?.to_string(),
            option_id: r.str()?.to_string(),
        });
    }
    r.finish()?;
    Ok(Submission {
        paper_id,
        student_name,
        answers,
        elapsed_seconds,
        focus_losses,
        seconds_away,
        log,
        extension,
    })
}

pub fn encode_code_tokens(blocks: &[CodeTokens]) -> Vec<u8> {
    let mut w = ByteWriter::new();
    w.u8(TOKENS_VERSION)
        .u32(u32::try_from(blocks.len()).expect("fewer than 4 billion blocks"));
    for block in blocks {
        w.str(&block.question_id)
            .u32(block.block)
            .u32(u32::try_from(block.lines.len()).expect("fewer than 4 billion lines"));
        for line in &block.lines {
            w.u32(u32::try_from(line.len()).expect("fewer than 4 billion runs"));
            for run in line {
                w.u8(run.kind.code()).str(&run.text);
            }
        }
    }
    w.into_vec()
}

pub fn decode_code_tokens(data: &[u8]) -> Result<Vec<CodeTokens>, DecodeError> {
    let mut r = ByteReader::new(data);
    let version = r.u8()?;
    if version != TOKENS_VERSION {
        return Err(DecodeError::UnsupportedVersion(version));
    }
    let count = r.count(12)?;
    let mut blocks = Vec::with_capacity(count);
    for _ in 0..count {
        let question_id = r.str()?.to_string();
        let block = r.u32()?;
        let n_lines = r.count(4)?;
        let mut lines = Vec::with_capacity(n_lines);
        for _ in 0..n_lines {
            let n_runs = r.count(5)?;
            let mut runs = Vec::with_capacity(n_runs);
            for _ in 0..n_runs {
                let kind =
                    TokenKind::from_code(r.u8()?).ok_or(DecodeError::Invalid("token kind"))?;
                runs.push(TokenRun {
                    kind,
                    text: r.str()?.to_string(),
                });
            }
            lines.push(runs);
        }
        blocks.push(CodeTokens {
            question_id,
            block,
            lines,
        });
    }
    r.finish()?;
    Ok(blocks)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_submission() -> Submission {
        Submission {
            paper_id: [7; 16],
            student_name: "Alex Example".into(),
            answers: vec![
                Answer {
                    question_id: "q1".into(),
                    selected: vec!["q1-o2".into()],
                    seconds_spent: 12,
                    reported: true,
                },
                Answer {
                    question_id: "q2".into(),
                    selected: vec![],
                    seconds_spent: 3,
                    reported: false,
                },
            ],
            elapsed_seconds: 300,
            focus_losses: 1,
            seconds_away: 4,
            log: vec![
                LogEntry::new(10, LogKind::Tick, "q1", "q1-o2"),
                LogEntry::new(40, LogKind::FocusLost, "", ""),
            ],
            extension: TimeExtension::Third,
        }
    }

    #[test]
    fn code_tokens_round_trip() {
        let blocks = vec![CodeTokens {
            question_id: "q1".into(),
            block: 0,
            lines: vec![
                vec![
                    TokenRun {
                        kind: TokenKind::Keyword,
                        text: "int".into(),
                    },
                    TokenRun {
                        kind: TokenKind::Plain,
                        text: " x = ".into(),
                    },
                ],
                vec![],
            ],
        }];
        let bytes = encode_code_tokens(&blocks);
        assert_eq!(decode_code_tokens(&bytes), Ok(blocks));
        for len in 0..bytes.len() {
            assert!(decode_code_tokens(&bytes[..len]).is_err(), "length {len}");
        }
    }

    #[test]
    fn submission_round_trips() {
        let s = sample_submission();
        assert_eq!(decode_submission(&encode_submission(&s)), Ok(s));
    }

    /// Offset of the extension byte, right after seconds_away.
    fn extension_at(s: &Submission) -> usize {
        1 + 16 + 4 + s.student_name.len() + 4 + 4 + 4
    }

    /// The copy as a version 2 build wrote it: the same bytes without the reported flags, each
    /// right after its answer's seconds_spent.
    fn version_2_bytes(s: &Submission) -> Vec<u8> {
        let mut bytes = encode_submission(s);
        let mut at = extension_at(s) + 1 + 4;
        for answer in &s.answers {
            at += 4 + answer.question_id.len() + 4;
            bytes.remove(at);
            at += 4 + answer.selected.iter().map(|o| 4 + o.len()).sum::<usize>();
        }
        bytes[0] = 2;
        bytes
    }

    fn without_reports(answers: &[Answer]) -> Vec<Answer> {
        answers
            .iter()
            .map(|a| Answer {
                reported: false,
                ..a.clone()
            })
            .collect()
    }

    #[test]
    fn version_2_copies_still_open_without_reports() {
        let s = sample_submission();
        let old = decode_submission(&version_2_bytes(&s)).expect("version 2");
        assert_eq!(old.extension, s.extension);
        assert_eq!(old.answers, without_reports(&s.answers));
        assert_eq!(old.log, s.log);
    }

    #[test]
    fn version_1_copies_still_open_without_extension() {
        let s = sample_submission();
        let mut bytes = version_2_bytes(&s);
        bytes.remove(extension_at(&s));
        bytes[0] = 1;
        let old = decode_submission(&bytes).expect("version 1");
        assert_eq!(old.extension, TimeExtension::None);
        assert_eq!(old.answers, without_reports(&s.answers));
        assert_eq!(old.log, s.log);
    }

    #[test]
    fn unknown_extension_or_report_flag_is_refused() {
        let s = sample_submission();
        let mut bytes = encode_submission(&s);
        bytes[extension_at(&s)] = 9;
        assert!(decode_submission(&bytes).is_err());
        let mut bytes = encode_submission(&s);
        let first_report = extension_at(&s) + 1 + 4 + 4 + s.answers[0].question_id.len() + 4;
        assert_eq!(bytes[first_report], 1);
        bytes[first_report] = 2;
        assert!(decode_submission(&bytes).is_err());
    }

    #[test]
    fn archive_round_trips() {
        let entries = vec![
            Entry {
                path: "paper.qcm.md".into(),
                data: b"---\ntitle: T\n---\n".to_vec(),
            },
            Entry {
                path: "media/a.png".into(),
                data: vec![0, 1, 2],
            },
        ];
        assert_eq!(decode_archive(&encode_archive(&entries)), Ok(entries));
    }

    #[test]
    fn every_truncation_and_extension_is_refused() {
        let bytes = encode_submission(&sample_submission());
        for len in 0..bytes.len() {
            assert!(decode_submission(&bytes[..len]).is_err(), "length {len}");
        }
        let mut longer = bytes.clone();
        longer.push(0);
        assert_eq!(decode_submission(&longer), Err(DecodeError::TrailingBytes));
    }

    #[test]
    fn forged_counts_do_not_allocate() {
        let mut w = ByteWriter::new();
        w.u8(ARCHIVE_VERSION).u32(u32::MAX);
        assert_eq!(decode_archive(w.as_slice()), Err(DecodeError::Truncated));
    }

    #[test]
    fn unsafe_paths_are_refused() {
        for path in [
            "", "/etc/x", "../x", "a/../b", "a//b", "./a", "c:\\x", "a\\b", "a/",
        ] {
            assert!(!is_safe_path(path), "{path}");
        }
        assert!(is_safe_path("media/apple.png"));
        let bad = encode_archive(&[Entry {
            path: "../evil".into(),
            data: vec![],
        }]);
        assert_eq!(
            decode_archive(&bad),
            Err(DecodeError::Invalid("unsafe file path"))
        );
    }

    #[test]
    fn repeated_paths_are_refused() {
        let entry = Entry {
            path: "a".into(),
            data: vec![],
        };
        let bytes = encode_archive(&[entry.clone(), entry]);
        assert_eq!(
            decode_archive(&bytes),
            Err(DecodeError::Invalid("repeated file path"))
        );
    }
}
