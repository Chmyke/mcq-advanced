---
title: Démo enseignant : contrôle de fin de trimestre
description: Un sujet fictif de douze questions, passé par une classe fictive de trente-deux élèves.
lang: fr
duration: 15
penalty: 0
unansweredPenalty: 0
scoring: partial
shuffleQuestions: true
shuffleOptions: true
showAnswerCount: true
---
Notes d'auteur : visibles seulement dans l'éditeur, jamais par les élèves. Ce sujet fictif sert de
démo enseignant. Il contient une erreur volontaire : à la question sur la somme de fractions, la
mauvaise case est cochée. L'onglet Corriger la repère grâce aux copies, et un clic corrige toutes
les notes.

# THEME maths | Mathématiques
COURSE: Mathématiques, chapitre 3

## Q equation-premier-degre | level 3 | 30s
Que vaut $x$ si $2x + 3 = 11$ ?

- [ ] $x = 8$
  > C'est $2x$ qui vaut 8 : il reste à diviser par 2.
- [x] $x = 4$
  > On retire 3 des deux côtés, $2x = 8$, puis on divise par 2.
- [ ] $x = 7$
  > Le 3 a été ajouté au lieu d'être retiré : $2x = 14$.

RULE: Ce qu'on fait d'un côté de l'égalité, on le fait de l'autre.
REVIEW: Mathématiques, chapitre 3, équations

## Q somme-fractions | level 3 | 40s
Combien vaut $\frac{1}{2} + \frac{1}{3}$ ?

- [ ] $\frac{5}{6}$
  > Au même dénominateur : $\frac{3}{6} + \frac{2}{6} = \frac{5}{6}$.
- [x] $\frac{1}{6}$
  > C'est la différence des deux fractions, pas leur somme.
- [ ] $\frac{2}{5}$
  > On n'additionne jamais les dénominateurs.

RULE: Pour additionner deux fractions, on les met d'abord au même dénominateur.
REVIEW: Mathématiques, chapitre 3, fractions

## Q hypotenuse-triangle | level 3 | 40s
![Triangle rectangle dont les côtés de l'angle droit mesurent 3 cm et 4 cm](media/triangle.svg)

Les côtés de l'angle droit mesurent 3 cm et 4 cm. Combien mesure l'hypoténuse ?

- [ ] 7 cm
  > C'est la somme des deux côtés, toujours plus longue que l'hypoténuse.
- [ ] 25 cm
  > 25 est le carré de l'hypoténuse : il manque la racine carrée.
- [x] 5 cm
  > $$\sqrt{3^2 + 4^2} = \sqrt{25} = 5$$

RULE: Dans un triangle rectangle, $a^2 + b^2 = c^2$.
REVIEW: Mathématiques, chapitre 3, théorème de Pythagore

## Q baisse-hausse | level 4 | 50s
Un prix baisse de 20 %, puis le nouveau prix augmente de 20 %. Par rapport au prix de départ ?

- [x] Il a baissé de 4 %
  > $100 \times 0{,}8 \times 1{,}2 = 96$ : il manque 4 %.
- [ ] Il est revenu au même prix
  > La hausse s'applique à un prix plus petit : elle rapporte moins que la baisse n'a retiré.
- [ ] Il a augmenté de 4 %
  > Le sens est inversé : $96 < 100$.
- [ ] Il a baissé de 20 %
  > La hausse a bien rattrapé une partie de la baisse.

RULE: Des pourcentages successifs se multiplient, ils ne s'additionnent pas.
REVIEW: Mathématiques, chapitre 3, pourcentages

# THEME sciences | Sciences
COURSE: Sciences, chapitre 1

## Q saisons-ete | level 2 | 30s
Pourquoi fait-il plus chaud en été qu'en hiver en France ?

- [ ] La Terre est alors plus proche du Soleil
  > C'est l'inverse : la Terre en est au plus près début janvier.
- [x] Les rayons du Soleil arrivent moins inclinés
  > Plus droits, ils concentrent leur énergie sur une plus petite surface, et les jours sont plus longs.
- [ ] Le Soleil brille plus fort en été
  > Sa puissance varie à peine au fil de l'année.

RULE: Les saisons viennent de l'inclinaison de l'axe de la Terre, pas de sa distance au Soleil.
REVIEW: Sciences, chapitre 1, saisons

## Q photosynthese | level 2 | 30s
Que produit une plante par la photosynthèse ?

- [ ] Du dioxyde de carbone
  > Elle le consomme, elle ne le produit pas.
- [x] Du dioxygène
  > Il est rejeté dans l'air.
- [x] Du glucose
  > Il nourrit la plante.
- [ ] De l'azote
  > La photosynthèse ne fait pas intervenir l'azote.

RULE: Avec la lumière, la plante transforme eau et dioxyde de carbone en glucose et dioxygène.
REVIEW: Sciences, chapitre 1, photosynthèse

## Q vitesse-moyenne | level 3 | 40s
Une voiture parcourt 120 km en 1 h 30. Quelle est sa vitesse moyenne ?

- [ ] 90 km/h
  > 1 h 30 vaut 1,5 h, pas 1,3 h.
- [ ] 120 km/h
  > Ce serait vrai pour un trajet d'une heure.
- [x] 80 km/h
  > $v = \frac{d}{t} = \frac{120}{1{,}5} = 80$

RULE: La vitesse moyenne est la distance divisée par la durée, en heures.
REVIEW: Sciences, chapitre 1, vitesse

## Q ebullition-eau | level 1 | 20s
Vrai ou faux ? Au niveau de la mer, l'eau pure bout à 100 °C.

- [x] Vrai
  > C'est la définition historique de l'échelle Celsius.
- [ ] Faux
  > En altitude elle bout plus bas, mais au niveau de la mer c'est bien 100 °C.

RULE: La température d'ébullition dépend de la pression.
REVIEW: Sciences, chapitre 1, changements d'état

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

- [ ] `10`
  > `range(1, 4)` s'arrête avant 4.
- [x] `6`
  > `i` prend les valeurs 1, 2 et 3 : 1 + 2 + 3 = 6.
- [ ] `3`
  > 3 est la dernière valeur de `i`, pas la somme.

RULE: Suivez la boucle un tour après l'autre.
REVIEW: Python, chapitre 2, boucles

## Q concatenation | level 3 | 30s
Qu'affiche `print("3" + "4")` ?

- [ ] `7`
  > Il faudrait des nombres, sans guillemets.
- [ ] Une erreur
  > Additionner deux textes est permis.
- [x] `34`
  > Entre guillemets, ce sont des textes : `+` les met bout à bout.

RULE: Le sens de `+` dépend du type des valeurs.
REVIEW: Python, chapitre 2, types

## Q listes-python | level 3 | 40s
Quelles affirmations sur les listes Python sont vraies ?

- [ ] Sa taille est fixée à la création
  > `append` l'allonge à tout moment.
- [x] Le premier élément a l'indice `0`
  > Les indices commencent à 0.
- [ ] `len(liste)` donne le dernier élément
  > `len` donne le nombre d'éléments.
- [x] Une liste peut mêler des types
  > `[1, "deux", 3.0]` est une liste valide.

RULE: Une liste est ordonnée, modifiable et indexée à partir de 0.
REVIEW: Python, chapitre 2, listes

## Q division-entiere | level 4 | 40s
Que vaut `17 // 5` en Python ?

- [ ] `3.4`
  > C'est le résultat de `17 / 5`.
- [x] `3`
  > `//` garde la partie entière du quotient.
- [ ] `2`
  > 2 est le reste, donné par `17 % 5`.

RULE: `/` divise, `//` garde la partie entière, `%` donne le reste.
REVIEW: Python, chapitre 2, opérateurs
