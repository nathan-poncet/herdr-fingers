//! The composition root: one function per CLI subcommand.

use std::collections::BTreeMap;
use std::io::Read;

use thiserror::Error;

use crate::adapters::actions::{self, ActionContext, ActionError};
use crate::adapters::config::{self, ConfigError};
use crate::adapters::herdr::{HerdrClient, HerdrError, PluginContext};
use crate::adapters::{log, tui};
use crate::domain::geometry::{GeometryError, OverlayGeometry};
use crate::domain::matcher::find_candidates;
use crate::domain::patterns::{self, PatternError, PatternSet};
use crate::domain::screen::Screen;
use crate::domain::session::{Outcome, Session};
use crate::domain::settings::Settings;

/// Environment variable carrying the overlay geometry from `start` to `ui`.
pub const GEOMETRY_ENV: &str = "HERDR_FINGERS_GEOMETRY";
/// Environment variable carrying a comma-separated pattern subset.
pub const PATTERNS_ENV: &str = "HERDR_FINGERS_PATTERNS";
/// The pane title declared in `herdr-plugin.toml` for the overlay.
pub const OVERLAY_TITLE: &str = "Fingers";
const OVERLAY_ENTRYPOINT: &str = "overlay";

#[derive(Debug, Error)]
pub enum AppError {
    #[error(transparent)]
    Herdr(#[from] HerdrError),
    #[error(transparent)]
    Geometry(#[from] GeometryError),
    #[error(transparent)]
    Pattern(#[from] PatternError),
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    Action(#[from] ActionError),
    #[error("terminal error: {0}")]
    Terminal(#[from] std::io::Error),
    #[error("no focused pane: Herdr did not pass HERDR_PANE_ID")]
    NoFocusedPane,
    #[error("invalid {GEOMETRY_ENV}: {0}")]
    BadGeometry(serde_json::Error),
    #[error("unknown option `{0}`")]
    UnknownOption(String),
}

/// `start`: the plugin action. Records where the focused pane sits, then
/// asks Herdr to open the overlay over it.
pub fn start(args: &[String]) -> Result<(), AppError> {
    let context = PluginContext::from_env();
    let client = HerdrClient::from_env()?;
    let pane_id = context.focused_pane_id.ok_or(AppError::NoFocusedPane)?;
    if client.pane_label(&pane_id)?.as_deref() == Some(OVERLAY_TITLE) {
        return Ok(());
    }
    let layout = client.layout(&pane_id)?;
    let geometry = OverlayGeometry::locate(&layout, &pane_id)?;
    let mut env = BTreeMap::new();
    env.insert(
        GEOMETRY_ENV.to_string(),
        serde_json::to_string(&geometry).expect("geometry serializes"),
    );
    if let Some(names) = option_value(args, "--patterns")? {
        let names: Vec<&str> = names.split(',').map(str::trim).collect();
        patterns::builtin_specs(&names)?;
        env.insert(PATTERNS_ENV.to_string(), names.join(","));
    }
    client.open_plugin_pane(&context.plugin_id, OVERLAY_ENTRYPOINT, env)?;
    Ok(())
}

/// `ui`: runs inside the overlay pane. Errors are shown on screen before the
/// pane closes, and logged to the plugin's state directory.
pub fn ui() -> Result<(), AppError> {
    let context = PluginContext::from_env();
    match run_overlay(&context) {
        Ok(()) => Ok(()),
        Err(error) => {
            log::append(context.state_dir.as_deref(), &format!("error: {error}"));
            tui::show_error_and_wait(&error.to_string())?;
            Err(error)
        }
    }
}

fn run_overlay(context: &PluginContext) -> Result<(), AppError> {
    let client = HerdrClient::from_env()?;
    let geometry: Option<OverlayGeometry> = match std::env::var(GEOMETRY_ENV) {
        Ok(json) => Some(serde_json::from_str(&json).map_err(AppError::BadGeometry)?),
        Err(_) => None,
    };
    let pane_id = geometry
        .as_ref()
        .map(|g| g.pane_id.clone())
        .or_else(|| context.focused_pane_id.clone())
        .ok_or(AppError::NoFocusedPane)?;
    let width = match &geometry {
        Some(geometry) => geometry.pane.width,
        None => client
            .layout(&pane_id)
            .ok()
            .and_then(|layout| layout.panes.into_iter().find(|p| p.pane_id == pane_id))
            .map_or(0, |p| p.rect.width),
    };

    let (mut settings, notice) = match config::load(context.config_dir.as_deref()) {
        Ok(settings) => (settings, None),
        Err(error) => {
            log::append(context.state_dir.as_deref(), &format!("config: {error}"));
            (
                Settings::default(),
                Some(format!("config.toml ignored: {error}")),
            )
        }
    };
    if let Ok(names) = std::env::var(PATTERNS_ENV) {
        let names: Vec<&str> = names.split(',').map(str::trim).collect();
        settings.patterns = PatternSet::compile(&patterns::builtin_specs(&names)?)?;
    }

    let screen = Screen::from_ansi(&client.read_visible(&pane_id)?, width);
    let candidates = find_candidates(&screen, &settings.patterns);
    let mut session = Session::new(candidates, &settings.alphabet);

    let outcome = tui::run(
        &screen,
        &mut session,
        &settings.theme,
        geometry.as_ref(),
        notice.as_deref(),
    )?;

    if let Outcome::Picked(selection) = outcome {
        let ctx = ActionContext {
            client: &client,
            pane_id: &pane_id,
            cwd: client.pane_cwd(&pane_id).unwrap_or(None),
            settings: &settings,
        };
        let message = actions::perform(&selection, &ctx)?;
        log::append(context.state_dir.as_deref(), &message);
    }
    Ok(())
}

/// `scan`: reads a screen dump on stdin and prints what would get a hint.
/// Handy for trying custom patterns: `herdr pane read <id> | herdr-fingers scan`.
pub fn scan(args: &[String]) -> Result<(), AppError> {
    let mut text = String::new();
    std::io::stdin().read_to_string(&mut text)?;
    let width = option_value(args, "--width")?
        .and_then(|value| value.parse().ok())
        .unwrap_or(0);
    let context = PluginContext::from_env();
    let settings = config::load(context.config_dir.as_deref())?;
    let screen = Screen::from_ansi(&text, width);
    let session = Session::new(
        find_candidates(&screen, &settings.patterns),
        &settings.alphabet,
    );
    for target in session.targets() {
        let first = target.segments[0];
        println!(
            "{}\t{}:{}\t{}",
            target.hint,
            first.row + 1,
            first.col + 1,
            target.text
        );
    }
    Ok(())
}

fn option_value<'a>(args: &'a [String], name: &str) -> Result<Option<&'a str>, AppError> {
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if let Some(value) = arg.strip_prefix(&format!("{name}=")) {
            return Ok(Some(value));
        }
        if arg == name {
            return Ok(iter.next().map(String::as_str));
        }
        if arg.starts_with("--") {
            return Err(AppError::UnknownOption(arg.clone()));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn options_accept_both_spellings_and_reject_unknown_ones() {
        assert_eq!(
            option_value(&args(&["--patterns", "url,path"]), "--patterns").unwrap(),
            Some("url,path")
        );
        assert_eq!(
            option_value(&args(&["--patterns=url"]), "--patterns").unwrap(),
            Some("url")
        );
        assert_eq!(option_value(&args(&[]), "--patterns").unwrap(), None);
        assert!(matches!(
            option_value(&args(&["--bogus"]), "--patterns"),
            Err(AppError::UnknownOption(name)) if name == "--bogus"
        ));
    }
}
