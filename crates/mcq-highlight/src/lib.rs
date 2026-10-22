//! ROLE:    turns code blocks into coloured tokens when a paper is published, so players stay
//!          light: they only map token kinds to the colours of the light or dark theme.
//! OWNER:   stateless; `Highlighter` owns the grammars, loaded once.
//! INVARIANTS: the token texts of a line, joined, give the source line back exactly;
//!             an unknown language gives plain tokens, never an error.

use mcq_model::{Block, CodeTokens, Paper, TokenKind, TokenRun};
use syntect::parsing::{ParseState, Scope, ScopeStack, ScopeStackOp, SyntaxReference, SyntaxSet};

/// The grammars of Sublime Text's default packages (C#, C++, C, Java, Python, JavaScript, Rust,
/// SQL, HTML, CSS, JSON, shell and others).
pub struct Highlighter {
    syntaxes: SyntaxSet,
    rules: Vec<(Scope, TokenKind)>,
}

impl Default for Highlighter {
    fn default() -> Self {
        Self::new()
    }
}

/// Scope prefixes, most specific first: the first one found in a token's scope stack, from the
/// innermost scope outwards, gives its kind.
const RULES: [(&str, TokenKind); 22] = [
    ("comment", TokenKind::Comment),
    ("string", TokenKind::String),
    ("constant.numeric", TokenKind::Number),
    ("constant.character.escape", TokenKind::String),
    ("constant", TokenKind::Constant),
    ("keyword.operator.word", TokenKind::Keyword),
    ("keyword.operator", TokenKind::Operator),
    ("keyword", TokenKind::Keyword),
    ("storage.type.function", TokenKind::Keyword),
    ("storage.type.class", TokenKind::Keyword),
    ("storage.type.struct", TokenKind::Keyword),
    ("storage.type", TokenKind::Type),
    ("storage.modifier", TokenKind::Keyword),
    ("storage", TokenKind::Keyword),
    ("entity.name.function", TokenKind::Function),
    ("support.function", TokenKind::Function),
    ("variable.function", TokenKind::Function),
    ("entity.name", TokenKind::Type),
    ("support.type", TokenKind::Type),
    ("support.class", TokenKind::Type),
    ("variable.language", TokenKind::Constant),
    ("meta.preprocessor", TokenKind::Keyword),
];

impl Highlighter {
    pub fn new() -> Self {
        let rules = RULES
            .iter()
            .filter_map(|(scope, kind)| Scope::new(scope).ok().map(|s| (s, *kind)))
            .collect();
        Self {
            syntaxes: SyntaxSet::load_defaults_newlines(),
            rules,
        }
    }

    /// The grammar for a fence tag, with the aliases people type on Discord or GitHub.
    fn syntax(&self, language: &str) -> Option<&SyntaxReference> {
        let tag = language.trim().to_ascii_lowercase();
        let token = match tag.as_str() {
            "" | "text" | "plain" | "txt" => return None,
            "c#" | "csharp" | "cs" => "cs",
            "c++" | "cpp" | "cxx" | "cc" | "hpp" => "cpp",
            "py" | "python" => "py",
            "js" | "javascript" | "jsx" => "js",
            "rs" | "rust" => "rs",
            "sh" | "bash" | "shell" | "zsh" => "sh",
            "yml" | "yaml" => "yaml",
            other => other,
        };
        self.syntaxes
            .find_syntax_by_token(token)
            .or_else(|| self.syntaxes.find_syntax_by_extension(token))
    }

    /// True when the language gets colours rather than plain text.
    pub fn knows(&self, language: &str) -> bool {
        self.syntax(language).is_some()
    }

    /// One token list per source line.
    pub fn highlight(&self, language: &str, source: &str) -> Vec<Vec<TokenRun>> {
        let Some(syntax) = self.syntax(language) else {
            return source.split('\n').map(plain).collect();
        };
        let mut state = ParseState::new(syntax);
        let mut stack = ScopeStack::new();
        let mut out = Vec::new();
        for line in source.split('\n') {
            // The grammars expect a newline at the end of each line.
            let with_newline = format!("{line}\n");
            let Ok(ops) = state.parse_line(&with_newline, &self.syntaxes) else {
                // A grammar failure on odd input degrades to plain text from here on.
                out.push(plain(line));
                state = ParseState::new(syntax);
                stack = ScopeStack::new();
                continue;
            };
            out.push(self.runs(line, &ops, &mut stack));
        }
        out
    }

    fn runs(
        &self,
        line: &str,
        ops: &[(usize, ScopeStackOp)],
        stack: &mut ScopeStack,
    ) -> Vec<TokenRun> {
        let mut runs: Vec<TokenRun> = Vec::new();
        let mut start = 0;
        let push = |runs: &mut Vec<TokenRun>, text: &str, kind: TokenKind| {
            if text.is_empty() {
                return;
            }
            match runs.last_mut() {
                Some(last) if last.kind == kind => last.text.push_str(text),
                _ => runs.push(TokenRun {
                    kind,
                    text: text.to_string(),
                }),
            }
        };
        for (offset, op) in ops {
            let end = (*offset).min(line.len());
            if end > start && line.is_char_boundary(end) {
                push(&mut runs, &line[start..end], self.kind(stack));
                start = end;
            }
            // A malformed op only loses colours, never text.
            let _ = stack.apply(op);
        }
        if start < line.len() {
            push(&mut runs, &line[start..], self.kind(stack));
        }
        runs
    }

    fn kind(&self, stack: &ScopeStack) -> TokenKind {
        for scope in stack.as_slice().iter().rev() {
            for (prefix, kind) in &self.rules {
                if prefix.is_prefix_of(*scope) {
                    return *kind;
                }
            }
        }
        TokenKind::Plain
    }

    /// Tokens for every code block of a paper whose language is known.
    pub fn paper(&self, paper: &Paper) -> Vec<CodeTokens> {
        let mut out = Vec::new();
        for question in &paper.questions {
            let code_blocks = question.body.iter().filter_map(|b| match b {
                Block::Code(code) => Some(code),
                _ => None,
            });
            for (index, code) in code_blocks.enumerate() {
                if !self.knows(&code.language) {
                    continue;
                }
                out.push(CodeTokens {
                    question_id: question.id.clone(),
                    block: u32::try_from(index).unwrap_or(u32::MAX),
                    lines: self.highlight(&code.language, &code.source),
                });
            }
        }
        out
    }
}

fn plain(line: &str) -> Vec<TokenRun> {
    if line.is_empty() {
        Vec::new()
    } else {
        vec![TokenRun {
            kind: TokenKind::Plain,
            text: line.to_string(),
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text_of(lines: &[Vec<TokenRun>]) -> String {
        lines
            .iter()
            .map(|l| l.iter().map(|r| r.text.as_str()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn kind_of(lines: &[Vec<TokenRun>], text: &str) -> Option<TokenKind> {
        lines
            .iter()
            .flatten()
            .find(|r| r.text.trim() == text)
            .map(|r| r.kind)
    }

    #[test]
    fn csharp_gets_the_usual_colours() {
        let h = Highlighter::new();
        let src = "// sum\nint total = 0;\nfor (int i = 1; i <= 3; i++)\n    total += i * i;\nConsole.WriteLine(\"done\");";
        let lines = h.highlight("cs", src);
        assert_eq!(text_of(&lines), src, "text is kept exactly");
        assert_eq!(kind_of(&lines, "// sum"), Some(TokenKind::Comment));
        assert_eq!(kind_of(&lines, "for"), Some(TokenKind::Keyword));
        assert_eq!(kind_of(&lines, "\"done\""), Some(TokenKind::String));
        assert!(lines.iter().flatten().any(|r| r.kind == TokenKind::Number));
    }

    #[test]
    fn aliases_and_other_languages() {
        let h = Highlighter::new();
        for tag in [
            "c#", "CSharp", "cpp", "c++", "c", "python", "py", "js", "rust", "java", "sql", "json",
            "bash", "html", "css",
        ] {
            assert!(h.knows(tag), "{tag}");
        }
        assert!(!h.knows(""));
        assert!(!h.knows("klingon"));
        let lines = h.highlight("cpp", "#include <vector>\nstd::vector<int> v;");
        assert_eq!(text_of(&lines), "#include <vector>\nstd::vector<int> v;");
    }

    #[test]
    fn unknown_language_is_plain_and_exact() {
        let h = Highlighter::new();
        let lines = h.highlight("klingon", "qapla'\n\n  batlh");
        assert_eq!(text_of(&lines), "qapla'\n\n  batlh");
        assert!(lines.iter().flatten().all(|r| r.kind == TokenKind::Plain));
    }

    #[test]
    fn odd_input_keeps_its_text() {
        let h = Highlighter::new();
        for src in ["\"unclosed", "/* open", "é ü 中文 \u{1F600}", "\t\tx", ""] {
            assert_eq!(text_of(&h.highlight("cs", src)), src);
        }
    }
}
