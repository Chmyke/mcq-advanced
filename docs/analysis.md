# Class analysis

Once the copies are in, the Studio tells the teacher what the class did not get and which
questions to rewrite, in the order a teacher acts on it, the marks last. Everything is computed on
the teacher's device from the published package (its answer key) and the sealed copies, which
open with the teacher's identity only.

## Grading again

Every copy is graded again from the answer key, with the paper's scoring: nothing in a copy is
trusted but the ticked boxes. A copy of another package is left out and reported.

## 1. Themes to teach again

A theme needs at least 3 questions for its rate to mean something. It is to be taught again when
the class's mean rate on it is under 50 %, or when at least 40 % of the class has to relearn it
(a student's rate under 50 %). Students in between ("fragile", 50 to 75 %) do not count: with
them, every theme of a class around 12/20 would come up.

When a theme to reteach holds a question whose key is in doubt (see below), the sentence says so:
the notion may be fine, the question may not.

## 2. Shared confusions

A distractor is a shared confusion when it holds at least 25 % of the answers given **and** gathers
the wrong answers beyond chance: were the wrong answers spread evenly over the distractors, a
count as high would happen less than once in a hundred (binomial test). A hard question spreads
its mistakes; one misleading idea gathers them. With a single distractor (true or false), the two
cannot be told apart and no confusion is claimed.

## 3. Questions to review

From 8 copies on, students are ranked by total credit and the upper and lower 27 % (rounded
down, at least one) are compared.

| Finding | Rule |
|---|---|
| Key to check | Discrimination (upper minus lower group success) is negative, or a distractor separates strong and weak students by at least 0.2 and better than the key: it behaves like the right answer. Nothing else is said about the question. |
| Too hard | 15 % of the class or fewer got it. |
| Dead distractors | From 8 copies, distractors nobody chose. |
| Too easy | 95 % or more got it. |
| Weak discrimination | Discrimination between 0 and 0.1. |

The point-biserial (correlation between the question and the rest of the paper) is exported with
each question. The paper's reliability is KR-20 (coefficient alpha with partial credit), shown from
8 copies: under 0.5 very low, under 0.7 low, under 0.8 fair, good above.

## 4. Copies to look at closely

An anomaly is a reason to ask a question, never a verdict, and never changes a mark.

| Anomaly | Rule |
|---|---|
| Another paper | The copy answers another package: left out. |
| Same name | Several copies under one name (case and spaces aside): all kept. |
| Unknown questions | Answers to questions missing from the key. |
| Identical copies | The same answers on every question, at least 4 identical wrong ones. |
| Left the window | 5 times or more. |
| Overtime | Past the time limit. |
| Unusually fast | Under 10 % of the estimated time with at least 70 % success. |

## Small classes

Under 30 copies, every figure is shown as a trend to confirm. Under 8 copies, discrimination,
reliability and dead distractors are not computed.

## Deciding about a doubtful key

Next to a key to check, the teacher decides, and every copy is marked again at once:

- **Neutralize**: the question leaves the marks. A paper of N questions is marked on N-1; the
  question keeps its statistics but no finding.
- **Accept an option**: on a single-answer question it becomes the right answer in place of the
  key; on a multiple-answer one it joins the right answers.

A later decision on the same question replaces the earlier one, and undoing one grades the
question with its key again. Decisions are shown above the questions to review, in
`questions.csv` (the alert column says `NEUTRALIZED` or `KEY CORRECTED`, the right rows show the
key used) and on the revision sheets, which show the key the copies were marked with.

## Revision sheets

Each student's sheet lists where they stand per theme, each mistake with the right answer, its
explanation and the rule to remember, and the (at most 3) weakest themes to start with. A question
whose key is in doubt is not corrected on the sheet: the student is told to ask in class.

## Spreadsheets

`notes.csv` (one row per student, one column per theme), `themes.csv` and `questions.csv` (one row
per question, one sub-row per option): UTF-8 with a byte order mark, `;` between fields, numbers
with one decimal (a comma in French), as the earlier tool wrote them.
