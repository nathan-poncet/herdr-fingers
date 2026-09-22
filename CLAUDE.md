# Working conventions for Claude

tmux-fingers for Herdr (Rust, Clean Architecture, TDD). Architecture rules:
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md). The essentials:

## Language & history
- Code, comments and commit messages in **English**. French docs mirror under `docs/fr/`.
- Prefix commits and issues with a **Gitmoji** (📝 docs, ✨ feat, 🐛 fix, ✅ tests, ♻️ refactor, 👷 ci, 🔒 security…).

## Design
- `src/domain/` is the kernel: pure, synchronous, deterministic. It may use `regex`,
  `unicode-width`, `serde` derives and `thiserror`, nothing else — no I/O, no
  processes, no terminal, no Herdr. `tests/dependency_rule.rs` enforces it.
- `src/adapters/` talks to the outside world behind small typed surfaces
  (Herdr socket, clipboard, terminal, config file, actions). `src/app.rs` is the
  composition root, one function per CLI subcommand.
- The renderer is a pure function of a `View`; the event loop only maps keys to
  kernel `Key`s and feeds the `Session` state machine.
- Newtypes and enums over strings and bools; validate at the edge (config.rs), so
  the kernel never sees a raw config value.
- Fail soft in the overlay: a broken `config.toml` falls back to defaults with a
  visible notice; an error before the first frame is shown on screen and logged
  to `$HERDR_PLUGIN_STATE_DIR/herdr-fingers.log`.

## Comments
- Prefer **self-documenting code**: precise names, small functions, strong types.
- Comment only a non-obvious *why* the code cannot express — never restate *what*.
- Never reference planning artifacts (issue IDs, milestones) in code.

## Tests
- TDD: red first; test names state behaviour (`a_row_filling_the_width_joins_the_next_row`), never `test_foo1`.
- Deterministic always: fake Herdr over a Unix socket in a temp dir, ratatui `TestBackend`, temp files. No real Herdr, no real clipboard, no sleeps.
- Before pushing: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --locked`. CI runs them on Linux and macOS.
- `herdr-plugin.toml` and `Cargo.toml` must agree on the version; `tests/manifest.rs` checks it.
