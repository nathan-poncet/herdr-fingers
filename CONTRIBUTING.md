# Contributing to herdr-fingers

Thanks for helping! This is a small codebase with a few firm rules — this
page is everything you need to get a green PR on the first try.

## Setup

```sh
git clone https://github.com/nathan-poncet/herdr-fingers.git
cd herdr-fingers
cargo build --release
herdr plugin link "$PWD"          # Herdr now runs your build on prefix+f
```

`herdr plugin link` points Herdr at this checkout; rebuild with
`cargo build --release` and the next `prefix+f` uses the new binary. Unlink
with `herdr plugin unlink nathan-poncet.herdr-fingers`.

Useful while developing:

```sh
herdr pane read <pane-id> --source visible --format ansi > dump.txt
cargo run -- scan --width 120 < dump.txt      # what would get a hint, and which
cargo run -- default-config                   # the commented default config
```

To watch the overlay without opening it over your own screen, run
`herdr-fingers ui` in a scratch pane with `HERDR_FINGERS_GEOMETRY` set to a
JSON `{"pane_id": …, "area": {…}, "pane": {…}}` (see
`OverlayGeometry`); it reads that pane and draws in the scratch one.

## Before you push

CI runs exactly these, on Linux and macOS — run them locally first:

```sh
cargo fmt --all --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
cargo build --release --locked
```

## Architecture in one minute

Clean Architecture in one crate — the rings are folders:

| Ring | Folder | May use |
|---|---|---|
| Kernel | `src/domain/` | `regex`, `unicode-width`, `serde` derives, `thiserror` — **no I/O** |
| Adapters | `src/adapters/` | Herdr socket, terminal (ratatui/crossterm), clipboard, files, processes |
| Composition root | `src/app.rs`, `src/main.rs` | anything |

`tests/dependency_rule.rs` fails if the kernel imports an adapter, ratatui,
crossterm, `std::io`, `std::fs`, `std::process`… Full rules and a walk
through one key press: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

Ground rules:

- **Validate at the edge.** `adapters/config.rs` turns the TOML into typed
  `Settings`; the kernel never sees a string it has to interpret.
- **Render is pure.** `tui::render` takes a `View` and paints a buffer; the
  event loop only maps keys and feeds the `Session` state machine.
- **Fail soft in the overlay.** A bad `config.toml` falls back to defaults
  with a visible notice; an error before the first frame is shown on screen
  and logged to `$HERDR_PLUGIN_STATE_DIR/herdr-fingers.log`.

## Tests

TDD is the house style: write the failing test first.

- Test names state behaviour: `a_row_filling_the_width_joins_the_next_row`,
  never `test_screen_1`.
- Deterministic always: the Herdr client is tested against a fake server on
  a Unix socket in a temp directory; the renderer against ratatui's
  `TestBackend`; actions against `sh -c` writing to temp files. No real
  Herdr, no real clipboard, no sleeps.
- A new built-in pattern gets a row in `each_builtin_recognises_its_canonical_example`
  and a line in both READMEs.

## Commits & PRs

- Code, comments and commit messages are in **English**; French docs mirror
  under `docs/fr/` and must be updated in the same PR.
- Prefix commit subjects with a [Gitmoji](https://gitmoji.dev): ✨ feature,
  🐛 fix, ♻️ refactor, ✅ tests, 📝 docs, 👷 CI, 🔒 security…
- Keep PRs focused: one feature or fix per PR, with tests for behaviour
  changes.
- Anything a user would notice gets a line under `[Unreleased]` in
  [CHANGELOG.md](CHANGELOG.md); the release moves that section under the
  new version and its notes are taken from it.

## Releasing

1. Move the `[Unreleased]` entries under a new version heading in `CHANGELOG.md`.
2. Bump `version` in **both** `Cargo.toml` and `herdr-plugin.toml`
   (`./scripts/check-version.sh` verifies they agree), run `cargo build` so
   `Cargo.lock` follows.
3. Commit (`🔖 Release x.y.z`), tag `vx.y.z`, push the tag. The release
   workflow tests, builds Linux and macOS archives, and publishes a GitHub
   release with the CHANGELOG section as notes.

Herdr users get the new version with `herdr plugin install nathan-poncet/herdr-fingers`
(pin one with `--ref vx.y.z`); the marketplace picks it up within the hour.

## Reporting bugs & proposing features

Use the issue templates. Security flaws go through the private channel
described in [SECURITY.md](SECURITY.md), never through a public issue.
Everyone taking part is held to the [code of conduct](CODE_OF_CONDUCT.md).

## Licensing of contributions

herdr-fingers is MIT. By opening a pull request you agree that your
contribution is licensed under the same terms.
