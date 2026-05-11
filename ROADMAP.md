# 🦀 Roadmap Magistrale : RustyEngine Framework (Édition Puriste)

Ce document est le plan de vol ultime pour créer un moteur de jeu 2D "from scratch" en Rust. L'approche est minimaliste : utiliser uniquement les primitives matérielles et système, et reconstruire chaque brique logique (Maths, ECS, UI, Physique) manuellement.

---

## 🛠️ Philosophie Technique & Stack Limitée
Pour maximiser l'apprentissage, le projet se limite strictement aux bibliothèques suivantes :
*   **Windowing :** `winit` (Gestion de la fenêtre et des événements).
*   **Graphics :** `wgpu` (Abstraction moderne du GPU, shaders en WGSL).
*   **Async :** `tokio` (Chargement non-bloquant des assets).
*   **Parallel :** `rayon` (Calculs intensifs et systèmes ECS maison).
*   **Data :** `serde` (Sauvegardes et configuration).
*   **DIY (Fait Maison) :** Mathématiques (Vec/Mat), ECS, UI, Physique, Animation, Audio, ProcGen.

---

## 🏗️ PROJET 1 : Cosmic Clicker (L'Incrémental)
**Objectif principal :** Maîtriser la boucle de jeu de base, la précision mathématique et l'interface utilisateur (UI).

### 🎯 Objectifs de Dev :
- **Maths DIY :** Créer les structures `Vec2`, `Vec3`, `Mat4` avec `#[repr(C)]`. Implémenter la matrice de projection Orthographique manuellement.
- **BigNumber DIY :** Gérer l'économie au-delà de $1.8 \times 10^{308}$ (Structure Mantisse `f64` + Exposant `i64`).
- **Tick Rate :** Séparer la logique (TPS) du rendu (FPS) pour des calculs de gain constants quelle que soit la fluidité visuelle.
- **UI Immediate Mode DIY :** Créer un système capable de générer des triangles pour des boutons et de l'affichage de texte (Glyph Atlas).

### 🎮 Features Game Design :
- **Le "Clic" Central :** Une planète sur laquelle cliquer avec une petite animation de scale (interpolation).
- **Auto-mineurs :** Des sondes spatiales qui génèrent des ressources automatiquement via le système de Tick.
- **Boutique d'Upgrades :** Augmentation de la valeur du clic et de la vitesse de production (Coûts exponentiels).
- **Prestige Simple :** Un bouton "Reset" qui donne un bonus permanent (multiplicateur) pour la prochaine partie.

---

## 🎨 PROJET 2 : Neon Swarm (Survivor-like)
**Objectif principal :** Gérer la performance brute, les collisions massives et l'instanciation GPU.

### 🎯 Objectifs de Dev :
- **ECS Maison :** Implémenter un système d'entités simple basé sur des **Sparse Sets** pour garantir la contiguïté mémoire et le cache CPU.
- **GPU Instancing :** Modifier le RenderPipeline pour dessiner 10 000 ennemis avec un seul appel de dessin (Buffer d'instances).
- **Object Pooling :** Recycler les projectiles pour éviter les pics d'allocation/désallocation.
- **Game Feel :** Implémenter un mouvement fluide avec accélération et friction.

### 🎮 Features Game Design :
- **Auto-Attaque :** Le joueur se concentre sur le déplacement ; le moteur gère le ciblage automatique.
- **Système d'XP :** Des orbes à ramasser avec détection de collision cercle/cercle.
- **Choix d'Améliorations :** À chaque niveau, choisir entre 3 bonus aléatoires (Vitesse, Dégâts, Cadence).
- **Spawn directionnel :** Les ennemis apparaissent toujours juste hors de la vue de la caméra (Frustum).

---

## 🌍 PROJET 3 : Rusty Valley (Stardew-like)
**Objectif principal :** Gérer l'espace (Tilemaps), l'inventaire et la persistance des données.

### 🎯 Objectifs de Dev :
- **Asset Manager (Tokio) :** Système de chargement asynchrone des textures retournant des `Handles` ou `Arc<Texture>`.
- **Tilemap Engine :** Rendu de grille optimisé utilisant une seule texture (Atlas) et un shader qui calcule les UV par position.
- **Y-Sorting :** Gérer la superposition des sprites (profondeur) en utilisant le Depth Buffer de `wgpu`.
- **Persistence :** Sauvegarder l'état complet du monde et de l'inventaire via `serde` (Bincode pour la performance).

### 🎮 Features Game Design :
- **Agriculture :** Labourer, planter, arroser (états gérés par des composants ECS et des timers).
- **Barre d'outils :** Système de "Hotbar" pour passer de la pioche à l'arrosoir.
- **Coffres :** Stocker des objets et les retrouver après avoir relancé le jeu (sérialisation de conteneurs).
- **Marchand :** Un PNJ simple pour transformer tes récoltes en monnaie BigNumber.

---

## 💀 PROJET 4 : Void Crawler (Isaac-like)
**Objectif principal :** Algorithmes complexes, génération procédurale et IA.

### 🎯 Objectifs de Dev :
- **ProcGen :** Algorithme de génération de donjons (Type Random Walk ou Grid-based).
- **IA State Machine :** Gérer les comportements ennemis : Idle, Chase (Poursuite), Attack.
- **Synergies d'objets :** Architecture permettant aux bonus de modifier les propriétés des projectiles (taille, couleur, effets).
- **SDF Text Rendering :** Implémenter le rendu de texte via Signed Distance Fields pour une netteté parfaite, peu importe le zoom.

### 🎮 Features Game Design :
- **Salles procédurales :** Chaque partie génère une carte différente avec des connexions logiques.
- **Larmes/Projectiles :** Tirer dans 4 directions avec une physique de "poids" et de portée.
- **Boss Room :** Une salle unique avec un ennemi aux patterns de tir complexes.
- **Objets Passifs :** Ramasser un objet qui change visuellement le personnage et ses statistiques.

---

## ⚙️ PROJET 5 : Rusty Factory (Automation)
**Objectif principal :** Architecture système, multithreading et optimisation de flux massifs.

### 🎯 Objectifs de Dev :
- **Spatial Partitioning :** Implémenter une Grille Spatiale ou un **Quadtree** pour limiter les calculs de collision aux zones proches.
- **Data-Oriented Design (DOD) :** Organiser les données en structures de tableaux (SoA) pour maximiser l'efficacité des cœurs CPU.
- **Multithreading (Rayon) :** Paralléliser la simulation des convoyeurs et des usines.
- **Command Pattern :** Encapsuler les actions de construction pour permettre le "Undo/Redo".

### 🎮 Features Game Design :
- **Convoyeurs :** Transporter des objets sur une grille avec logique de file d'attente (Splines ou segments).
- **Foreuses & Fonderies :** Systèmes interconnectés (Extraction -> Transformation -> Sortie).
- **Arbre technologique :** Débloquer de nouvelles machines via la consommation de ressources produites.
- **Logistique :** Gérer les goulots d'étranglement (si le tapis est plein, la machine s'arrête).

---

## 💡 Conseils Techniques du "Master"

1. **Alignement Mémoire :** Le GPU WGSL attend des données alignées sur 16 octets. Utilisez `#[repr(C)]` et ajoutez des paddings (`_pad: f32`) dans vos structs Rust si nécessaire.
2. **Sparse Sets pour l'ECS :** C'est la structure la plus simple pour un ECS maison performant. Elle permet des itérations rapides et des accès par ID constants.
3. **Le Pipeline UI :** Pour l'UI, ne faites pas de draw calls par bouton. Générez un énorme buffer de sommets (Vertices) pour toute l'interface et dessinez-le en un seul passage.
4. **Physique par Grille :** Pour Stardew et Factorio, ne simulez pas de physique complexe. Testez simplement la case `(x, y)` de votre grille. C'est un calcul en $O(1)$.
5. **Fun vs Graphismes :** Validez vos mécaniques avec des carrés et des ronds de couleurs. Si le "Neon Swarm" est jouable et fun avec des cubes, il sera légendaire avec des assets.

---