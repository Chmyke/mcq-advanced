---
title: Modèle : fonctions et probabilités
description: Un sujet fictif de huit questions, niveau lycée, à dupliquer pour écrire le vôtre.
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

# THEME fonctions | Fonctions
COURSE: Mathématiques, chapitre 3, fonctions

## Q derivee-cube | level 2 | 30s
Quelle est la dérivée de la fonction $f(x) = x^3$ ?

- [ ] $\frac{x^4}{4}$
  > C'est une primitive de $x^3$ : on a intégré au lieu de dériver.
- [x] $3x^2$
  > L'exposant passe devant et diminue de 1 : $(x^n)' = n x^{n-1}$.
- [ ] $3x$
  > L'exposant a perdu 2 au lieu de 1.

RULE: La dérivée de $x^n$ est $n x^{n-1}$.
REVIEW: Mathématiques, chapitre 3, dérivées usuelles

## Q second-degre-racines | level 3 | 40s
Quelles sont les solutions de l'équation $x^2 - 5x + 6 = 0$ ?

- [x] $x = 2$ et $x = 3$
  > $x^2 - 5x + 6 = (x - 2)(x - 3)$, qui s'annule en 2 et en 3.
- [ ] $x = -2$ et $x = -3$
  > Leur somme vaut $-5$ : le signe du coefficient de $x$ a été mal lu.
- [ ] $x = 1$ et $x = 6$
  > Leur produit vaut bien 6, mais leur somme vaut 7 et non 5.

RULE: Si $x_1$ et $x_2$ sont les racines de $x^2 + bx + c$, alors $x_1 + x_2 = -b$ et $x_1 x_2 = c$.
REVIEW: Mathématiques, chapitre 3, second degré

## Q composition-fonctions | level 4 | 40s
On pose $f(x) = 2x + 1$ et $g(x) = x^2$. Que vaut $f(g(3))$ ?

- [ ] 16
  > C'est $f(3) + g(3) = 7 + 9$ : on a ajouté au lieu de composer.
- [ ] 49
  > C'est $g(f(3)) = 7^2$ : l'ordre de la composition est inversé.
- [x] 19
  > On calcule d'abord $g(3) = 9$, puis $f(9) = 2 \times 9 + 1 = 19$.

RULE: Dans $f(g(x))$, on applique d'abord $g$, puis $f$ au résultat.
REVIEW: Mathématiques, chapitre 3, composition

## Q fonctions-croissantes | level 3 | 30s
Lesquelles de ces fonctions sont croissantes sur l'ensemble des réels ?

- [x] $x \mapsto e^x$
  > Sa dérivée, $e^x$, est strictement positive partout.
- [ ] $x \mapsto x^2$
  > Elle décroît sur les réels négatifs, puis croît.
- [x] $x \mapsto 3x - 2$
  > Une fonction affine de coefficient positif est croissante.
- [ ] $x \mapsto -x$
  > Son coefficient vaut $-1$ : elle est décroissante.

RULE: Une fonction dont la dérivée est positive sur un intervalle y est croissante.
REVIEW: Mathématiques, chapitre 3, sens de variation

# THEME probabilites | Probabilités
COURSE: Mathématiques, chapitre 5, probabilités

## Q de-equilibre | level 1 | 20s
Avec un dé équilibré à six faces, quelle est la probabilité d'obtenir 6 ?

- [ ] $\frac{1}{5}$
  > Les six faces comptent, le 6 compris.
- [x] $\frac{1}{6}$
  > Une issue favorable sur six issues équiprobables.
- [ ] $\frac{5}{6}$
  > C'est la probabilité de l'événement contraire : ne pas obtenir 6.

RULE: Pour des issues équiprobables, $P = \frac{\text{issues favorables}}{\text{issues possibles}}$.
REVIEW: Mathématiques, chapitre 5, équiprobabilité

## Q deux-pieces | level 3 | 40s
On lance deux pièces équilibrées. Quelle est la probabilité d'obtenir au moins un pile ?

- [x] $\frac{3}{4}$
  > Sur les quatre issues PP, PF, FP et FF, seule FF ne contient pas de pile.
- [ ] $\frac{1}{2}$
  > On a compté trois issues (deux piles, un pile, zéro pile) comme si elles étaient équiprobables.
- [ ] $\frac{1}{4}$
  > C'est la probabilité de n'obtenir aucun pile.

RULE: « Au moins un » se calcule souvent plus vite par l'événement contraire : $1 - P(\text{aucun})$.
REVIEW: Mathématiques, chapitre 5, événement contraire

## Q urne-sans-remise | level 4 | 40s
Une urne contient 3 boules rouges et 2 bleues. On en tire deux, l'une après l'autre, sans remise.
Quelle est la probabilité que les deux soient rouges ?

- [ ] $\frac{9}{25}$
  > C'est le résultat avec remise : $\frac{3}{5} \times \frac{3}{5}$.
- [ ] $\frac{3}{5}$
  > C'est la probabilité pour le premier tirage seulement.
- [x] $\frac{3}{10}$
  > $\frac{3}{5} \times \frac{2}{4} = \frac{6}{20}$ : après une rouge, il en reste 2 sur 4.

RULE: Sans remise, chaque tirage change la composition de l'urne pour le suivant.
REVIEW: Mathématiques, chapitre 5, arbres pondérés

## Q evenement-contraire | level 2 | 20s
Vrai ou faux ? Si un événement a une probabilité de 0,3, son contraire a une probabilité de 0,7.

- [x] Vrai
  > Un événement et son contraire ont des probabilités dont la somme vaut 1.
- [ ] Faux
  > $1 - 0{,}3 = 0{,}7$ : c'est exactement la probabilité du contraire.

RULE: $P(\overline{A}) = 1 - P(A)$.
REVIEW: Mathématiques, chapitre 5, événement contraire
