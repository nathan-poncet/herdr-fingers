# herdr-fingers

**tmux-fingers pour [Herdr](https://herdr.dev).** Une touche, et chaque
chemin, URL, SHA Git, IP, UUID ou nombre à l'écran reçoit une étiquette
d'une ou deux lettres ; tape l'étiquette, c'est dans ton presse-papiers.
Maintiens Shift pour le taper dans la pane à la place, Ctrl pour l'ouvrir.
Sans souris, sans sélection à la main, sans remonter l'historique pour
retrouver la chose.

```text
$ git status
On branch main
Your branch is up to date with 'dorigin/main'.          ← étiquette « d »
        modified:   ssrc/domain/session.rs              ← étiquette « s »
        new file:   adocs/ARCHITECTURE.md               ← étiquette « a »
```

Un portage de [tmux-fingers](https://github.com/Morantron/tmux-fingers)
vers Herdr, réécrit en Rust : mêmes motifs, mêmes dispositions clavier,
mêmes actions et multi-sélection, sous forme de plugin Herdr natif.
([English](../../README.md))

## Installation

```sh
herdr plugin install nathan-poncet/herdr-fingers
```

Herdr clone le dépôt et le compile (une toolchain Rust est nécessaire :
[rustup.rs](https://rustup.rs)). Ajoute ensuite un raccourci dans
`~/.config/herdr/config.toml` :

```toml
[[keys.command]]
key = "prefix+f"
type = "plugin_action"
command = "nathan-poncet.herdr-fingers.start"
description = "copier avec des étiquettes"
```

Recharge et essaie :

```sh
herdr server reload-config
```

Appuie sur `prefix+f` (`ctrl+b` puis `f` par défaut) dans n'importe quelle pane.

## Utilisation

Des étiquettes apparaissent sur chaque correspondance. Les plus courtes
sont en bas de l'écran, là où se trouve la sortie la plus récente ; deux
textes identiques partagent la même étiquette.

| Touche | Effet |
|---|---|
| étiquette (`a`, `sd`…) | copie la correspondance dans le presse-papiers |
| `Shift` + étiquette | la tape dans la pane |
| `Ctrl` + étiquette | l'ouvre : URL dans le navigateur, fichier dans son application |
| `Alt` + étiquette | action personnalisée (aucune par défaut) |
| `Tab` | multi-sélection : choisis plusieurs étiquettes, puis `Tab` ou `Entrée` |
| `Retour arrière` | efface la dernière touche tapée |
| `?` | aide |
| `Échap`, `q`, `Ctrl+C` | ferme |

Taper la première lettre d'une étiquette à deux lettres masque toutes les
autres. Une URL ou un chemin coupé sur deux lignes par le terminal ne fait
qu'une étiquette et se copie sans le saut de ligne. L'overlay redessine la
pane exactement à sa place, avec ses propres couleurs, même quand l'onglet
est divisé.

## Ce qui reçoit une étiquette

Les motifs intégrés de tmux-fingers, portés un pour un :

| Nom | Reconnaît |
|---|---|
| `ip` | adresses IPv4 |
| `uuid` | UUID |
| `sha` | SHA Git (7 à 128 chiffres hexadécimaux) |
| `digit` | nombres de quatre chiffres ou plus |
| `url` | `http(s)://`, `git@`, `git://`, `ssh://`, `ftp://`, `file:///` |
| `path` | tout ce qui contient un `/` : `src/main.rs`, `~/.config`, `/etc/hosts` |
| `hex` | nombres `0x…` |
| `kubernetes` | noms de ressources Kubernetes (`configmap/…`, `deployment.apps/…`) |
| `kubernetes-pod` | noms de pods gérés par un deployment (`nginx-66b6c48dd5-7xb2r`) |
| `git-status` | le chemin d'une ligne `modified:` / `new file:` / `deleted:` |
| `git-status-branch` | la branche distante dans `Your branch is up to date with '…'` |
| `diff` | le chemin d'un en-tête `--- a/…` / `+++ b/…` |

Quand deux motifs commencent à la même colonne, le premier de ce tableau
gagne ; les motifs personnalisés passent avant tous les autres.

## Configuration

Le plugin écrit un `config.toml` entièrement commenté au premier
lancement. Pour le trouver :

```sh
herdr plugin config-dir nathan-poncet.herdr-fingers
```

Chaque clé est facultative ; [`examples/config.toml`](../../examples/config.toml)
les liste toutes avec leur valeur par défaut. Celles qu'on change le plus :

```toml
keyboard_layout = "azerty"          # ou qwerty-homerow, dvorak, colemak, …
# alphabet = "asdfghjkl"            # tes propres touches plutôt qu'une disposition
hint_position = "left"              # ou "right"

main_action = ":copy:"              # étiquette seule
shift_action = ":paste:"            # Shift + étiquette
ctrl_action = ":open:"              # Ctrl + étiquette
alt_action = ""                     # Alt + étiquette ; ex. "xargs nvim" ou un script

enabled_builtin_patterns = ["url", "path", "sha", "git-status"]

[[patterns]]
name = "ticket"
regex = "PROJ-[0-9]+"

[[patterns]]
name = "env"
regex = "env=(?P<match>[a-z0-9_-]+)"   # seul le groupe est copié

[style]
hint = { fg = "black", bg = "yellow", bold = true }
highlight = { fg = "yellow" }
backdrop = { dim = true }           # atténue tout ce qui n'est pas une correspondance
```

### Actions

`:copy:`, `:paste:` et `:open:` sont intégrées ; `""` ne fait rien. Tout le
reste est une ligne de commande, lancée depuis le répertoire de travail de
la pane avec le texte choisi sur son **stdin** et deux variables
d'environnement, exactement comme tmux-fingers : `MODIFIER` (`main`,
`shift`, `ctrl` ou `alt`) et `HINT`.

```toml
alt_action = "sh -c 'open \"https://github.com/search?q=$(cat)\"'"
```

En multi-sélection, les textes sont joints par `multi_separator` (une
espace par défaut) et passés à l'action du dernier modificateur utilisé.

### Presse-papiers

Par défaut, `:copy:` envoie le texte sous forme de séquence **OSC 52**.
Herdr la transmet au terminal depuis lequel tu es attaché, donc le texte
atterrit dans *ton* presse-papiers même quand le serveur Herdr tourne sur
une autre machine via SSH. La plupart des terminaux l'acceptent d'emblée
(Ghostty, Kitty, WezTerm, iTerm2, Alacritty, foot, Windows Terminal) ;
tmux et quelques autres demandent de l'activer. Mets `clipboard = "system"`
pour utiliser `pbcopy`, `wl-copy`, `xclip` ou `xsel` sur la machine qui fait
tourner Herdr, ou `"both"`.

Herdr peut afficher un toast quand une pane définit le presse-papiers
(`[ui.toast.clipboard]` dans la config de Herdr) ;
`show_copied_notification = true` ajoute un toast avec le texte copié via
le système de notifications de Herdr.

### Dispositions clavier

`keyboard_layout` accepte les dispositions de tmux-fingers : `qwerty`,
`azerty`, `qwertz`, `dvorak`, `colemak`, chacune avec ses variantes
`-homerow`, `-left-hand` et `-right-hand`. Les touches `c`, `i`, `m`, `n`
et `q` ne servent jamais d'étiquette, pour ne pas entrer en conflit avec
les commandes de l'overlay.

## Différences avec tmux-fingers

- **Pas de mode « jump ».** tmux-fingers peut placer le curseur du copy-mode
  sur une correspondance ; l'API de Herdr ne l'expose pas. Tout le reste
  est là : actions main/shift/ctrl/alt, `:copy:`/`:paste:`/`:open:`,
  multi-sélection, motifs personnalisés, dispositions clavier.
- **Syntaxe regex Rust** pour les motifs personnalisés (pas de look-around,
  pas de références arrière) ; les groupes nommés s'écrivent `(?P<match>…)`
  ou `(?<match>…)`.
- **Les couleurs sont conservées.** L'overlay redessine la pane avec son
  propre style.

## Dépannage

```sh
herdr plugin list                                            # présent et activé ?
herdr plugin action list --plugin nathan-poncet.herdr-fingers
herdr plugin log list --plugin nathan-poncet.herdr-fingers   # ce que Herdr a vu en lançant l'action
tail "$(herdr plugin config-dir nathan-poncet.herdr-fingers | sed 's#/config/#/state/#')/herdr-fingers.log"
```

Rien ne se passe sur la touche ? Vérifie l'identifiant d'action dans ton
bloc `[[keys.command]]` et lance `herdr server reload-config`. L'overlay
s'ouvre mais affiche « Nothing to pick » ? Essaie le moteur de motifs sur
une capture de la pane :

```sh
herdr pane read <pane-id> --source visible --format ansi > dump.txt
herdr-fingers scan --width <largeur de la pane> < dump.txt   # étiquette, ligne:colonne, texte
```

`herdr-fingers` est `target/release/herdr-fingers` dans le répertoire que
`herdr plugin list` indique comme racine du plugin.

## Prérequis

- Herdr 0.7 ou plus récent, sous Linux ou macOS
- Une toolchain Rust (1.85+) pour compiler : `herdr plugin install` lance `cargo build --release`

## Compiler depuis les sources

```sh
git clone https://github.com/nathan-poncet/herdr-fingers.git
cd herdr-fingers
cargo build --release
herdr plugin link "$PWD"
```

`cargo test` lance la suite : le parseur ANSI, le recollage des lignes
coupées, chaque motif intégré, la génération des étiquettes, la machine à
états de sélection, le chargeur de config, le client Herdr face à un faux
socket et le rendu sur un backend de test ratatui. `cargo fmt --check` et
`cargo clippy --all-targets -- -D warnings` doivent être propres ; la CI
exécute tout cela sous Linux et macOS.

Le code suit une petite Clean Architecture : un noyau pur sous
`src/domain/` (aucune E/S, vérifié par un test) et des adaptateurs pour
Herdr, le terminal, le presse-papiers et le fichier de config. Voir
[docs/fr/ARCHITECTURE.md](ARCHITECTURE.md).

## Contribuer

Commence par [CONTRIBUTING.md](../../CONTRIBUTING.md). Bugs et idées passent
par les modèles d'issue ; les failles de sécurité par
[SECURITY.md](../../SECURITY.md). Chacun est tenu au
[code de conduite](../../CODE_OF_CONDUCT.md).

## Crédits

[tmux-fingers](https://github.com/Morantron/tmux-fingers) de Jorge Morante
est l'original : les motifs, les dispositions clavier, les actions par
modificateur et la multi-sélection en viennent. Merci.

## Licence

[MIT](../../LICENSE).
