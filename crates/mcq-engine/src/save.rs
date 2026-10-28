//! Autosave: the whole session in a few hundred bytes, written after every change, so a crash
//! or a closed tab loses nothing. It holds the student's own answers only, never the paper.

use mcq_format::binary::{ByteReader, ByteWriter, DecodeError};
use mcq_model::{LogEntry, LogKind, TimeExtension};

use crate::{Session, SessionItem};

const MAGIC: &[u8; 4] = b"MCQS";
/// Version 2 adds the declared time extension, version 3 the reported questions; older saves
/// resume without them.
const VERSION: u8 = 3;

pub fn encode_session(s: &Session) -> Vec<u8> {
    let mut w = ByteWriter::new();
    w.fixed(MAGIC)
        .u8(VERSION)
        .fixed(&s.paper_id)
        .str(&s.student_name)
        .u32(index(s.current))
        .u32(s.elapsed_seconds)
        .u8(u8::from(s.submitted))
        .u32(s.focus_losses)
        .u32(s.seconds_away)
        .u8(u8::from(s.pace_alerted))
        .u8(s.extension.code())
        .u32(index(s.items.len()));
    for item in &s.items {
        w.str(&item.question_id)
            .u8(u8::from(item.flagged))
            .u8(u8::from(item.reported))
            .u32(item.seconds_spent)
            .u32(index(item.option_order.len()));
        for id in &item.option_order {
            w.str(id);
        }
        w.u32(index(item.selected.len()));
        for id in &item.selected {
            w.str(id);
        }
    }
    w.u32(index(s.log.len()));
    for entry in &s.log {
        w.u32(entry.seconds)
            .u8(entry.kind.code())
            .str(&entry.question_id)
            .str(&entry.option_id);
    }
    w.into_vec()
}

pub fn decode_session(data: &[u8]) -> Result<Session, DecodeError> {
    let mut r = ByteReader::new(data);
    if r.take(4)? != MAGIC {
        return Err(DecodeError::Invalid("not a saved session"));
    }
    let version = r.u8()?;
    if !(1..=VERSION).contains(&version) {
        return Err(DecodeError::UnsupportedVersion(version));
    }
    let paper_id = r.fixed()?;
    let student_name = r.str()?.to_string();
    let current = r.u32()? as usize;
    let elapsed_seconds = r.u32()?;
    let submitted = flag(r.u8()?)?;
    let focus_losses = r.u32()?;
    let seconds_away = r.u32()?;
    let pace_alerted = flag(r.u8()?)?;
    let extension = if version >= 2 {
        TimeExtension::from_code(r.u8()?).ok_or(DecodeError::Invalid("unknown time extension"))?
    } else {
        TimeExtension::None
    };
    let count = r.count(13)?;
    let mut items = Vec::with_capacity(count);
    for _ in 0..count {
        let question_id = r.str()?.to_string();
        let flagged = flag(r.u8()?)?;
        let reported = if version >= 3 { flag(r.u8()?)? } else { false };
        let seconds_spent = r.u32()?;
        let option_order = strings(&mut r)?;
        let selected = strings(&mut r)?;
        if selected.iter().any(|s| !option_order.contains(s)) {
            return Err(DecodeError::Invalid("selected option not shown"));
        }
        items.push(SessionItem {
            question_id,
            option_order,
            selected,
            flagged,
            reported,
            seconds_spent,
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
    if current >= items.len().max(1) {
        return Err(DecodeError::Invalid("current question out of range"));
    }
    Ok(Session {
        paper_id,
        student_name,
        items,
        current,
        elapsed_seconds,
        submitted,
        focus_losses,
        seconds_away,
        log,
        pace_alerted,
        extension,
    })
}

fn index(n: usize) -> u32 {
    u32::try_from(n).expect("fewer than 4 billion items")
}

fn flag(byte: u8) -> Result<bool, DecodeError> {
    match byte {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(DecodeError::Invalid("flag")),
    }
}

fn strings(r: &mut ByteReader<'_>) -> Result<Vec<String>, DecodeError> {
    let n = r.count(4)?;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        out.push(r.str()?.to_string());
    }
    Ok(out)
}
