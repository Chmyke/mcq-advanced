---
title: Model paper: Python and algorithms
description: A fictional eight-question computing paper, upper secondary level, to duplicate and make your own.
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

# THEME python-basics | Python basics
COURSE: Computing, chapter 1, Python basics

## Q integer-division | level 2 | 30s
Read this program.

```python
x = 7
y = 2
print(x // y, x % y)
```

What does it print?

- [ ] `3.5 1`
  > `//` is integer division: it gives a whole number, here 3.
- [x] `3 1`
  > 7 = 2 × 3 + 1: the integer quotient is 3 and the remainder 1.
- [ ] `3 0.5`
  > `%` gives the remainder of the integer division, here 1.

RULE: `a // b` gives the integer quotient, `a % b` the remainder.
REVIEW: Computing, chapter 1, operators

## Q list-index | level 3 | 40s
Read this program.

```python
fruits = ["apple", "kiwi", "banana"]
print(len(fruits[1]))
```

Which value does it print?

- [x] `4`
  > `fruits[1]` is the second item, `"kiwi"`, which has 4 letters.
- [ ] `5`
  > That is the length of `"apple"`, the item at index 0.
- [ ] `3`
  > That is the number of items in the list, `len(fruits)`.

RULE: Indexes start at 0: `items[1]` is the second item.
REVIEW: Computing, chapter 1, lists and strings

## Q list-changed | level 4 | 40s
Read this program.

```python {3}
def double(items):
    for i in range(len(items)):
        items[i] = items[i] * 2

values = [1, 2, 3]
double(values)
print(values)
```

What does it print?

- [ ] `[1, 2, 3]`
  > The function gets the list itself, not a copy: its changes remain.
- [ ] `None`
  > `double` returns `None`, but it is `values` that is printed.
- [x] `[2, 4, 6]`
  > The function changes the list it gets, which is the same list as `values`.

RULE: A function that changes a list passed to it changes the caller's list.
REVIEW: Computing, chapter 1, functions and lists

## Q boolean-expressions | level 3 | 30s
Which Python expressions evaluate to `True`?

- [x] `3 < 5 and 5 < 8`
  > Both comparisons are true, so their `and` is too.
- [ ] `"10" == 10`
  > A string and an integer are never equal in Python.
- [x] `not (2 > 3)`
  > `2 > 3` is `False`, and `not False` is `True`.
- [ ] `len("abc") == 4`
  > `"abc"` has 3 characters.

RULE: Evaluate a Boolean expression from the inside out.
REVIEW: Computing, chapter 1, Booleans

# THEME algorithms | Algorithms
COURSE: Computing, chapter 3, algorithms

## Q function-keyword | level 1 | 20s
Which Python keyword defines a function?

- [ ] `function`
  > That is the JavaScript keyword.
- [x] `def`
  > `def name(parameters):` opens the definition of a function.
- [ ] `func`
  > That is the keyword of Go and Swift.

RULE: In Python, a function is defined with `def`, then its name and its parameters.
REVIEW: Computing, chapter 3, functions

## Q while-loop | level 3 | 40s
Read this program.

```python
n = 10
count = 0
while n > 1:
    n = n // 2
    count = count + 1
print(count)
```

How many times does the loop run, so which value is printed?

- [x] `3`
  > `n` goes from 10 to 5, then 2, then 1: three rounds, and the loop stops.
- [ ] `5`
  > 5 is the value of `n` after the first round.
- [ ] `4`
  > 10, 5, 2 and 1 are four values, but only three divisions.

RULE: Trace a `while` loop round by round and test the condition before each round.
REVIEW: Computing, chapter 3, loops

## Q binary-search | level 4 | 40s
In the worst case, about how many items does a binary search look at in a sorted list of 1000 items?

- [ ] 1000
  > That is the worst case of a linear search, which looks at everything.
- [ ] 500
  > That is the average case of a linear search.
- [x] 10
  > Each step halves the list, and $2^{10} = 1024$ is more than 1000.

RULE: A binary search takes about $\log_2 n$ steps.
REVIEW: Computing, chapter 3, complexity

## Q nested-list | level 2 | 20s
True or false? A Python list can hold other lists.

- [x] True
  > `[[1, 2], [3, 4]]` is a list of two lists, handy for a two-dimensional table.
- [ ] False
  > A list accepts items of any type, lists included.

RULE: A list of lists stands for a table: `t[i][j]` is the cell in row `i`, column `j`.
REVIEW: Computing, chapter 3, tables
