---
title: Démo : un petit tour de MCQ Advanced
description: Un QCM fictif de six questions pour découvrir l'épreuve, puis sa correction.
lang: fr
duration: 10
penalty: 0
unansweredPenalty: 0
scoring: partial
shuffleQuestions: false
shuffleOptions: true
showAnswerCount: true
---
Notes d'auteur : visibles seulement dans l'éditeur, jamais par les élèves. Ce QCM fictif sert de
démo sur le site : une formule, une image, du code coloré, une question à plusieurs réponses et un
vrai ou faux.

# THEME sciences | Sciences
COURSE: Sciences, chapitre 1

## Q equation-premier-degre | level 3 | 30s
Que vaut $x$ si $2x + 3 = 11$ ?

- [x] $x = 4$
  > On retire 3 des deux côtés, $2x = 8$, puis on divise par 2.
- [ ] $x = 8$
  > C'est $2x$ qui vaut 8 : il reste à diviser par 2.
- [ ] $x = 7$
  > Le 3 a été ajouté au lieu d'être retiré : $2x = 14$.

RULE: Ce qu'on fait d'un côté de l'égalité, on le fait de l'autre.
REVIEW: Sciences, chapitre 1, équations

## Q hypotenuse-triangle | level 3 | 40s
![Triangle rectangle dont les côtés de l'angle droit mesurent 3 cm et 4 cm](media/triangle.svg)

Les côtés de l'angle droit mesurent 3 cm et 4 cm. Combien mesure l'hypoténuse ?

- [x] 5 cm
  > $$\sqrt{3^2 + 4^2} = \sqrt{25} = 5$$
- [ ] 7 cm
  > C'est la somme des deux côtés, toujours plus longue que l'hypoténuse.
- [ ] 25 cm
  > 25 est le carré de l'hypoténuse : il manque la racine carrée.

RULE: Dans un triangle rectangle, $a^2 + b^2 = c^2$.
REVIEW: Sciences, chapitre 1, théorème de Pythagore

## Q saisons-ete | level 2 | 30s
Pourquoi fait-il plus chaud en été qu'en hiver en France ?

- [x] Les rayons du Soleil arrivent moins inclinés
  > Plus droits, ils concentrent leur énergie sur une plus petite surface, et les jours sont plus longs.
- [ ] La Terre est alors plus proche du Soleil
  > C'est l'inverse : la Terre en est au plus près début janvier.
- [ ] Le Soleil brille plus fort en été
  > Sa puissance varie à peine au fil de l'année.

RULE: Les saisons viennent de l'inclinaison de l'axe de la Terre, pas de sa distance au Soleil.
REVIEW: Sciences, chapitre 1, saisons

# THEME code | Programmation
COURSE: Python, chapitre 2

## Q boucle-somme | level 3 | 40s
Lisez ce programme.

```python {3}
total = 0
for i in range(1, 4):
    total += i
print(total)
```

Qu'affiche-t-il ?

- [x] `6`
  > `i` prend les valeurs 1, 2 et 3 : 1 + 2 + 3 = 6.
- [ ] `10`
  > `range(1, 4)` s'arrête avant 4.
- [ ] `3`
  > 3 est la dernière valeur de `i`, pas la somme.

RULE: Suivez la boucle un tour après l'autre.
REVIEW: Python, chapitre 2, boucles

## Q listes-python | level 2 | 30s
Quelles affirmations sur les listes Python sont vraies ?

- [x] Le premier élément a l'indice `0`
  > Les indices commencent à 0.
- [x] Une liste peut mêler des types
  > `[1, "deux", 3.0]` est une liste valide.
- [ ] Sa taille est fixée à la création
  > `append` l'allonge à tout moment.
- [ ] `len(liste)` donne le dernier élément
  > `len` donne le nombre d'éléments.

RULE: Une liste est ordonnée, modifiable et indexée à partir de 0.
REVIEW: Python, chapitre 2, listes

## Q indentation-python | level 2 | 20s
Vrai ou faux ? En Python, l'indentation fait partie de la syntaxe.

- [x] Vrai
  > Elle délimite les blocs, là où d'autres langages utilisent des accolades.
- [ ] Faux
  > Sans elle, le corps d'une boucle n'est pas reconnu.

RULE: En Python, ce qui est décalé d'un cran appartient au bloc au-dessus.
REVIEW: Python, chapitre 2, blocs
