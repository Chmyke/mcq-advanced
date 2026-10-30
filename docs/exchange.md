# Exchanging papers with other tools

MCQ Advanced imports the question banks teachers already have, exports to Moodle, and prints a
paper for students. Everything runs locally, from the command line or the Studio.

```powershell
mcq import bank.gift --out my-paper/my-paper.qcm.md
mcq import bank.xml --out my-paper.qcmz --lang fr
mcq export my-paper/my-paper.qcm.md --to moodle --out bank.xml
mcq handout my-paper/my-paper.qcm.md --out my-paper/handout.md
```

In the Studio, **Import** opens the same files (or drop them on the page), **Export Moodle XML**
writes a bank, and **Paper without answers** downloads the handout.

Only single-answer, multiple-answer and true or false questions come in. Every other question
(short answer, numerical, matching, essay, cloze, description) is left out and named in the
import report, as is every detail that is simplified on the way. Run `mcq check` on the result:
the pedagogical checks often find something to improve in an old bank.

## GIFT

Moodle's text format, read as Moodle reads it:

- questions are separated by empty lines; `//` lines are comments;
- `::name::` gives the question its id (as a slug);
- `$CATEGORY: $course$/top/Unit/Topic` puts the following questions in the theme `Topic`;
- `=right ~wrong`, weights `~%50%`, feedback per answer `#...`, general feedback `####...`
  (it becomes the rule shown after correction);
- `{T}`, `{TRUE}`, `{F}`, `{FALSE}` with their feedbacks;
- `\~ \= \# \{ \} \: \\ \n` escapes;
- `[html]`, `[moodle]`, `[plain]` and `[markdown]` text formats;
- in comments, `[id:...]` sets the id, `[tag:mcq-level-3]` and `[tag:mcq-time-30]` the level and
  the time;
- text after the answers makes a gap: `Light reaches us in {=8 minutes ~8 hours} or so.`

## Moodle XML

The file Moodle writes when it exports a question bank.

| Moodle | MCQ Advanced |
|---|---|
| `multichoice`, `truefalse` | questions; other types are left out |
| category path, last name | theme (title); its id number is the theme id, its description the place to revise |
| common parent category | paper title and description (not Moodle's own `Default for ...`) |
| `idnumber`, else `name` | question id |
| `questiontext` (HTML, auto format, plain text, Markdown) | statement: text, `<pre>` code, images, sounds, videos |
| `<file>` attached to the text | media, stored under `media/` |
| answer `fraction` | correct when it earns credit (the best ones on a single-answer question) |
| answer feedback | option explanation |
| `generalfeedback` | rule; a `<p class="mcq-review">` paragraph is the review pointer |
| tags `mcq-level-N`, `mcq-time-N` | level and time in seconds |
| `shuffleanswers` | option shuffling, when all questions agree |
| `single` false | several answers; the paper then grades with partial credit |

Images on answers and external links have no place in an option: their text alternative is kept
and the report says so. A question worth more than one point in Moodle is worth one here.

### Export

`mcq export` writes one category per paper (with its description) and one per theme (its id
number is the theme id), then each question as `multichoice`, media embedded. Multiple-answer
questions get Moodle's partial credit: each correct option is worth 100/n %, each wrong one
-100/n %, which is this tool's own partial credit.

Formulas leave as `\(...\)` and `\[...\]`, the delimiters of Moodle's MathJax filter, and the
import turns them, and `$$...$$`, back into `$...$` and `$$...$$`; any other dollar sign is a
dollar. Code leaves coloured as in the player's light theme, highlighted lines on a pale
background; the import keeps its text, language and highlighted lines. Moodle cannot tell
students how many answers to tick before they answer, so `showAnswerCount` stays behind.

To see how a Moodle site shows all of this, export `examples/moodle-check/moodle-check.qcm.md`
(formulas, prices, coloured code, picture, sound, video, several answers, true or false, text
styles) and import it into a test question bank: each statement says what it should show.

A bank holds questions, not quiz settings: duration, penalties, question order and author notes
stay behind (the export says which), to be set in the Moodle quiz. Everything else comes back
unchanged when the bank is imported again, and exporting it again gives the same file: the tests
check both on a fictional paper and, locally, on real ones.

## Aiken

```text
Which gas do plants take in?
A) Oxygen
B) Carbon dioxide
ANSWER: B
```

As in Moodle, with two tolerances: the question may span several lines, and `ANSWER: A, C`
marks several correct options. Options are `A)` or `A.`; questions go to one theme named after
the file.

## CSV

One question per row; the first row names the columns. Names are matched without case, accents,
spaces or punctuation, in English or French, so most quiz tool templates come in as they are.
The separator is `;`, `,` or a tab, whichever the first row uses most.

| Column | Recognised names |
|---|---|
| statement (required) | question, question text, statement, énoncé, intitulé, text |
| options (two or more) | option 1, answer 1, réponse 1, choice 1, proposition 1..., or A, B, C... |
| correct answers (required) | correct, correct answer(s), answer, bonne réponse, solution, key, corrigé |
| theme | theme, thème, category, catégorie, topic, chapitre |
| id | id |
| rule | explanation, explication, feedback, rule, règle, answer explanation |
| review pointer | review, revoir, course |
| level | level, niveau (1 to 5) |
| time | time, time limit, time in seconds, temps, durée (seconds) |
| type | question type, type: rows whose type is not a choice question are left out |

The correct answers are letters (`B`, `A,C`), numbers from 1 (`2`, `1;3`) or the text of an
option.

## Handout

`mcq handout` writes the paper for students in Markdown: title, rules (number of questions,
duration, penalties), themes, then each question with its options lettered A, B, C... in the
order of the source, and the number of boxes to tick on multiple-answer questions. No correct
mark, explanation, rule or author note. The text follows the paper language (`lang: fr` or
English).

The handout is generated, never edited: `mcq handout --check` exits with 1 when it no longer
matches its paper, so a script can catch a forgotten update. An existing file is replaced only if
`mcq handout` wrote it (or with `--force`), and keeps its line ends.
