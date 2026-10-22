# MCQ Advanced

Multiple-choice quizzes that teach, not only grade. A teacher writes a quiz, the editor points out
badly written questions as they are typed, students take it on any device without an account, and
the results tell the class what to revisit, topic by topic.

**Status: early development.** There is no usable release yet. The core is being written: the
paper format, scoring and the pedagogical linter work from the command line; security, the player
and the editor come next. It is the Rust rewrite of a quiz tool already used in class, whose
scoring, encryption, export and cohort analysis are carried over.

## Principles

- **Diagnose, explain, then grade.** Mastery per category, an explanation for every option, the
  grade last.
- **No server, no account, no tracking.** Everything runs on the teacher's and the students'
  devices. Student data never leaves them unless the teacher moves a file.
- **Honest security.** The code is public, so protection comes from keys alone, built from
  standard cryptography, with a published threat model that says what is guaranteed and what is
  not.
- **Text first.** A quiz is a readable Markdown file: easy to write by hand, to keep in Git, or to
  draft with an AI. A visual editor works on that same file.
- **Accessible.** Target: WCAG 2.2 AA, including adjustable time limits.

## Planned

**Editor (for the teacher)**
- Single and multiple correct answers, an explanation per option, categories, difficulty, code,
  images and formulas.
- Settings: global, per-question or no time limit; penalties for wrong or blank answers; partial
  credit; certainty-based marking; random draw of N questions out of M; shuffling; introduction
  text; what students see at the end (nothing, score, mastery, full correction).
- A pedagogical linter, based on published item-writing guidelines (Haladyna, Downing and
  Rodriguez): longest-answer bias, answer position bias, negative stems, "all of the above",
  near-duplicate questions, unbalanced categories.

**Player (for the student)**
- Web first (works offline once loaded), then desktop; phones through the browser.
- Keyboard navigation, flag for review, overview grid, autosave and resume after a crash.

**Security**
- The quiz is signed by the teacher and encrypted; it opens with a code given at the start of the
  exam, generated or chosen by the teacher.
- The answer key is encrypted separately, out of the students' reach during the exam.
- Answers are sealed for the teacher's key before any correction is shown.
- Per-student draw and shuffle; focus loss is logged and reported, not hidden.

**Grading and analysis**
- Re-grading from the teacher's side, mastery per category, item difficulty, discrimination
  (upper and lower 27 %), point-biserial, distractor analysis, KR-20 reliability, anomaly report,
  CSV export.

**Interoperability**
- Import from Moodle GIFT, Moodle XML and Aiken; export to Moodle XML and QTI 2.1.

## Writing a paper

A paper is a Markdown file: see [the format](docs/format.md). Check it with:

```powershell
cargo run -p mcq-cli -- check my-paper/my-paper.qcm.md
```

Every problem has a stable code, listed in [checks and writing advice](docs/lint-rules.md).

## Building and checking

The core crates (`mcq-model`, `mcq-format`, `mcq-grade`, `mcq-lint`) have no dependency and also
build for WebAssembly. The single check of the repository runs on Windows PowerShell:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools/check.ps1
```

It runs rustfmt, clippy, the tests and the WebAssembly build of the core. It also includes a
content guard: real exam subjects, answer keys, results and keys must never enter
this repository, and the guard refuses them.

## License

GPL-3.0-only. See [LICENSE](LICENSE).
