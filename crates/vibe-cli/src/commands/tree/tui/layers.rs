//! Layer browser over the same descriptors as plain/JSON; hiding is never disabling.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-074#LAYERS-IN-TREE");
use super::super::{
    layers::{Layer, collect},
    model::PackageTree,
    plain,
};
use super::{AppEvent, theme::Theme, ui::Window};
use anyhow::Result;
use rat_salsa::{Control, RunConfig, SalsaAppContext, poll::PollCrossterm, run_tui};
use ratatui_core::{buffer::Buffer, layout::Rect, text::Line, widgets::Widget};
use ratatui_crossterm::crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui_widgets::paragraph::Paragraph;
use std::path::{Path, PathBuf};

type Global = SalsaAppContext<AppEvent, anyhow::Error>;
struct Browser {
    root: PathBuf,
    project: String,
    layers: Vec<Layer>,
    hidden: Vec<bool>,
    selected: usize,
    offset: usize,
    message: String,
    fatal: Option<anyhow::Error>,
    theme: Theme,
}
impl Browser {
    fn new(root: &Path, project: &PackageTree, layers: Vec<Layer>) -> Self {
        Self {
            root: root.into(),
            project: plain::render(project),
            hidden: vec![false; layers.len()],
            layers,
            selected: 0,
            offset: 0,
            message: String::new(),
            fatal: None,
            theme: Theme::default(),
        }
    }
    fn lines(&self) -> Vec<String> {
        let mut lines = vec!["Layers: Up/Down select · Space hide/show · Enter enable/disable personal/override · PgUp/PgDn scroll · Esc quit".into()];
        for (index, layer) in self.layers.iter().enumerate() {
            lines.push(format!(
                "{} {} [{}] {} — {}",
                if index == self.selected { ">" } else { " " },
                layer.id,
                if self.hidden[index] {
                    "hidden"
                } else {
                    "shown"
                },
                if layer.enabled { "enabled" } else { "disabled" },
                layer.state
            ));
        }
        lines.push(self.message.clone());
        let selected = &self.layers[self.selected];
        if !self.hidden[self.selected] {
            lines.push(format!(
                "Layer {} (independent package identities)",
                selected.id
            ));
            let text = if selected.id == "project" {
                Some(self.project.clone())
            } else {
                selected.tree.as_ref().map(plain::render)
            };
            if let Some(text) = text {
                lines.extend(text.lines().map(str::to_owned));
            }
            lines.extend(selected.relations.iter().map(|r| r.display_line()));
            lines.extend(selected.diagnostics.iter().cloned());
        }
        lines
    }
    fn handle(&mut self, event: &Event, controls: bool) -> Result<Control<AppEvent>> {
        let Event::Key(key) = event else {
            return Ok(Control::Changed);
        };
        if key.kind != KeyEventKind::Press {
            return Ok(Control::Continue);
        }
        match key.code {
            KeyCode::Esc => return Ok(Control::Quit),
            KeyCode::Up => {
                self.selected = self.selected.saturating_sub(1);
                self.offset = 0;
            }
            KeyCode::Down => {
                self.selected = (self.selected + 1).min(self.layers.len() - 1);
                self.offset = 0;
            }
            KeyCode::PageDown => self.offset = self.offset.saturating_add(10),
            KeyCode::PageUp => self.offset = self.offset.saturating_sub(10),
            KeyCode::Char(' ') => self.hidden[self.selected] = !self.hidden[self.selected],
            KeyCode::Enter => {
                if self.layers[self.selected].id == "project" {
                    self.message =
                        "The project layer is mandatory; hide/show changes this view only".into();
                } else if !controls {
                    self.message =
                        "Persisted controls are unavailable in read-only rendering".into();
                } else {
                    let enabled = self.layers[self.selected].enabled;
                    let command = if enabled {
                        crate::cli::UserOverrideCommand::Disable {
                            path: self.root.clone(),
                        }
                    } else {
                        crate::cli::UserOverrideCommand::Enable {
                            path: self.root.clone(),
                        }
                    };
                    let quiet = crate::output::Context::from_flags(
                        true,
                        false,
                        None,
                        true,
                        crate::cli::AgentModeArg::Auto,
                    );
                    let token = vibe_core::cancellation::CancellationToken::new();
                    let result = if self.layers[self.selected].id == "user" {
                        crate::commands::user_override::set_user_enabled(
                            &quiet, &self.root, !enabled, &token,
                        )
                    } else {
                        crate::commands::user_override::run(&quiet, command, &token)
                    };
                    match result {
                        Ok(()) => {
                            self.layers = collect(&self.root);
                            self.message="Layer loading changed for the next session; current session instructions are unchanged".into();
                        }
                        Err(_) => {
                            self.message =
                                "Layer control refused; inspect `vibe tree --layers --plain`".into()
                        }
                    }
                }
            }
            _ => {}
        }
        Ok(Control::Changed)
    }
}
fn init(_: &mut Browser, _: &mut Global) -> Result<()> {
    Ok(())
}
fn render(area: Rect, buf: &mut Buffer, app: &mut Browser, _: &mut Global) -> Result<()> {
    let inner = Window::centered(
        area,
        buf,
        Line::from(" Layered tree "),
        area.width,
        area.height,
        &app.theme,
    );
    let text = app.lines().join("\n");
    Paragraph::new(text)
        .style(app.theme.text())
        .scroll((app.offset.min(u16::MAX as usize) as u16, 0))
        .render(inner, buf);
    Ok(())
}
fn event(event: &AppEvent, app: &mut Browser, _: &mut Global) -> Result<Control<AppEvent>> {
    match event {
        AppEvent::Event(event) => app.handle(event, true),
    }
}
fn error(error: anyhow::Error, app: &mut Browser, _: &mut Global) -> Result<Control<AppEvent>> {
    app.fatal = Some(error);
    Ok(Control::Quit)
}
pub(in crate::commands::tree) fn run(
    root: &Path,
    project: &PackageTree,
    layers: Vec<Layer>,
) -> Result<()> {
    let mut app = Browser::new(root, project, layers);
    let mut global = Global::default();
    run_tui(
        init,
        render,
        event,
        error,
        &mut global,
        &mut app,
        RunConfig::default()?.poll(PollCrossterm),
    )?;
    if let Some(error) = app.fatal {
        return Err(error);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui_crossterm::crossterm::event::{KeyEvent, KeyModifiers};
    fn key(code: KeyCode) -> Event {
        Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
    }
    #[test]
    fn hide_is_view_only_and_read_only_rendering_cannot_persist_controls() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(
            root.path().join("vibe.toml"),
            "[project]\nname='layer-ui'\nversion='1.0.0'\n",
        )
        .unwrap();
        let tree = crate::commands::tree::build::build_tree(root.path()).unwrap();
        let layers = vec![
            Layer {
                id: "project",
                enabled: true,
                state: "available",
                root: None,
                tree: None,
                diagnostics: vec![],
                relations: vec![],
            },
            Layer {
                id: "user",
                enabled: true,
                state: "available",
                root: None,
                tree: None,
                diagnostics: vec![],
                relations: vec![],
            },
            Layer {
                id: "override",
                enabled: false,
                state: "absent",
                root: None,
                tree: None,
                diagnostics: vec![],
                relations: vec![],
            },
        ];
        let mut app = Browser::new(root.path(), &tree, layers);
        let _ = app.handle(&key(KeyCode::Down), false).unwrap();
        let _ = app.handle(&key(KeyCode::Char(' ')), false).unwrap();
        assert!(app.hidden[1]);
        assert!(app.layers[1].enabled);
        let _ = app.handle(&key(KeyCode::Enter), false).unwrap();
        assert!(app.message.contains("read-only"));
        assert!(app.layers[1].enabled);
        let mut buf = Buffer::empty(Rect::new(0, 0, 120, 25));
        render(buf.area, &mut buf, &mut app, &mut Global::default()).unwrap();
        let text = crate::commands::tree::tui::snapshot::to_text(&buf);
        assert!(text.contains("user [hidden] enabled"));
        let _ = app.handle(&key(KeyCode::Down), false).unwrap();
        let _ = app.handle(&key(KeyCode::Enter), false).unwrap();
        assert!(app.message.contains("read-only"));
        assert!(!app.layers[2].enabled);
        assert!(!root.path().join("override-bindings.toml").exists());
    }
}
