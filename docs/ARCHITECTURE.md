# Architecture

herdr-fingers is one Rust crate laid out as a Clean Architecture. The kernel
knows about screens, patterns, hints and picks; the use cases say what the
plugin does, against ports; the adapters implement those ports with Herdr,
ratatui, the clipboard and processes; `app.rs` wires them together once per
CLI subcommand.

## The Dependency Rule

Source dependencies point inward only. `tests/dependency_rule.rs` greps
`src/domain/` and `src/usecases/` and fails on `crate::adapters`,
`crate::app`, `ratatui`, `crossterm`, `serde_json`, `std::io`, `std::fs`,
`std::process`, `std::env`, `std::net`, `std::os`, `base64` or
`shell_words` (and, for the kernel, `crate::usecases`).

| Ring | Folder | Contents | May use |
|---|---|---|---|
| Kernel | `src/domain/` | `ansi`, `screen`, `patterns`, `matcher`, `hints`, `alphabet`, `session`, `geometry`, `settings`, `style` | `regex`, `unicode-width`, `thiserror` |
| Use cases | `src/usecases/` | `ports` (the traits), `start`, `pick`, `testing` (fakes) | the kernel |
| Adapters | `src/adapters/` | `herdr` implements `PaneHost`, `tui` implements `Picker`, `clipboard` implements `Clipboard`, `system` implements `Launcher`; `config` and `log` | the ports, the world |
| Composition root | `src/app.rs`, `src/main.rs` | one function per subcommand: `start`, `ui`, `scan` | anything |

## Ports

Four traits in `usecases/ports.rs`, one per kind of side effect the use
cases need. Each has a real adapter and an in-memory fake in
`usecases/testing.rs`.

| Port | Real adapter | Fake | Methods |
|---|---|---|---|
| `PaneHost` | `HerdrClient` (socket API) | `FakeHost` | `layout`, `read_visible`, `pane_label`, `pane_cwd`, `open_overlay`, `send_text`, `notify` |
| `Picker` | `TerminalPicker` (ratatui event loop) | `ScriptedPicker` (feeds keys to the real `Session`) | `pick(view, session) -> Outcome` |
| `Clipboard` | `SystemClipboard` (OSC 52 and/or a command) | `RecordingClipboard` | `copy(text)` |
| `Launcher` | `SystemLauncher` (`open`/`xdg-open`, shell command lines) | `RecordingLauncher` | `open(target, cwd)`, `run(command_line, stdin, env, cwd)` |

Errors cross a port as `PortError(String)`: by the time they reach a use
case there is nothing to do but show them.

## Two processes, one key press

Herdr plugins cannot draw on Herdr's screen; they get a pane. So a key
press goes through two short-lived processes, each running one use case:

```text
prefix+f
  └─ Herdr runs the plugin action  →  herdr-fingers start   (usecases::start)
        1. host.pane_label    is the focused pane already our overlay? then stop
        2. host.layout        where does the focused pane sit in the tab?
        3. host.open_overlay  entrypoint "overlay", env HERDR_FINGERS_GEOMETRY=pane|area|rect
  └─ Herdr opens an overlay pane (a zoomed split)  →  herdr-fingers ui   (usecases::pick)
        4. app: config::load  $HERDR_PLUGIN_CONFIG_DIR/config.toml → Settings (+ notice on error)
        5. host.read_visible  visible screen of the source pane, ANSI kept
        6. Screen::from_ansi → logical lines → PatternSet::find → Candidates
        7. Session::new       hints assigned (bottom first, identical texts shared)
        8. picker.pick        draw · read key · Session::press … until Picked/Cancelled
        9. dispatch           Copy → clipboard · Paste → host.send_text · Open → launcher.open · Shell → launcher.run
  └─ the process exits; Herdr closes the overlay and restores focus and zoom
```

The geometry travels as an environment variable because the layout must be
read **before** the overlay opens (the overlay changes it), and the screen
must be read **inside** the overlay process (it can be large, and the pane
keeps running meanwhile). It is encoded by hand
(`pane_id|x,y,w,h|x,y,w,h`) so the use cases need no JSON library.

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
  the tab, encode/decode for the environment, compute the content rectangle
  inside the overlay frame and a free strip for the status line.
- **`settings`** — `Action` (`:copy:`, `:paste:`, `:open:`, shell, nothing),
  `Actions` per modifier, `ClipboardMode`, `Theme`, `Settings`. Already
  validated: the kernel never sees a raw config value.
- **`style`** — `Color` (indexed / RGB, with name parsing) and `TextStyle`.

## Use cases

- **`start`** — `start(host, plugin_id, pane_id, patterns) -> Started`. Stops
  when the focused pane is already the overlay; otherwise locates the pane
  and opens the overlay with the geometry (and the optional pattern subset)
  in its environment.
- **`pick`** — `pick(request, deps) -> Picked`. Reads the pane, builds the
  screen and the session, hands them to the picker, then dispatches the
  selection to the action bound to the modifier: copy, paste, open, shell
  command (text on stdin, `MODIFIER` and `HINT` in the environment), or
  nothing. Returns a line for the log.

## Adapters

- **`herdr`** — newline-delimited JSON over the Unix socket in
  `HERDR_SOCKET_PATH`, one connection per request. `HerdrClient` has typed
  methods for the Herdr calls and implements `PaneHost` with them.
  `PluginContext` reads the environment Herdr injects.
- **`tui`** — `render(area, buffer, View)` is a pure function: paint the
  cells, patch highlight styles onto matched segments, write hints over the
  first (or last) cells of each match, then the status strip and the help
  box. `TerminalPicker` owns the event loop; `key_from_event` maps
  crossterm keys to kernel `Key`s (uppercase → Shift, Ctrl, Alt).
- **`clipboard`** — `SystemClipboard`: OSC 52 to stdout (Herdr forwards it
  to the attached client) and/or a local command (`pbcopy`, `wl-copy`,
  `xclip`, `xsel`).
- **`system`** — `SystemLauncher`: `open`/`xdg-open`, and shell command
  lines parsed with `shell-words`, run with the text on stdin.
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

Every kernel module has behaviour-named unit tests. The use cases run
end-to-end against the fakes: a fixed screen, a scripted key sequence
through the real `Session`, recording clipboard and launcher — so "Shift
pastes into the pane the hints came from" or "cancelling copies nothing"
are checked in CI without a terminal or a socket. Adapters are tested at
their own edge: the Herdr client against a fake server on a temp Unix
socket, the renderer on ratatui's `TestBackend`, the launcher via `sh -c`
into temp files, the config loader against temp directories.
`tests/dependency_rule.rs` enforces the ring boundaries and
`tests/manifest.rs` keeps `herdr-plugin.toml` and `Cargo.toml` in step.
