# Checks and writing advice

`mcq check` reads a paper and reports problems by stable code. **Errors** block publishing;
**warnings** are writing advice and never block; **infos** are neutral facts. Word lists cover
French and English.

Most rules come from published item-writing guidelines (Haladyna, Downing and Rodriguez, 2002)
and from classroom use of the earlier tool.

## Reading the file

| Code | Level | Meaning |
|---|---|---|
| `EMPTY_SOURCE` | error | The file is empty. |
| `NO_FRONTMATTER` | error | The file does not start with a `---` header. |
| `UNCLOSED_FRONTMATTER` | error | The header is never closed. |
| `UNKNOWN_KEY` | warning | Header key not understood (often a typo): ignored. |
| `BAD_VALUE` | warning | Header value not valid for its key: default kept. |
| `THEME_WITHOUT_ID` | error | `# THEME` without an id. |
| `DUPLICATE_THEME` | error | Theme declared twice. |
| `OPTION_WITHOUT_QUESTION` | error | Option line outside a question. |
| `UNCLOSED_CODE_FENCE` | error | A code block is never closed. |
| `BAD_LEVEL` | warning | Level outside 1 to 5. |
| `UNKNOWN_ATTRIBUTE` | warning | Question attribute not understood. |
| `CODE_OUTSIDE_QUESTION` | warning | Code block outside a question: ignored. |
| `ORPHAN_EXPLANATION` | warning | `>` line not directly under an option: ignored. |
| `UNKNOWN_MEDIA` | warning | Media of an unsupported type: ignored. |
| `MEDIA_NOT_FOUND` | error | Media file missing, or outside the paper folder. |

## Structure

| Code | Level | Meaning |
|---|---|---|
| `NO_TITLE` | error | No `title:`. |
| `NO_QUESTION` | error | No question at all. |
| `QUESTION_WITHOUT_ID` | error | A question has no id. |
| `DUPLICATE_QUESTION_ID` | error | Two questions share an id. |
| `EMPTY_STATEMENT` | error | A question has no statement text. |
| `QUESTION_WITHOUT_THEME` | error | A question comes before any theme. |
| `UNKNOWN_THEME` | error | A question points to an undeclared theme. |
| `EMPTY_OPTION` | error | An option has no text. |
| `TOO_FEW_OPTIONS` | error | Fewer than 2 options. |
| `NO_CORRECT_OPTION` | error | No option marked `[x]`. |
| `ALL_OPTIONS_CORRECT` | error | Every option is correct: nothing to tell apart. |
| `DUPLICATE_OPTION` | error | Two options with the same text. |
| `ANSWER_COUNT_MISMATCH` | error | `answers N` disagrees with the options marked correct. |
| `MULTIPLE_CORRECT_OPTIONS` | info | Several correct options: students tick boxes. |

## Writing advice

| Code | Level | Meaning |
|---|---|---|
| `TOO_MANY_OPTIONS` | warning | More than 4 options: reading costs more than choosing. |
| `META_OPTION` | warning | "All/none of the above" can be guessed by elimination and hides which distractor misled. |
| `NEGATIVE_STEM` | warning | The question asks for an exception ("not", "except") without capitals or bold. |
| `MISSING_QUESTION` | warning | The statement never asks anything (no `?`, no final `:`). |
| `VAGUE_STEM` | warning | "What happens?" with options mixing compiling and running. |
| `MEDIA_WITHOUT_ALT` | warning | Media without a text alternative. |
| `NO_EXPLANATION` | warning | No option is explained. |
| `PARTIAL_EXPLANATION` | warning | Some options are not explained. |
| `NO_RULE` | warning | No `RULE:` to remember. |
| `NO_COURSE_REF` | warning | Nowhere to revise: no `COURSE:` on the theme, no `REVIEW:` on the question. |
| `LENGTH_BIAS` | warning | The correct answer is at least 1.6 times longer than the longest distractor. |
| `DUPLICATE_QUESTION` | warning | Statement and code share at least 72 % of their words with another question, in this paper or a paper of the same folder. |

## The paper as a whole

| Code | Level | Meaning |
|---|---|---|
| `DURATION` | info | Question count and estimated time, with a 20 % margin. |
| `TOO_LONG` | warning | More than 90 minutes: remove questions rather than lengthen the session. |
| `DURATION_MISMATCH` | warning | The announced duration is shorter than the estimate. |
| `LEVEL_MISSING` | warning | Questions without a level. |
| `TOO_MUCH_RECALL` | warning | More than 15 % of recall questions (level 1). |
| `NOT_ENOUGH_APPLY` | warning | Fewer than half the questions at levels 3 to 5. |
| `EMPTY_THEME` | warning | A theme without questions. |
| `THIN_THEME` | warning | A theme with 1 or 2 questions: its success rate means little. |
| `LENGTH_RANK_BIAS` | warning | From 10 questions: the correct answer is the longest option in more than 35 % of them. |
| `ANSWER_POSITION_BIAS` | warning | From 10 questions: more than 60 % of the correct answers sit at the same position. |

## In the Studio

The Studio shows every check above as the paper is typed, next to the question concerned, and
adds two about the weight of the media students will download:

| Code | Level | Meaning |
|---|---|---|
| `HEAVY_MEDIA` | warning | One media file above 5 MB: compress or shorten it. |
| `HEAVY_PAPER` | warning | Media above 25 MB in all: slow to open on a phone or a busy network. |
