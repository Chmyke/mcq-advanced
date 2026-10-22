---
title: Sample paper: fruit and code
description: A fictional paper used by the tests.
lang: en
duration: 10
penalty: 0.25
unansweredPenalty: 0
scoring: partial
shuffleQuestions: false
shuffleOptions: true
---
Author notes before the first theme are kept for the author only.

# THEME colours | Fruit colours
COURSE: Fruit, chapter 1

## Q fruit-banana-colour | level 2 | 15s
A ripe banana is usually which colour?

- [x] Yellow
  > Ripening reveals the yellow pigments.
  >
  > New paragraph.
- [ ] Blue
  > No banana pigment is blue.
- [ ] Purple
  > You may be thinking of plums.

RULE: Colour follows ripeness.
REVIEW: Fruit, chapter 1, ripening

# THEME loops | Counting loops
COURSE: Code, chapter 2

## Q code-sum-of-squares | level 3
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
  > i takes the values 1, 2 and 3.
- [ ] It prints `6`
  > That would be the sum without squaring.
- [ ] `i` still exists after the loop
  > It is scoped to the `for` statement.

RULE: Trace a loop one iteration at a time.

## Q fruit-apple-picture | level 1
![A red apple on a table](media/apple.png)
![](media/crunch.ogg)
Which fruit is shown?

- [x] An apple
  > Round, red, with a short stalk.
- [ ] A pear
  > A pear narrows towards the stalk.

RULE: Look at the shape before the colour.
