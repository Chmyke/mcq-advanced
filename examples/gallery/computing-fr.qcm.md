---
title: Modèle : Python et algorithmique
description: Un sujet fictif de huit questions d'informatique, niveau lycée, à dupliquer pour écrire le vôtre.
lang: fr
duration: 6
penalty: 0
unansweredPenalty: 0
scoring: partial
shuffleQuestions: false
shuffleOptions: true
showAnswerCount: true
---
Notes d'auteur : visibles seulement dans l'éditeur, jamais par les élèves. Ce sujet fictif est un
modèle à dupliquer : gardez sa structure, deux thèmes de quatre questions, un niveau et un temps par
question, une explication sous chaque réponse, et remplacez le contenu par le vôtre.

# THEME bases-python | Bases de Python
COURSE: Informatique, chapitre 1, bases de Python

## Q division-entiere | level 2 | 30s
Lisez ce programme.

```python
x = 7
y = 2
print(x // y, x % y)
```

Qu'affiche-t-il ?

- [ ] `3.5 1`
  > `//` est la division entière : elle donne un entier, ici 3.
- [x] `3 1`
  > 7 = 2 × 3 + 1 : le quotient entier vaut 3 et le reste 1.
- [ ] `3 0.5`
  > `%` donne le reste de la division entière, ici 1.

RULE: `a // b` donne le quotient entier, `a % b` le reste.
REVIEW: Informatique, chapitre 1, opérateurs

## Q indice-liste | level 3 | 40s
Lisez ce programme.

```python
fruits = ["pomme", "kiwi", "banane"]
print(len(fruits[1]))
```

Quelle valeur affiche-t-il ?

- [x] `4`
  > `fruits[1]` est le deuxième élément, `"kiwi"`, qui compte 4 lettres.
- [ ] `5`
  > C'est la longueur de `"pomme"`, l'élément d'indice 0.
- [ ] `3`
  > C'est le nombre d'éléments de la liste, `len(fruits)`.

RULE: Les indices commencent à 0 : `liste[1]` est le deuxième élément.
REVIEW: Informatique, chapitre 1, listes et chaînes

## Q liste-modifiee | level 4 | 40s
Lisez ce programme.

```python {3}
def doubler(liste):
    for i in range(len(liste)):
        liste[i] = liste[i] * 2

valeurs = [1, 2, 3]
doubler(valeurs)
print(valeurs)
```

Qu'affiche-t-il ?

- [ ] `[1, 2, 3]`
  > La fonction reçoit la liste elle-même, pas une copie : ses modifications restent.
- [ ] `None`
  > `doubler` renvoie `None`, mais c'est `valeurs` qui est affichée.
- [x] `[2, 4, 6]`
  > La fonction modifie la liste reçue, qui est la même que `valeurs`.

RULE: Une fonction qui modifie une liste reçue en paramètre modifie la liste de l'appelant.
REVIEW: Informatique, chapitre 1, fonctions et listes

## Q expressions-booleennes | level 3 | 30s
Quelles expressions Python valent `True` ?

- [x] `3 < 5 and 5 < 8`
  > Les deux comparaisons sont vraies, donc leur `and` aussi.
- [ ] `"10" == 10`
  > Une chaîne et un entier ne sont jamais égaux en Python.
- [x] `not (2 > 3)`
  > `2 > 3` vaut `False`, et `not False` vaut `True`.
- [ ] `len("abc") == 4`
  > `"abc"` compte 3 caractères.

RULE: Évaluez une expression booléenne de l'intérieur vers l'extérieur.
REVIEW: Informatique, chapitre 1, booléens

# THEME algorithmique | Algorithmique
COURSE: Informatique, chapitre 3, algorithmique

## Q mot-cle-fonction | level 1 | 20s
Quel mot-clé Python sert à définir une fonction ?

- [ ] `function`
  > C'est le mot-clé de JavaScript.
- [x] `def`
  > `def nom(paramètres):` ouvre la définition d'une fonction.
- [ ] `func`
  > C'est le mot-clé de Go et de Swift.

RULE: En Python, une fonction se définit avec `def`, suivi de son nom et de ses paramètres.
REVIEW: Informatique, chapitre 3, fonctions

## Q boucle-while | level 3 | 40s
Lisez ce programme.

```python
n = 10
compte = 0
while n > 1:
    n = n // 2
    compte = compte + 1
print(compte)
```

Combien de tours la boucle fait-elle, donc quelle valeur s'affiche ?

- [x] `3`
  > `n` passe de 10 à 5, puis 2, puis 1 : trois tours, et la boucle s'arrête.
- [ ] `5`
  > 5 est la valeur de `n` après le premier tour.
- [ ] `4`
  > 10, 5, 2 et 1 font quatre valeurs, mais seulement trois divisions.

RULE: Suivez une boucle `while` tour par tour et testez la condition avant chaque tour.
REVIEW: Informatique, chapitre 3, boucles

## Q recherche-dichotomique | level 4 | 40s
Au pire, environ combien d'éléments une recherche dichotomique examine-t-elle dans une liste triée de 1000 éléments ?

- [ ] 1000
  > C'est le pire cas d'une recherche séquentielle, qui examine tout.
- [ ] 500
  > C'est le cas moyen d'une recherche séquentielle.
- [x] 10
  > Chaque étape coupe la liste en deux, et $2^{10} = 1024$ dépasse 1000.

RULE: Une recherche dichotomique fait environ $\log_2 n$ étapes.
REVIEW: Informatique, chapitre 3, complexité

## Q liste-imbriquee | level 2 | 20s
Vrai ou faux ? Une liste Python peut contenir d'autres listes.

- [x] Vrai
  > `[[1, 2], [3, 4]]` est une liste de deux listes, utile pour un tableau à deux dimensions.
- [ ] Faux
  > Une liste accepte des éléments de tout type, listes comprises.

RULE: Une liste de listes représente un tableau : `t[i][j]` est la case de la ligne `i`, colonne `j`.
REVIEW: Informatique, chapitre 3, tableaux
