---
title: Demo: a short tour of MCQ Advanced
description: A fictional six-question paper to try the exam, then its correction.
lang: en
duration: 10
penalty: 0
unansweredPenalty: 0
scoring: partial
shuffleQuestions: false
shuffleOptions: true
showAnswerCount: true
---
Author notes: shown in the editor only, never to students. This fictional paper is the website
demo: a formula, a picture, coloured code, a multiple-answer question and a true or false.

# THEME science | Science
COURSE: Science, chapter 1

## Q linear-equation | level 3 | 30s
What is $x$ if $2x + 3 = 11$?

- [x] $x = 4$
  > Take 3 from both sides, $2x = 8$, then divide by 2.
- [ ] $x = 8$
  > That is $2x$: it still has to be divided by 2.
- [ ] $x = 7$
  > The 3 was added instead of taken away: $2x = 14$.

RULE: Whatever you do to one side of the equation, do it to the other.
REVIEW: Science, chapter 1, equations

## Q triangle-hypotenuse | level 3 | 40s
![Right triangle whose sides around the right angle measure 3 cm and 4 cm](media/triangle.svg)

The sides around the right angle measure 3 cm and 4 cm. How long is the hypotenuse?

- [x] 5 cm
  > $$\sqrt{3^2 + 4^2} = \sqrt{25} = 5$$
- [ ] 7 cm
  > That is the sum of the two sides, always longer than the hypotenuse.
- [ ] 25 cm
  > 25 is the square of the hypotenuse: the square root is missing.

RULE: In a right triangle, $a^2 + b^2 = c^2$.
REVIEW: Science, chapter 1, Pythagoras' theorem

## Q summer-seasons | level 2 | 30s
Why is it warmer in summer than in winter in Europe?

- [x] The Sun's rays arrive less slanted
  > Steeper rays spread their energy over a smaller area, and the days are longer.
- [ ] The Earth is closer to the Sun then
  > It is the other way round: the Earth is closest in early January.
- [ ] The Sun shines harder in summer
  > Its power barely changes over the year.

RULE: Seasons come from the tilt of the Earth's axis, not from its distance to the Sun.
REVIEW: Science, chapter 1, seasons

# THEME code | Programming
COURSE: Python, chapter 2

## Q loop-sum | level 3 | 40s
Read this program.

```python {3}
total = 0
for i in range(1, 4):
    total += i
print(total)
```

What does it print?

- [x] `6`
  > `i` takes the values 1, 2 and 3: 1 + 2 + 3 = 6.
- [ ] `10`
  > `range(1, 4)` stops before 4.
- [ ] `3`
  > 3 is the last value of `i`, not the sum.

RULE: Trace a loop one iteration at a time.
REVIEW: Python, chapter 2, loops

## Q python-lists | level 2 | 30s
Which statements about Python lists are true?

- [x] The first item has index `0`
  > Indexes start at 0.
- [x] A list can mix types
  > `[1, "two", 3.0]` is a valid list.
- [ ] Its size is fixed when created
  > `append` makes it longer at any time.
- [ ] `len(items)` gives the last item
  > `len` gives the number of items.

RULE: A list is ordered, changeable and indexed from 0.
REVIEW: Python, chapter 2, lists

## Q python-indentation | level 2 | 20s
True or false? In Python, indentation is part of the syntax.

- [x] True
  > It marks the blocks, where other languages use braces.
- [ ] False
  > Without it, the body of a loop is not recognised.

RULE: In Python, what is indented one step belongs to the block above.
REVIEW: Python, chapter 2, blocks
