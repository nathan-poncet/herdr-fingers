# Changelog

All notable changes to herdr-fingers are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the versions
[Semantic Versioning](https://semver.org/). Entries describe what a user of
the plugin notices; the build pipeline and the docs live in the commit history.

## [Unreleased]

## [0.1.0] - 2026-09-22

### Added
- Hints over everything worth copying in the focused Herdr pane: paths,
  URLs, Git SHAs, IPv4 addresses, UUIDs, hex numbers, long numbers,
  Kubernetes resources and pod names, `git status` paths and branch names,
  diff headers — the tmux-fingers built-ins, ported one for one.
- Type a hint to copy its text through Herdr to the terminal you are
  looking at (OSC 52), so it works over SSH; hold Shift to type it into the
  pane instead, Ctrl to open it with the system opener, Alt for a custom
  command. Every action is configurable, including arbitrary shell commands
  fed on stdin with `MODIFIER` and `HINT` in their environment.
- Multi-select with Tab: pick several hints, confirm with Tab or Enter, get
  them joined by a configurable separator.
- The overlay redraws the pane where it is, with its own colors, even when
  the tab is split; a URL or path wrapped over two rows is one hint.
- Identical texts share one hint, and the shortest hints go to the bottom
  of the screen, where the freshest output is.
- Keyboard layouts from tmux-fingers (qwerty, azerty, qwertz, dvorak,
  colemak and their home-row and one-hand variants), or your own alphabet.
- A commented `config.toml` written on first run; custom patterns in Rust
  regex syntax with an optional `match` group.
- `herdr-fingers scan` to preview the hints a screen dump would get.
