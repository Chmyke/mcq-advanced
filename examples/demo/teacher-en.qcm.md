---
title: Teacher demo: end-of-term test
description: A fictional twelve-question paper, taken by a fictional class of thirty-two students.
lang: en
duration: 15
penalty: 0
unansweredPenalty: 0
scoring: partial
shuffleQuestions: true
shuffleOptions: true
showAnswerCount: true
---
Author notes: seen in the editor only, never by students. This fictional paper is the teacher
demo. It holds a deliberate mistake: on the question about adding fractions, the wrong box is
ticked. The Correct tab finds it from the copies, and one click fixes every mark.

# THEME maths | Mathematics
COURSE: Mathematics, chapter 3

## Q linear-equation | level 3 | 30s
What is $x$ if $2x + 3 = 11$?

- [ ] $x = 8$
  > It is $2x$ that equals 8: divide by 2 as well.
- [x] $x = 4$
  > Take 3 from both sides, $2x = 8$, then divide by 2.
- [ ] $x = 7$
  > The 3 was added instead of taken away: $2x = 14$.

RULE: Whatever you do to one side of the equation, do to the other.
REVIEW: Mathematics, chapter 3, equations

## Q adding-fractions | level 3 | 40s
What is $\frac{1}{2} + \frac{1}{3}$?

- [ ] 5/6
  > Over the same denominator: $\frac{3}{6} + \frac{2}{6} = \frac{5}{6}$.
- [x] 1/6
  > That is the difference of the two fractions, not their sum.
- [ ] 2/5
  > Denominators are never added.

RULE: To add two fractions, first put them over the same denominator.
REVIEW: Mathematics, chapter 3, fractions

## Q triangle-hypotenuse | level 3 | 40s
![Right triangle whose sides around the right angle are 3 cm and 4 cm](media/triangle.svg)

The sides around the right angle are 3 cm and 4 cm. How long is the hypotenuse?

- [ ] 7 cm
  > That is the sum of the two sides, always longer than the hypotenuse.
- [ ] 25 cm
  > 25 is the square of the hypotenuse: the square root is missing.
- [x] 5 cm
  > $$\sqrt{3^2 + 4^2} = \sqrt{25} = 5$$

RULE: In a right triangle, $a^2 + b^2 = c^2$.
REVIEW: Mathematics, chapter 3, Pythagorean theorem

## Q down-then-up | level 4 | 50s
A price falls by 20 %, then the new price rises by 20 %. Compared with the starting price?

- [x] It fell by 4 %
  > $100 \times 0.8 \times 1.2 = 96$: 4 % is missing.
- [ ] It is back to the same price
  > The rise applies to a smaller price: it adds less than the fall took away.
- [ ] It rose by 4 %
  > The direction is reversed: $96 < 100$.
- [ ] It fell by 20 %
  > The rise did make up part of the fall.

RULE: Successive percentages multiply, they do not add up.
REVIEW: Mathematics, chapter 3, percentages

# THEME science | Science
COURSE: Science, chapter 1

## Q summer-seasons | level 2 | 30s
Why is it warmer in summer than in winter in Europe?

- [ ] The Earth is then closer to the Sun
  > The other way round: the Earth is closest to it in early January.
- [x] The Sun's rays arrive less slanted
  > Straighter, they spread their energy over a smaller area, and days are longer.
- [ ] The Sun shines harder in summer
  > Its power barely changes over the year.

RULE: Seasons come from the tilt of the Earth's axis, not from its distance to the Sun.
REVIEW: Science, chapter 1, seasons

## Q photosynthesis | level 2 | 30s
What does a plant produce through photosynthesis?

- [ ] Carbon dioxide
  > The plant uses it up, it does not produce it.
- [x] Oxygen
  > It is released into the air.
- [x] Glucose
  > It feeds the plant.
- [ ] Nitrogen
  > Photosynthesis does not involve nitrogen.

RULE: With light, a plant turns water and carbon dioxide into glucose and oxygen.
REVIEW: Science, chapter 1, photosynthesis

## Q average-speed | level 3 | 40s
A car drives 120 km in 1 h 30 min. What is its average speed?

- [ ] 90 km/h
  > 1 h 30 min is 1.5 h, not 1.3 h.
- [ ] 120 km/h
  > That would hold for a one-hour drive.
- [x] 80 km/h
  > $v = \frac{d}{t} = \frac{120}{1.5} = 80$

RULE: Average speed is the distance divided by the time, in hours.
REVIEW: Science, chapter 1, speed

## Q boiling-water | level 1 | 20s
True or false? At sea level, pure water boils at 100 °C.

- [x] True
  > It is the historical definition of the Celsius scale.
- [ ] False
  > It boils lower up a mountain, but at sea level it is indeed 100 °C.

RULE: The boiling point depends on the pressure.
REVIEW: Science, chapter 1, changes of state

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

- [ ] `10`
  > `range(1, 4)` stops before 4.
- [x] `6`
  > `i` takes the values 1, 2 and 3: 1 + 2 + 3 = 6.
- [ ] `3`
  > 3 is the last value of `i`, not the sum.

RULE: Follow the loop one turn at a time.
REVIEW: Python, chapter 2, loops

## Q string-concatenation | level 3 | 30s
What does `print("3" + "4")` print?

- [ ] `7`
  > That would need numbers, without quotes.
- [ ] An error
  > Adding two texts is allowed.
- [x] `34`
  > Between quotes they are texts: `+` puts them end to end.

RULE: What `+` does depends on the type of the values.
REVIEW: Python, chapter 2, types

## Q python-lists | level 3 | 40s
Which statements about Python lists are true?

- [ ] Its size is fixed when it is created
  > `append` makes it longer at any time.
- [x] The first item has index `0`
  > Indices start at 0.
- [ ] `len(items)` gives the last item
  > `len` gives the number of items.
- [x] A list can mix types
  > `[1, "two", 3.0]` is a valid list.

RULE: A list is ordered, changeable and indexed from 0.
REVIEW: Python, chapter 2, lists

## Q floor-division | level 4 | 40s
What is `17 // 5` in Python?

- [ ] `3.4`
  > That is the result of `17 / 5`.
- [x] `3`
  > `//` keeps the whole part of the quotient.
- [ ] `2`
  > 2 is the remainder, given by `17 % 5`.

RULE: `/` divides, `//` keeps the whole part, `%` gives the remainder.
REVIEW: Python, chapter 2, operators
