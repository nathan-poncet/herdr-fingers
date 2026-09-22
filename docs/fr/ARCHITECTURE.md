# Architecture

herdr-fingers est une seule crate Rust organisée en petite Clean
Architecture. Le noyau connaît les écrans, les motifs, les étiquettes et
les sélections ; il n'a jamais entendu parler de Herdr, de ratatui ni du
presse-papiers. Tout ce qui touche au monde extérieur est un adaptateur
derrière une petite surface typée, et `app.rs` assemble le tout, une fois
par sous-commande. ([English](../ARCHITECTURE.md))

## La règle de dépendance

Les dépendances de code pointent uniquement vers l'intérieur.
`tests/dependency_rule.rs` parcourt `src/domain/` et échoue sur
`crate::adapters`, `ratatui`, `crossterm`, `serde_json`, `std::io`,
`std::fs`, `std::process`, `std::env`, `std::net`, `std::os`, `base64` ou
`shell_words`.

| Anneau | Dossier | Contenu | Peut utiliser |
|---|---|---|---|
| Noyau | `src/domain/` | `ansi`, `screen`, `patterns`, `matcher`, `hints`, `alphabet`, `session`, `geometry`, `settings`, `style` | `regex`, `unicode-width`, dérivations `serde`, `thiserror` |
| Adaptateurs | `src/adapters/` | `herdr` (client socket), `tui` (rendu ratatui + touches), `clipboard`, `actions`, `config`, `log` | le noyau, le monde |
| Racine de composition | `src/app.rs`, `src/main.rs` | une fonction par sous-commande : `start`, `ui`, `scan` | tout |

## Deux processus pour une touche

Un plugin Herdr ne peut pas dessiner sur l'écran de Herdr ; il reçoit une
pane. Une pression de touche traverse donc deux processus éphémères :

```text
prefix+f
  └─ Herdr lance l'action du plugin  →  herdr-fingers start
        1. pane.get          la pane focalisée est-elle déjà notre overlay ? alors stop
        2. pane.layout       où se trouve la pane focalisée dans l'onglet ?
        3. plugin.pane.open  entrée « overlay », env HERDR_FINGERS_GEOMETRY={…}
  └─ Herdr ouvre une pane overlay (un split zoomé)  →  herdr-fingers ui
        4. config::load      $HERDR_PLUGIN_CONFIG_DIR/config.toml → Settings
        5. pane.read         écran visible de la pane source, ANSI conservé
        6. Screen::from_ansi → lignes logiques → PatternSet::find → Candidates
        7. Session::new      étiquettes attribuées (le bas d'abord, textes identiques partagés)
        8. tui::run          dessiner · lire une touche · Session::press … jusqu'à Picked/Cancelled
        9. actions::perform  copier (OSC 52 / commande) · coller (pane.send_text) · ouvrir · shell
  └─ le processus se termine ; Herdr ferme l'overlay et restaure focus et zoom
```

La géométrie voyage dans une variable d'environnement parce que la
disposition doit être lue **avant** l'ouverture de l'overlay (qui la
modifie), tandis que l'écran doit être lu **dans** le processus de
l'overlay (il peut être volumineux, et la pane continue de tourner).

## Modules du noyau

- **`ansi`** : transforme la capture ANSI de Herdr en `Row` de
  `Cell { text, width, style }`. Gère les SGR (16/256 couleurs, truecolor,
  attributs), ignore toute autre séquence d'échappement, développe les
  tabulations, rattache les diacritiques combinants à leur cellule, compte
  les caractères larges pour deux colonnes.
- **`screen`** : `Screen` (lignes + largeur de rendu) et `LogicalLine` :
  lignes recollées quand une ligne va jusqu'au bord droit et se termine par
  de l'encre, avec une table octet → cellule pour replacer une
  correspondance sur la grille sous forme d'un `Segment` par ligne. Le
  rectangle de pane de Herdr peut inclure une colonne de séparation, donc
  « jusqu'au bord » signifie largeur ou largeur − 1.
- **`patterns`** : les motifs intégrés de tmux-fingers en `(nom, regex)`, la
  compilation d'un `PatternSet` et `find` : candidats de tous les motifs,
  triés par début, puis priorité, puis longueur ; un balayage glouton les
  garde disjoints. Le groupe `match` restreint la partie copiée ; les
  blancs de fin sont retirés pour qu'un `.+` ne copie jamais de remplissage.
- **`matcher`** : applique les motifs aux lignes logiques et renvoie des
  `Candidate { text, pattern, segments }` dans l'ordre de lecture.
- **`hints`** : étiquettes sans préfixe commun sur un alphabet : d'abord des
  touches seules, puis la moins bonne touche est développée en étiquettes à
  deux touches, etc. Triées par longueur, puis par préférence des touches.
- **`alphabet`** : les dispositions de tmux-fingers ; touches réservées
  (`c i m n q`) retirées pour que les étiquettes n'entrent jamais en conflit
  avec les commandes.
- **`session`** : la machine à états. `Session::new` attribue les
  étiquettes (le candidat le plus bas reçoit la meilleure, les textes
  identiques partagent, un candidat plus court que son étiquette n'en reçoit
  pas). `press(Key) -> Outcome` gère les préfixes, le retour arrière, la
  multi-sélection (Tab/Entrée), l'aide et l'annulation.
- **`geometry`** : `Rect`, `Layout`, `OverlayGeometry` : localiser la pane
  dans l'onglet, calculer le rectangle de contenu dans le cadre de l'overlay
  et une bande libre pour la ligne d'état.
- **`settings`** : `Action` (`:copy:`, `:paste:`, `:open:`, shell, rien),
  `Actions` par modificateur, `ClipboardMode`, `Theme`, `Settings`. Déjà
  validés : le noyau ne voit jamais une valeur brute de config.
- **`style`** : `Color` (indexée / RVB, avec analyse des noms) et `TextStyle`.

## Adaptateurs

- **`herdr`** : JSON délimité par des sauts de ligne sur le socket Unix de
  `HERDR_SOCKET_PATH`, une connexion par requête, des aides typées pour les
  méthodes utilisées (`pane.layout`, `pane.read`, `pane.get`,
  `plugin.pane.open`, `pane.send_text`, `notification.show`).
  `PluginContext` lit l'environnement injecté par Herdr.
- **`tui`** : `render(area, buffer, View)` est une fonction pure : peindre
  les cellules, appliquer le style de surbrillance aux segments, écrire les
  étiquettes sur les premières (ou dernières) cellules de chaque
  correspondance, puis la bande d'état et la boîte d'aide. `run` possède la
  boucle d'événements et `key_from_event` traduit les touches crossterm en
  `Key` du noyau (majuscule → Shift, Ctrl, Alt).
- **`clipboard`** : OSC 52 sur stdout (Herdr le transmet au client attaché)
  et/ou une commande locale (`pbcopy`, `wl-copy`, `xclip`, `xsel`).
- **`actions`** : exécute l'`Action` d'une `Selection` : presse-papiers,
  `pane.send_text`, `open`/`xdg-open`, ou une commande shell avec le texte
  sur stdin et `MODIFIER`/`HINT` dans l'environnement.
- **`config`** : structures serde avec `deny_unknown_fields`, converties en
  `Settings` ; un fichier absent est écrit à partir de
  `examples/config.toml`, qu'un test maintient égal aux valeurs par défaut.
- **`log`** : ajoute à `$HERDR_PLUGIN_STATE_DIR/herdr-fingers.log`.

## Politique d'échec

- Tout problème avant la première image (pas de socket, pas de pane,
  géométrie invalide) s'affiche en plein écran avec « press any key » et
  est journalisé : l'overlay ne fait pas que clignoter.
- Un `config.toml` cassé est journalisé, les valeurs par défaut
  s'appliquent et la bande d'état affiche l'erreur : la copie fonctionne
  toujours.
- Annuler ne touche jamais au presse-papiers ; une action échouée est journalisée.

## Tests

Chaque module du noyau a des tests unitaires nommés par comportement. Les
adaptateurs sont testés sans le monde extérieur : le client Herdr face à un
faux serveur sur un socket Unix temporaire, le rendu sur le `TestBackend`
de ratatui, les actions via `sh -c` vers des fichiers temporaires, le
chargeur de config dans des répertoires temporaires.
`tests/dependency_rule.rs` fait respecter les frontières des anneaux et
`tests/manifest.rs` garde `herdr-plugin.toml` et `Cargo.toml` alignés.
