//! Carries out the action bound to the modifier the user held.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use thiserror::Error;

use super::clipboard::{self, ClipboardError};
use super::herdr::{HerdrClient, HerdrError};
use crate::domain::session::Selection;
use crate::domain::settings::{Action, Settings};

#[derive(Debug, Error)]
pub enum ActionError {
    #[error(transparent)]
    Clipboard(#[from] ClipboardError),
    #[error(transparent)]
    Herdr(#[from] HerdrError),
    #[error("cannot run `{command}`: {source}")]
    Spawn {
        command: String,
        #[source]
        source: std::io::Error,
    },
    #[error("`{0}` exited with an error")]
    Failed(String),
    #[error("cannot parse the action command line: {0}")]
    CommandLine(#[from] shell_words::ParseError),
}

/// Everything an action may need besides the picked text.
pub struct ActionContext<'a> {
    pub client: &'a HerdrClient,
    pub pane_id: &'a str,
    pub cwd: Option<PathBuf>,
    pub settings: &'a Settings,
}

/// Runs the action for `selection`; returns a short status message.
pub fn perform(selection: &Selection, ctx: &ActionContext<'_>) -> Result<String, ActionError> {
    let text = selection.texts.join(&ctx.settings.multi_separator);
    let action = ctx.settings.actions.for_modifier(selection.modifier);
    match action {
        Action::Copy => {
            clipboard::copy(
                &text,
                ctx.settings.clipboard,
                &ctx.settings.clipboard_command,
            )?;
            if ctx.settings.notify_on_copy
                && let Err(error) = ctx.client.notify("Copied", &preview(&text))
            {
                return Ok(format!("copied (toast failed: {error})"));
            }
            Ok(format!("copied {}", preview(&text)))
        }
        Action::Paste => {
            ctx.client.send_text(ctx.pane_id, &text)?;
            Ok(format!("pasted {}", preview(&text)))
        }
        Action::Open => {
            for item in &selection.texts {
                open_with_system(item, ctx.cwd.as_deref())?;
            }
            Ok(format!("opened {}", preview(&text)))
        }
        Action::Shell(command_line) => {
            run_shell_action(command_line, &text, selection, ctx.cwd.as_deref())?;
            Ok(format!("ran {command_line}"))
        }
        Action::Nothing => Ok(String::new()),
    }
}

fn open_with_system(target: &str, cwd: Option<&std::path::Path>) -> Result<(), ActionError> {
    let opener = if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    let mut command = Command::new(opener);
    command
        .arg(target)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if let Some(dir) = cwd {
        command.current_dir(dir);
    }
    let status = command.status().map_err(|source| ActionError::Spawn {
        command: format!("{opener} {target}"),
        source,
    })?;
    if status.success() {
        Ok(())
    } else {
        Err(ActionError::Failed(format!("{opener} {target}")))
    }
}

fn run_shell_action(
    command_line: &str,
    text: &str,
    selection: &Selection,
    cwd: Option<&std::path::Path>,
) -> Result<(), ActionError> {
    let argv = shell_words::split(command_line)?;
    let Some((program, args)) = argv.split_first() else {
        return Ok(());
    };
    let mut command = Command::new(program);
    command
        .args(args)
        .env("MODIFIER", selection.modifier.as_str())
        .env("HINT", &selection.hint)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if let Some(dir) = cwd {
        command.current_dir(dir);
    }
    let mut child = command.spawn().map_err(|source| ActionError::Spawn {
        command: command_line.to_string(),
        source,
    })?;
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(text.as_bytes());
    }
    let status = child.wait().map_err(|source| ActionError::Spawn {
        command: command_line.to_string(),
        source,
    })?;
    if status.success() {
        Ok(())
    } else {
        Err(ActionError::Failed(command_line.to_string()))
    }
}

/// The first line of `text`, shortened for a status line or a toast.
pub fn preview(text: &str) -> String {
    const MAX_CHARS: usize = 40;
    let first_line = text.lines().next().unwrap_or("");
    let mut shortened: String = first_line.chars().take(MAX_CHARS).collect();
    if first_line.chars().count() > MAX_CHARS || text.lines().count() > 1 {
        shortened.push('…');
    }
    shortened
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::session::Modifier;

    fn selection(texts: &[&str], modifier: Modifier) -> Selection {
        Selection {
            texts: texts.iter().map(|t| t.to_string()).collect(),
            hint: "ab".into(),
            modifier,
        }
    }

    fn temp_file(name: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("herdr-fingers-{name}-{unique}"))
    }

    #[test]
    fn a_shell_action_gets_the_text_on_stdin_and_the_hint_in_its_environment() {
        let target = temp_file("action");
        let mut settings = Settings::default();
        settings.actions.alt = Action::Shell(format!(
            "sh -c 'printf \"%s|%s|\" \"$MODIFIER\" \"$HINT\" > {0}; cat >> {0}'",
            target.display()
        ));
        settings.multi_separator = "+".into();
        let client = HerdrClient::new("/nonexistent.sock");
        let ctx = ActionContext {
            client: &client,
            pane_id: "w1:p1",
            cwd: None,
            settings: &settings,
        };
        let message = perform(&selection(&["one", "two"], Modifier::Alt), &ctx).unwrap();
        assert!(message.starts_with("ran sh -c"));
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "alt|ab|one+two");
        let _ = std::fs::remove_file(target);
    }

    #[test]
    fn a_failing_shell_action_is_reported() {
        let mut settings = Settings::default();
        settings.actions.main = Action::Shell("sh -c 'exit 2'".into());
        let client = HerdrClient::new("/nonexistent.sock");
        let ctx = ActionContext {
            client: &client,
            pane_id: "w1:p1",
            cwd: None,
            settings: &settings,
        };
        let error = perform(&selection(&["x"], Modifier::Main), &ctx).unwrap_err();
        assert!(matches!(error, ActionError::Failed(_)));
    }

    #[test]
    fn nothing_does_nothing_and_says_nothing() {
        let settings = Settings::default();
        let client = HerdrClient::new("/nonexistent.sock");
        let ctx = ActionContext {
            client: &client,
            pane_id: "w1:p1",
            cwd: None,
            settings: &settings,
        };
        assert_eq!(
            perform(&selection(&["x"], Modifier::Alt), &ctx).unwrap(),
            ""
        );
    }

    #[test]
    fn previews_are_short_single_lines() {
        assert_eq!(preview("short"), "short");
        assert_eq!(preview("first\nsecond"), "first…");
        assert_eq!(preview(&"x".repeat(50)), format!("{}…", "x".repeat(40)));
    }
}
