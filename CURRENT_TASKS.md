## 🏗️ Étape 1 : Créer la "Boîte" (La Fenêtre et les Maths)
**Le Contexte :** Un ordinateur ne sait pas ce qu'est une "image" ou un "joueur". Il ne connaît que des nombres. Pour commencer, on doit créer une fenêtre pour afficher des choses et des outils mathématiques pour dire où elles se trouvent.

*   **L'Objectif :** Faire apparaître une fenêtre vide et réussir à afficher dans ta console `Vector2 { x: 10.0, y: 5.0 }`.
*   **Pourquoi faire ses propres Maths ?** Pour comprendre comment un objet se déplace. Si tu as une position $A$ et une vitesse $B$, ta nouvelle position est $A + B$.
*   **L'aide :** Commence par créer une `struct Vec2 { x: f32, y: f32 }`. Ajoute une fonction pour les additionner.



---

## 🎨 Étape 2 : Parler à la Carte Graphique (wgpu)
**Le Contexte :** Ton processeur (CPU) est intelligent mais lent pour le dessin. Ta carte graphique (GPU) est "bête" mais ultra-rapide. `wgpu` est le traducteur qui permet à Rust de donner des ordres à la carte graphique.

*   **L'Objectif :** Remplir ta fenêtre avec une couleur unie (ex: un fond bleu spatial) qui change quand tu cliques.
*   **C'est quoi un "Shader" ?** C'est un mini-programme (écrit en langage WGSL) qui s'exécute sur ta carte graphique pour décider de la couleur de chaque pixel.
*   **L'aide :** Ne cherche pas à tout comprendre sur `wgpu` d'un coup. Copie le "boilerplate" (le code de base) d'un tutoriel `wgpu` pour juste "effacer l'écran" (clear screen) avec une couleur.

---

## ⚙️ Étape 3 : Le Cœur du Jeu (La Boucle de Tick)
**Le Contexte :** Un jeu est une boucle infinie : `Lire les entrées -> Calculer -> Dessiner`. Dans un Clicker, l'argent monte tout seul. Si ton PC rame, l'argent ne doit pas monter plus lentement !

*   **L'Objectif :** Faire en sorte qu'un compteur de "Ressources" augmente de +1 toutes les secondes exactement, peu importe si ton jeu tourne à 30 ou 144 FPS.
*   **Le "Tick Rate" :** C'est la fréquence de calcul de ta logique (ex: 60 fois par seconde).
*   **L'aide :** Utilise `std::time::Instant` pour calculer le temps écoulé entre deux images. Si le temps dépasse 1/60e de seconde, tu ajoutes tes ressources.

---

## 🖱️ Étape 4 : L'Interface (Cliquer sur des trucs)
**Le Contexte :** Pour un jeu de clic, il faut savoir si la souris est au-dessus de la planète. Comme on fait tout "from scratch", on n'a pas de boutons Windows ou HTML. On doit les dessiner nous-mêmes.

*   **L'Objectif :** Afficher un carré à l'écran. Si ta souris est à l'intérieur du carré quand tu cliques, ton score augmente.
*   **Le concept AABB :** C'est la façon la plus simple de gérer les collisions. On vérifie si $Souris_x > Carre_{gauche}$ et $Souris_x < Carre_{droit}$, etc.
*   **L'aide :** Dessine un carré simple avec `wgpu`. Récupère les coordonnées de ta souris via `winit` et compare-les aux limites de ton carré.



---

## 💰 Étape 5 : Les Grands Nombres (BigNumber)
**Le Contexte :** Dans un Clicker, on finit souvent avec des milliards de milliards de ressources. Les nombres classiques (`i32` ou `f64`) finissent par devenir imprécis ou exploser.

*   **L'Objectif :** Pouvoir afficher "1.5 Quadrillion" sans que le jeu ne plante.
*   **La solution :** Au lieu d'un seul nombre, on utilise une structure :
    $$ \text{Valeur} = \text{Mantisse} \times 10^{\text{Exposant}} $$
*   **L'aide :** Crée une `struct BigResource { mantissa: f64, exponent: i64 }`. Apprends à multiplier deux `BigResource`.