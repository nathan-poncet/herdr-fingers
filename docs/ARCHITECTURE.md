# Architecture

herdr-fingers is one Rust crate laid out as a small Clean Architecture. The
kernel knows about screens, patterns, hints and picks; it has never heard of
Herdr, ratatui or the clipboard. Everything that touches the world is an
adapter behind a small typed surface, and `app.rs` wires them together once
per CLI subcommand. ([Français](fr/ARCHITECTURE.md))

## The Dependency Rule

Source dependencies point inward only. `tests/dependency_rule.rs` greps
`src/domain/` and fails on `crate::adapters`, `ratatui`, `crossterm`,
`serde_json`, `std::io`, `std::fs`, `std::process`, `std::env`, `std::net`,
`std::os`, `base64` or `shell_words`.

| Ring | Folder | Contents | May use |
|---|---|---|---|
| Kernel | `src/domain/` | `ansi`, `screen`, `patterns`, `matcher`, `hints`, `alphabet`, `session`, `geometry`, `settings`, `style` | `regex`, `unicode-width`, `serde` derives, `thiserror` |
| Adapters | `src/adapters/` | `herdr` (socket client), `tui` (ratatui renderer + key mapping), `clipboard`, `actions`, `config`, `log` | the kernel, the world |
| Composition root | `src/app.rs`, `src/main.rs` | one function per subcommand: `start`, `ui`, `scan` | anything |

## Two processes, one key press

Herdr plugins cannot draw on Herdr's screen; they get a pane. So a key
press goes through two short-lived processes:

```text
prefix+f
  └─ Herdr runs the plugin action  →  herdr-fingers start
        1. pane.get        is the focused pane already our overlay? then stop
        2. pane.layout     where does the focused pane sit in the tab?
        3. plugin.pane.open  entrypoint "overlay", env HERDR_FINGERS_GEOMETRY={…}
  └─ Herdr opens an overlay pane (a zoomed split)  →  herdr-fingers ui
        4. config::load    $HERDR_PLUGIN_CONFIG_DIR/config.toml → Settings
        5. pane.read       visible screen of the source pane, ANSI kept
        6. Screen::from_ansi → logical lines → PatternSet::find → Candidates
        7. Session::new    hints assigned (bottom first, identical texts shared)
        8. tui::run        draw · read key · Session::press … until Picked/Cancelled
        9. actions::perform  copy (OSC 52 / command) · paste (pane.send_text) · open · shell
  └─ the process exits; Herdr closes the overlay and restores focus and zoom
```

The geometry travels as an environment variable because the layout must be
read **before** the overlay opens (the overlay changes it), and the screen
must be read **inside** the overlay process (it can be large, and the pane
keeps running meanwhile).

## Kernel modules

- **`ansi`** — turns Herdr's ANSI dump into `Row`s of `Cell { text, width, style }`.
  Handles SGR (16/256/truecolor, attributes), drops every other escape
  sequence, expands tabs, attaches combining marks to their base cell,
  counts wide characters as two columns.
- **`screen`** — `Screen` (rows + rendered width) and `LogicalLine`: rows glued
  back together when a row runs to the right edge and ends in ink, with a
  byte-to-cell map so a match on the joined text can be placed back on the
  grid as one `Segment` per row. Herdr's pane rectangle may include a
  separator column, so "runs to the edge" means width or width − 1.
- **`patterns`** — the tmux-fingers built-ins as `(name, regex)`, `PatternSet`
  compilation, and `find`: candidates from every pattern, sorted by start,
  then precedence, then length; a greedy sweep keeps them non-overlapping.
  The `match` group narrows the copied part; trailing whitespace is trimmed
  so `.+` never copies padding.
- **`matcher`** — runs the patterns over the logical lines and returns
  `Candidate { text, pattern, segments }` in reading order.
- **`hints`** — prefix-free labels over an alphabet: single keys first, then
  the worst key is expanded into two-key labels, and so on. Sorted shortest
  first, then by key preference.
- **`alphabet`** — the tmux-fingers layouts; reserved keys (`c i m n q`)
  removed so hints never collide with controls.
- **`session`** — the state machine. `Session::new` assigns hints (bottom-most
  candidate gets the best hint, identical texts share, a candidate shorter
  than its hint gets none). `press(Key) -> Outcome` handles prefixes,
  backspace, multi-select (Tab/Enter), help and cancel.
- **`geometry`** — `Rect`, `Layout`, `OverlayGeometry`: locate the pane in
  the tab, compute the content rectangle inside the overlay frame and a
  free strip for the status line.
- **`settings`** — `Action` (`:copy:`, `:paste:`, `:open:`, shell, nothing),
  `Actions` per modifier, `ClipboardMode`, `Theme`, `Settings`. Already
  validated: the kernel never sees a raw config value.
- **`style`** — `Color` (indexed / RGB, with name parsing) and `TextStyle`.

## Adapters

- **`herdr`** — newline-delimited JSON over the Unix socket in
  `HERDR_SOCKET_PATH`, one connection per request, typed helpers for the
  five methods used (`pane.layout`, `pane.read`, `pane.get`,
  `plugin.pane.open`, `pane.send_text`, `notification.show`). `PluginContext`
  reads the environment Herdr injects.
- **`tui`** — `render(area, buffer, View)` is a pure function: paint the
  cells, patch highlight styles onto matched segments, write hints over the
  first (or last) cells of each match, then the status strip and the help
  box. `run` owns the event loop and `key_from_event` maps crossterm keys to
  kernel `Key`s (uppercase → Shift, Ctrl, Alt).
- **`clipboard`** — OSC 52 to stdout (Herdr forwards it to the attached
  client) and/or a local command (`pbcopy`, `wl-copy`, `xclip`, `xsel`).
- **`actions`** — carries out the `Action` for a `Selection`: clipboard,
  `pane.send_text`, `open`/`xdg-open`, or a shell command with the text on
  stdin and `MODIFIER`/`HINT` in the environment.
- **`config`** — serde structs with `deny_unknown_fields`, converted to
  `Settings`; a missing file is written from `examples/config.toml`, which a
  test keeps equal to the defaults.
- **`log`** — appends to `$HERDR_PLUGIN_STATE_DIR/herdr-fingers.log`.

## Failure policy

- Anything wrong before the first frame (no socket, no pane, bad geometry)
  is shown full screen with "press any key" and logged, so the overlay does
  not just flash and vanish.
- A broken `config.toml` is logged, defaults apply, and the status strip
  shows the error — copying still works.
- Cancelling never touches the clipboard; a failed action is logged.

## Tests

Every kernel module has behaviour-named unit tests. Adapters are tested
without the world: the Herdr client against a fake server on a temp Unix
socket, the renderer on ratatui's `TestBackend`, actions via `sh -c` into
temp files, the config loader against temp directories.
`tests/dependency_rule.rs` enforces the ring boundaries and `tests/manifest.rs`
keeps `herdr-plugin.toml` and `Cargo.toml` in step.
