//! Checkbox selection shared by global MCP agent and server prompts.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-027#registration");

use anyhow::{Result, bail};
use console::{Key, Term};

use crate::output::sanitize_progress_text;

#[derive(Debug, thiserror::Error)]
pub(crate) enum SelectionError {
    #[error(
        "violates REQ spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-027#REG-GLOBAL-AGENT-SELECTION: selection cancelled; no registrations changed; fix surface: retry selection or pass explicit --agent/--server flags"
    )]
    Cancelled,
    #[error(
        "violates REQ spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-027#REG-GLOBAL-AGENT-SELECTION: nothing selected; fix surface: {0}"
    )]
    Empty(String),
}

#[derive(Clone, Copy)]
pub(crate) enum InitialSelection<'a> {
    All,
    Items(&'a [usize]),
}

struct Selection {
    all_selected: bool,
    selected: Vec<bool>,
    defaults: Vec<usize>,
    cursor: usize,
}

impl Selection {
    #[cfg(test)]
    fn new(count: usize, defaults: &[usize]) -> Result<Self> {
        Self::with_initial(count, defaults, InitialSelection::All)
    }

    fn with_initial(
        count: usize,
        defaults: &[usize],
        initial: InitialSelection<'_>,
    ) -> Result<Self> {
        let items = match initial {
            InitialSelection::All => &[][..],
            InitialSelection::Items(items) => items,
        };
        if defaults.iter().chain(items).any(|index| *index >= count) {
            bail!("selection defaults refer to a missing choice");
        }
        Ok(Self {
            all_selected: matches!(initial, InitialSelection::All),
            selected: (0..count).map(|index| items.contains(&index)).collect(),
            defaults: defaults.to_vec(),
            cursor: 0,
        })
    }

    fn group_checked(&self) -> bool {
        self.all_selected
    }

    fn toggle(&mut self) {
        if self.cursor == 0 {
            self.all_selected = !self.all_selected;
            self.selected.fill(false);
        } else if let Some(selected) = self.selected.get_mut(self.cursor - 1) {
            self.all_selected = false;
            *selected = !*selected;
        }
    }

    fn move_by(&mut self, down: bool) {
        if down {
            self.cursor = (self.cursor + 1).min(self.selected.len());
        } else {
            self.cursor = self.cursor.saturating_sub(1);
        }
    }

    fn indices(&self, empty_hint: &str) -> Result<Vec<usize>> {
        let indices: Vec<_> = self
            .selected
            .iter()
            .enumerate()
            .filter_map(|(index, selected)| {
                let checked = if self.all_selected {
                    self.defaults.contains(&index)
                } else {
                    *selected
                };
                checked.then_some(index)
            })
            .collect();
        if indices.is_empty() {
            return Err(SelectionError::Empty(empty_hint.to_owned()).into());
        }
        Ok(indices)
    }

    fn key(&mut self, key: Key, empty_hint: &str) -> Result<Option<Vec<usize>>> {
        match key {
            Key::ArrowUp => self.move_by(false),
            Key::ArrowDown => self.move_by(true),
            Key::Char(' ') => self.toggle(),
            Key::Enter => return self.indices(empty_hint).map(Some),
            Key::Escape | Key::CtrlC | Key::Char('\u{4}') | Key::Unknown => {
                return Err(SelectionError::Cancelled.into());
            }
            _ => {}
        }
        Ok(None)
    }

    fn viewport(&self, height: usize) -> std::ops::Range<usize> {
        let rows = self.selected.len() + 1;
        let height = height.max(1).min(rows);
        let start = self.cursor.saturating_sub(height - 1).min(rows - height);
        start..start + height
    }
}

/// Call while the invocation's progress renderer is suspended.
/// Labels and defaults are caller-owned; results are stable original indices.
pub(crate) fn choose(
    title: &str,
    group_label: &str,
    labels: &[String],
    defaults: &[usize],
    empty_hint: &str,
) -> Result<Vec<usize>> {
    choose_with_initial(
        title,
        group_label,
        labels,
        defaults,
        InitialSelection::All,
        empty_hint,
    )
}

pub(crate) fn choose_with_initial(
    title: &str,
    group_label: &str,
    labels: &[String],
    defaults: &[usize],
    initial: InitialSelection<'_>,
    empty_hint: &str,
) -> Result<Vec<usize>> {
    let terminal = Term::stderr();
    if !terminal.is_term() {
        bail!("checkbox selection requires an interactive terminal; {empty_hint}");
    }
    let mut selection = Selection::with_initial(labels.len(), defaults, initial)?;
    let mut rendered = 0;
    loop {
        terminal.clear_last_lines(rendered)?;
        let (height, width) = terminal.size();
        if height < 4 || width < 8 {
            bail!("terminal is too small for checkbox selection; enlarge it or use explicit flags");
        }
        let viewport = selection.viewport(usize::from(height).saturating_sub(3));
        let line = |text: &str| -> std::io::Result<()> {
            let sanitized = sanitize_progress_text(text);
            terminal.write_line(&console::truncate_str(
                &sanitized,
                usize::from(width).saturating_sub(1).max(1),
                "…",
            ))
        };
        line(title)?;
        line("Up/Down move · Space toggles · Enter confirms · Esc cancels")?;
        for row in viewport.clone() {
            let checked = if row == 0 {
                selection.group_checked()
            } else {
                selection.selected[row - 1]
            };
            let label = if row == 0 {
                group_label
            } else {
                &labels[row - 1]
            };
            line(&row_text(label, selection.cursor == row, checked))?;
        }
        line(&format!(
            "Rows {}–{} of {}",
            viewport.start + 1,
            viewport.end,
            labels.len() + 1
        ))?;
        rendered = viewport.len() + 3;
        let key = match terminal.read_key() {
            Ok(key) => key,
            Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => {
                return Err(SelectionError::Cancelled.into());
            }
            Err(error) => return Err(error.into()),
        };
        if let Some(indices) = selection.key(key, empty_hint)? {
            return Ok(indices);
        }
    }
}

fn row_text(label: &str, active: bool, checked: bool) -> String {
    format!(
        "{} [{}] {}",
        if active { ">" } else { " " },
        if checked { "+" } else { " " },
        label
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[specmark::verifies(
        "spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-027#REG-GLOBAL-UNINSTALL-DEFAULTS"
    )]
    fn checkbox_initial_items_leave_all_off_and_keep_group_targets_separate() {
        let mut selection =
            Selection::with_initial(4, &[0, 1, 2, 3], InitialSelection::Items(&[2])).unwrap();
        assert!(!selection.group_checked());
        assert_eq!(selection.selected, [false, false, true, false]);
        assert_eq!(selection.indices("choose").unwrap(), [2]);
        selection.toggle();
        assert!(selection.group_checked());
        assert_eq!(selection.selected, [false; 4]);
        assert_eq!(selection.indices("choose").unwrap(), [0, 1, 2, 3]);
        selection.cursor = 2;
        selection.toggle();
        assert!(!selection.group_checked());
        assert_eq!(selection.indices("choose").unwrap(), [1]);
        let empty = Selection::with_initial(4, &[0, 1], InitialSelection::Items(&[])).unwrap();
        assert!(!empty.group_checked());
        assert!(empty.indices("choose").is_err());
        assert!(Selection::with_initial(1, &[0], InitialSelection::Items(&[1])).is_err());
    }

    #[test]
    fn checkbox_defaults_check_only_all_and_return_sparse_defaults_in_source_order() {
        let selection = Selection::new(4, &[2, 0, 2]).unwrap();
        assert!(selection.all_selected);
        assert_eq!(selection.selected, [false; 4]);
        assert_eq!(selection.indices("choose an agent").unwrap(), [0, 2]);
    }

    #[test]
    fn checkbox_individual_subset_turns_all_off_and_all_clears_every_individual() {
        let mut selection = Selection::new(4, &[0, 2]).unwrap();
        selection.cursor = 1;
        selection.toggle();
        assert!(!selection.all_selected);
        assert_eq!(selection.indices("choose an agent").unwrap(), [0]);
        selection.cursor = 2;
        selection.toggle();
        assert_eq!(selection.indices("choose an agent").unwrap(), [0, 1]);
        selection.cursor = 0;
        selection.toggle();
        assert!(selection.all_selected);
        assert_eq!(selection.selected, [false; 4]);
        assert_eq!(selection.indices("choose an agent").unwrap(), [0, 2]);
    }

    #[test]
    fn checkbox_all_off_leaves_empty_and_back_on_restores_defaults() {
        let mut selection = Selection::new(3, &[0, 2]).unwrap();
        selection.toggle();
        assert!(!selection.all_selected);
        assert_eq!(selection.selected, [false; 3]);
        assert!(selection.indices("choose an agent").is_err());
        selection.toggle();
        assert!(selection.all_selected);
        assert_eq!(selection.indices("choose an agent").unwrap(), [0, 2]);
    }

    #[test]
    fn checkbox_last_individual_off_does_not_reenable_all() {
        let mut selection = Selection::new(3, &[0, 2]).unwrap();
        selection.cursor = 2;
        selection.toggle();
        selection.toggle();
        assert!(!selection.all_selected);
        assert_eq!(selection.selected, [false; 3]);
        assert!(selection.indices("choose an agent").is_err());
    }

    #[test]
    fn checkbox_selecting_every_individual_never_checks_all() {
        let mut selection = Selection::new(3, &[0, 2]).unwrap();
        for cursor in 1..=3 {
            selection.cursor = cursor;
            selection.toggle();
        }
        assert!(!selection.all_selected);
        assert_eq!(selection.selected, [true; 3]);
        assert_eq!(selection.indices("choose an agent").unwrap(), [0, 1, 2]);
    }

    #[test]
    fn checkbox_rows_render_plus_for_checked_and_space_for_unchecked() {
        assert_eq!(row_text("All", true, true), "> [+] All");
        assert_eq!(row_text("codex", false, false), "  [ ] codex");
    }

    #[test]
    fn checkbox_navigation_and_viewport_stay_bounded() {
        let mut selection = Selection::new(100, &[]).unwrap();
        selection.move_by(false);
        assert_eq!(selection.cursor, 0);
        for _ in 0..110 {
            selection.move_by(true);
        }
        assert_eq!(selection.cursor, 100);
        assert_eq!(selection.viewport(5), 96..101);
        assert_eq!(selection.viewport(0), 100..101);
    }

    #[test]
    fn checkbox_empty_and_invalid_defaults_are_typed_refusals() {
        let selection = Selection::new(2, &[]).unwrap();
        let error = selection.indices("pass --agent <name>").unwrap_err();
        assert!(matches!(
            error.downcast_ref::<SelectionError>(),
            Some(SelectionError::Empty(_))
        ));
        assert!(error.to_string().contains("--agent"));
        assert!(Selection::new(1, &[1]).is_err());
    }

    #[test]
    fn checkbox_enter_confirms_and_escape_or_eof_cancel() {
        let mut selection = Selection::new(3, &[0, 2]).unwrap();
        assert_eq!(
            selection.key(Key::Enter, "choose").unwrap(),
            Some(vec![0, 2])
        );
        for key in [Key::Escape, Key::CtrlC, Key::Char('\u{4}'), Key::Unknown] {
            assert!(matches!(
                selection
                    .key(key, "choose")
                    .unwrap_err()
                    .downcast_ref::<SelectionError>(),
                Some(SelectionError::Cancelled)
            ));
        }
        let mut empty = Selection::new(3, &[]).unwrap();
        assert!(
            empty
                .key(Key::Enter, "select with Space")
                .unwrap_err()
                .to_string()
                .contains("Space")
        );
    }
}
