//! Finds the paper inside text pasted from an AI's answer, which often comes with a sentence
//! before it and a code fence around it despite being asked for the file alone.

/// The paper in `text`, from its opening `---` line on: an introduction before it is dropped, and
/// when a code fence opens right before the header, everything from its closing fence on as well.
/// `None` when no header opens and closes, which tells pasted prose from a paper.
pub fn paper_in_pasted(text: &str) -> Option<&str> {
    let lines: Vec<(usize, &str)> = text
        .split_inclusive('\n')
        .scan(0, |at, line| {
            let start = *at;
            *at += line.len();
            Some((start, line))
        })
        .collect();
    let is_rule = |line: &str| line.trim_end() == "---";
    let open = lines.iter().position(|(_, l)| is_rule(l))?;
    lines[open + 1..].iter().position(|(_, l)| is_rule(l))?;
    let start = lines[open].0;
    let before = lines[..open]
        .iter()
        .rev()
        .map(|(_, l)| l.trim())
        .find(|l| !l.is_empty())
        .unwrap_or("");
    // A fence is three or more backticks or tildes, followed by an optional language.
    let marks = before
        .find(|c: char| c != '`' && c != '~')
        .unwrap_or(before.len());
    let fence = Some(&before[..marks])
        .filter(|f| f.len() >= 3 && (f.chars().all(|c| c == '`') || f.chars().all(|c| c == '~')));
    let end = match fence {
        Some(fence) => lines[open + 1..]
            .iter()
            .rev()
            .find(|(_, l)| l.trim() == fence)
            .map_or(text.len(), |(at, _)| *at),
        None => text.len(),
    };
    Some(&text[start..end])
}

#[cfg(test)]
mod tests {
    use super::paper_in_pasted;

    const PAPER: &str = "---\ntitle: T\n---\n\n# THEME a | A\n\n## Q q1\nWhy?\n\n```python\nx = 1\n```\n\n- [x] Yes\n- [ ] No\n";

    #[test]
    fn a_bare_paper_is_kept_whole() {
        assert_eq!(paper_in_pasted(PAPER), Some(PAPER));
    }

    #[test]
    fn the_introduction_and_the_fence_around_go() {
        for fence in ["```", "```markdown", "````md", "~~~"] {
            let mark = fence.trim_end_matches(char::is_alphabetic);
            let text = format!("Here is your test:\n\n{fence}\n{PAPER}{mark}\n\nGood luck!\n");
            assert_eq!(paper_in_pasted(&text), Some(PAPER), "{fence}");
        }
    }

    #[test]
    fn code_inside_the_paper_does_not_close_a_longer_fence() {
        let text = format!("````\n{PAPER}````\n");
        assert_eq!(paper_in_pasted(&text), Some(PAPER));
    }

    #[test]
    fn windows_line_ends_are_kept() {
        let paper = PAPER.replace('\n', "\r\n");
        let text = format!("Voici :\r\n```\r\n{paper}```\r\n");
        assert_eq!(paper_in_pasted(&text), Some(paper.as_str()));
    }

    #[test]
    fn prose_or_an_unclosed_header_is_no_paper() {
        assert_eq!(paper_in_pasted("Just a sentence."), None);
        assert_eq!(paper_in_pasted("---\ntitle: T\n"), None);
        assert_eq!(paper_in_pasted(""), None);
    }
}
