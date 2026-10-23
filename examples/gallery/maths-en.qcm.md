---
title: Model paper: functions and probability
description: A fictional eight-question paper, upper secondary level, to duplicate and make your own.
lang: en
duration: 6
penalty: 0
unansweredPenalty: 0
scoring: partial
shuffleQuestions: false
shuffleOptions: true
showAnswerCount: true
---
Author notes: shown in the editor only, never to students. This fictional paper is a model to
duplicate: keep its structure, two themes of four questions, a level and a time per question, an
explanation under every answer, and replace the content with your own.

# THEME functions | Functions
COURSE: Mathematics, chapter 3, functions

## Q derivative-cube | level 2 | 30s
What is the derivative of the function $f(x) = x^3$?

- [ ] $\frac{x^4}{4}$
  > That is an antiderivative of $x^3$: it was integrated instead of differentiated.
- [x] $3x^2$
  > The exponent comes down in front and drops by 1: $(x^n)' = n x^{n-1}$.
- [ ] $3x$
  > The exponent lost 2 instead of 1.

RULE: The derivative of $x^n$ is $n x^{n-1}$.
REVIEW: Mathematics, chapter 3, standard derivatives

## Q quadratic-roots | level 3 | 40s
What are the solutions of the equation $x^2 - 5x + 6 = 0$?

- [x] $x = 2$ and $x = 3$
  > $x^2 - 5x + 6 = (x - 2)(x - 3)$, which is zero at 2 and at 3.
- [ ] $x = -2$ and $x = -3$
  > Their sum is $-5$: the sign of the coefficient of $x$ was misread.
- [ ] $x = 1$ and $x = 6$
  > Their product is 6 indeed, but their sum is 7, not 5.

RULE: If $x_1$ and $x_2$ are the roots of $x^2 + bx + c$, then $x_1 + x_2 = -b$ and $x_1 x_2 = c$.
REVIEW: Mathematics, chapter 3, quadratics

## Q function-composition | level 4 | 40s
Let $f(x) = 2x + 1$ and $g(x) = x^2$. What is $f(g(3))$?

- [ ] 16
  > That is $f(3) + g(3) = 7 + 9$: the functions were added, not composed.
- [ ] 49
  > That is $g(f(3)) = 7^2$: the order of the composition is reversed.
- [x] 19
  > First $g(3) = 9$, then $f(9) = 2 \times 9 + 1 = 19$.

RULE: In $f(g(x))$, apply $g$ first, then $f$ to the result.
REVIEW: Mathematics, chapter 3, composition

## Q increasing-functions | level 3 | 30s
Which of these functions are increasing over all real numbers?

- [x] $x \mapsto e^x$
  > Its derivative, $e^x$, is strictly positive everywhere.
- [ ] $x \mapsto x^2$
  > It decreases over the negative numbers, then increases.
- [x] $x \mapsto 3x - 2$
  > A linear function with a positive coefficient is increasing.
- [ ] $x \mapsto -x$
  > Its coefficient is $-1$: it is decreasing.

RULE: A function whose derivative is positive over an interval is increasing there.
REVIEW: Mathematics, chapter 3, monotonicity

# THEME probability | Probability
COURSE: Mathematics, chapter 5, probability

## Q fair-die | level 1 | 20s
With a fair six-sided die, what is the probability of rolling a 6?

- [ ] $\frac{1}{5}$
  > All six faces count, the 6 included.
- [x] $\frac{1}{6}$
  > One favourable outcome out of six equally likely ones.
- [ ] $\frac{5}{6}$
  > That is the probability of the complement: rolling anything but a 6.

RULE: For equally likely outcomes, $P = \frac{\text{favourable outcomes}}{\text{possible outcomes}}$.
REVIEW: Mathematics, chapter 5, equally likely outcomes

## Q two-coins | level 3 | 40s
Two fair coins are tossed. What is the probability of getting at least one head?

- [x] $\frac{3}{4}$
  > Of the four outcomes HH, HT, TH and TT, only TT has no head.
- [ ] $\frac{1}{2}$
  > Three outcomes (two heads, one head, no head) were counted as if equally likely.
- [ ] $\frac{1}{4}$
  > That is the probability of getting no head at all.

RULE: "At least one" is often quicker through the complement: $1 - P(\text{none})$.
REVIEW: Mathematics, chapter 5, complementary events

## Q urn-without-replacement | level 4 | 40s
An urn holds 3 red balls and 2 blue ones. Two are drawn, one after the other, without replacement.
What is the probability that both are red?

- [ ] $\frac{9}{25}$
  > That is the result with replacement: $\frac{3}{5} \times \frac{3}{5}$.
- [ ] $\frac{3}{5}$
  > That is the probability for the first draw only.
- [x] $\frac{3}{10}$
  > $\frac{3}{5} \times \frac{2}{4} = \frac{6}{20}$: after a red one, 2 of the 4 left are red.

RULE: Without replacement, each draw changes what the urn holds for the next one.
REVIEW: Mathematics, chapter 5, tree diagrams

## Q complementary-event | level 2 | 20s
True or false? If an event has a probability of 0.3, its complement has a probability of 0.7.

- [x] True
  > The probabilities of an event and of its complement add up to 1.
- [ ] False
  > $1 - 0.3 = 0.7$: that is exactly the probability of the complement.

RULE: $P(\overline{A}) = 1 - P(A)$.
REVIEW: Mathematics, chapter 5, complementary events
