# MCQ Advanced

Multiple-choice quizzes that teach, not only grade. A teacher writes a quiz, the editor points out
badly written questions as they are typed, students take it on any device without an account, and
the results tell the class what to revisit, topic by topic.

**Status: early development.** There is no usable release yet. The core works from the command
line: the paper format, scoring, the pedagogical linter, and the security layer (signed and
encrypted packages, answer key apart, copies sealed for the teacher). The student player and the
teacher's Studio (writing, publishing, correcting and class analysis) run in the browser and on
the desktop. It imports GIFT, Moodle XML, Aiken and CSV banks and exports to Moodle. It is the Rust rewrite of a quiz tool already used in class, whose scoring,
encryption, export and cohort analysis are carried over.

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
- Import from Moodle GIFT, Moodle XML, Aiken and CSV; export to Moodle XML (done), QTI 2.1 next.

## Writing a paper

A paper is a Markdown file: see [the format](docs/format.md). Check it with:

```powershell
cargo run -p mcq-cli -- check my-paper/my-paper.qcm.md
```

Every problem has a stable code, listed in [checks and writing advice](docs/lint-rules.md).

## Bringing in an existing bank

```powershell
cargo run -p mcq-cli -- import bank.gift --out my-paper/my-paper.qcm.md
cargo run -p mcq-cli -- export my-paper/my-paper.qcm.md --to moodle --out bank.xml
cargo run -p mcq-cli -- handout my-paper/my-paper.qcm.md --out my-paper/handout.md
```

GIFT, Moodle XML, Aiken and CSV come in; Moodle XML goes out; the handout is the paper for
students, without answers. What each format keeps is in [exchanging papers](docs/exchange.md).

## Publishing a paper

```powershell
cargo run -p mcq-cli -- identity new teacher.mcqid
cargo run -p mcq-cli -- publish my-paper/my-paper.qcm.md --identity teacher.mcqid --out exam.mcqpack --end-code
```

The first command creates the teacher identity; the second prints the opening code to give when
the exam starts, and an end code that opens the correction later. Both read the passphrase from
the first line of input. See [security](SECURITY.md) for what is guaranteed and what is not.

## Taking a paper

The player opens a `.mcqpack` file, or a link such as `index.html?paper=exam.mcqpack`. It shows the
teacher's fingerprint, asks for the student's name and the opening code, then runs the exam:
coloured code, images, sound and video, single and multiple answers (announced before reading),
flags for review, an overview grid, keyboard keys, light and dark themes, larger text, and a save
after every click so a closed tab loses nothing. At the end it seals the copy and shows a receipt
code; practice papers, or the teacher's end code, show the correction.

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools/build-web.ps1
cargo run -p mcq-player --features desktop
```

The first builds the static web player into `target/web/player`; the second opens the desktop
window.

A link can also say `name=` (pre-filled, the student can change it), `lang=fr` or `lang=en` (the
interface language), and `source=`, the paper's `.qcmz` on the same site: the player then offers
"Open this paper in the editor" once the paper is handed in, which opens the Studio (`../studio/`,
next to the player) with `?open=`. The Studio opens such a paper over its draft, asking first when
the draft would be lost.

## The website demo

[`examples/demo`](examples/demo) holds a fictional six-question practice paper in English and
French: a formula, a picture, coloured code, a multiple-answer question and a true or false.
`crates/mcq-demo` builds its packages and `.qcmz` files byte for byte the same at every build, with
a public demo identity that signs practice packages only, and its tests keep both languages
identical in structure and free of validator warnings.

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools/build-site.ps1 -Out <folder>
```

writes `player/`, `studio/` and `demo/` into the folder, as the website serves them.

## Writing in the Studio

The Studio is the teacher's editor, in the browser or on the desktop, without an account or a
server. Questions are listed by theme with the validator's verdict; the selected question is
edited in the middle (single or multiple answers, true or false, text, code, media, an
explanation per answer, the rule to remember) with the validator's advice beside it and a live
preview exactly as students will see it, on a computer or a phone, light or dark. The Source view
shows and edits the same `.qcm.md` text. Images, sounds and videos are dropped, pasted, chosen or
recorded from the microphone, each with its text alternative. Publishing builds the signed
package, its opening and end codes, and a sheet to print for the exam room with the teacher's
fingerprint. The draft is kept in the browser after every change; the paper and its media go in
and out as one `.qcmz` file.

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools/build-web.ps1 -App studio
cargo run -p mcq-studio --features desktop
```

## Correcting and analysing

In the Studio's Correct space, the teacher drops the package and the students' copies, which
open with the teacher's identity only. Every copy is graded again from the answer key, then the
class's diagnosis reads as sentences, in the order a teacher acts on it: the themes to teach
again, the shared confusions (a distractor chosen beyond chance), the questions to review (a key
the strongest students avoid, questions too hard or too easy, dead distractors), the copies worth
a question (identical answers, many window exits, overtime), and the marks last. Small classes
are told their figures are trends. Each student gets a printable revision sheet; `notes.csv`,
`themes.csv` and `questions.csv` open in any spreadsheet. The rules are in
[class analysis](docs/analysis.md).

## Building and checking

The core crates (`mcq-model`, `mcq-format`, `mcq-grade`, `mcq-lint`) have no dependency;
`mcq-crypto` depends only on RustCrypto and dalek crates. All of them also build for WebAssembly. The single check of the repository runs on Windows PowerShell:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools/check.ps1
```

It runs rustfmt, clippy, the tests, the WebAssembly build of the core and of both apps, and
the desktop build of both apps. It also includes a
content guard: real exam subjects, answer keys, results and keys must never enter
this repository, and the guard refuses them.

## License

GPL-3.0-only. See [LICENSE](LICENSE).
