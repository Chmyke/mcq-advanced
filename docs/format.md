# The `.qcm.md` paper format

A paper is a UTF-8 Markdown file, readable as is, comfortable in Git, and easy to write by hand or
to draft with an AI. Its media live next to it:

```text
my-paper/
  my-paper.qcm.md
  media/
    diagram.png
    dictation.ogg
```

The web editor imports and exports the same folder as a single `.qcmz` file (the folder, zipped).

## A complete example

````markdown
---
title: Fruit and loops
description: Week 3 check
lang: en
duration: 20
penalty: 0.25
unansweredPenalty: 0
scoring: partial
shuffleQuestions: true
shuffleOptions: true
showAnswerCount: true
---
Notes before the first theme are for the author: kept in the source, never shown to students.

# THEME loops | Counting loops
COURSE: Chapter 2, loops

## Q loop-sum-squares | level 3 | 30s
Read this program.

```cs {3}
int total = 0;
for (int i = 1; i <= 3; i++)
    total += i * i;
Console.WriteLine(total);
```

Which statements are true?

- [x] It prints `14`
  > 1 + 4 + 9 = 14.
- [x] The loop body runs three times
- [ ] It prints `6`
  > That would be the sum without squaring.

RULE: Trace a loop one iteration at a time.
REVIEW: Chapter 2, tracing
````

## Header

The file starts with a `---` block of `key: value` lines, closed by `---`. Keys are
case-insensitive; surrounding double quotes are removed. Unknown keys and invalid values are
reported and ignored.

| Key | Value | Default |
|---|---|---|
| `title` | text, required | |
| `description` | text | empty |
| `lang` | language of the paper (`en`, `fr`...) | empty |
| `duration` | minutes, `0` for no limit | `0` |
| `penalty` | points lost per wrong answer (a question is worth 1); `0,25` is accepted | `0` |
| `unansweredPenalty` | points lost per blank question | `0` |
| `scoring` | `all-or-nothing` or `partial` | `all-or-nothing` |
| `shuffleQuestions` | `true`/`false` (also `yes`/`no`, `oui`/`non`, `1`/`0`) | `true` |
| `shuffleOptions` | same | `true` |
| `showAnswerCount` | tell students how many boxes to tick | `true` |

## Themes and questions

- `# THEME <id> | <title>` opens a theme. The title defaults to the id.
- `COURSE: <where>` right under a theme says where to revise it.
- `## Q <id> | <attributes>` opens a question. Ids are never reused, even after a deletion.
  Attributes, all optional, in any order: `level N` (1 remember, 2 understand, 3 apply,
  4 analyse, 5 evaluate) and `Ns`, the expected answering time in seconds. Without a time, the
  level gives one: 10, 15, 20, 30 or 40 seconds (15 without a level). `answers N` announces how
  many options to tick; student copies carry it because they carry no `[x]`. In a paper with
  its answers it must match the options marked correct.
- Every other line until the options is the statement, in Markdown.
- Lines outside any question (before the first theme, or between a theme header and its first
  question) are author notes: editors keep them, packages and student copies drop them.

## Statement content

- **Code**: a fenced block with a language tag, as on Discord or GitHub: ```` ```cs ````,
  ```` ```cpp ````, ```` ```python ````... Lines to emphasise go in braces: ```` ```cs {3,5-6} ````.
  Tabs become four spaces. Text, code and media keep their order.
- **Media**: a line holding only `![text alternative](media/file.ext)`. Images (`png`, `jpg`,
  `gif`, `webp`, `svg`, `avif`), sounds (`mp3`, `ogg`, `wav`, `m4a`, `opus`, `flac`, `weba`) and videos
  (`mp4`, `webm`, `ogv`, `mov`). Paths are relative to the paper and stay inside its folder. The
  text alternative is what a student who cannot see or hear the media gets.

## Options

- `- [x] text` is correct, `- [ ] text` (or `- [] text`) is not. Option ids are positional:
  `<question id>-o1`, `-o2`...
- Several `[x]` make a multiple-answer question: students see check boxes and, with
  `showAnswerCount`, how many to tick.
- Lines starting with `>` right under an option explain it. An empty `>` line starts a new
  paragraph.

## After the options

- `RULE: <text>` is the rule to remember, shown after correction.
- `REVIEW: <where>` points to the course part to revise for this question.

## French keywords

Papers written for the earlier tool keep working: `COURS:` (as `COURSE:`), `REVOIR:` (as
`REVIEW:`), `REGLE:` and `EXPLICATION:` (as `RULE:`), and `niveau N` (as `level N`).

## Scoring

- `all-or-nothing`: a question earns 1 point when exactly the correct options are ticked.
- `partial`: a question earns (correct ticked minus wrong ticked) / correct options, never below
  0. Ticking everything earns nothing.
- Net score = earned points, minus `penalty` per wrong question (no credit at all), minus
  `unansweredPenalty` per blank. The mark is the net score over the question count, never below
  0, times the scale (20, 100...). Time never changes the mark: overtime is only recorded.
