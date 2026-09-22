//! tmux-fingers for Herdr: hints over everything worth copying in a pane.
//!
//! `domain` is the pure kernel (parsing, matching, hint labels, the picking
//! state machine); `adapters` talk to Herdr, the terminal, the clipboard and
//! the file system; `app` wires them together for each CLI subcommand.

pub mod adapters;
pub mod app;
pub mod domain;
