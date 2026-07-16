//! Parse a `--send` key script into synthetic crossterm key-press events for the
//! AIUI render plane (PROP-042 §3 `#key-script`). Names are case-insensitive; a
//! `Shift+` prefix is honoured; unknown or side-effecting keys (`F4`/`F6`) are a
//! hard error naming the offending token — never a silent skip.

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-042#key-script");

use anyhow::{Result, bail};
use ratatui_crossterm::crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use specmark::spec;

/// Parse a space-separated key script into synthetic key-press events
/// (PROP-042 §3). An empty script yields no events.
#[spec(implements = "spec://vibevm/modules/vibe-cli/PROP-042#key-script")]
pub(crate) fn parse(script: &str) -> Result<Vec<Event>> {
    script.split_whitespace().map(parse_one).collect()
}

/// Parse one token (e.g. `F2`, `Down`, `Shift+Left`) into a key-press event.
fn parse_one(tok: &str) -> Result<Event> {
    let (mut mods, name) = match tok.split_once('+') {
        Some((m, n)) if m.eq_ignore_ascii_case("shift") => (KeyModifiers::SHIFT, n),
        Some(_) => bail!("unknown modifier in key `{tok}` (only `Shift+` is supported)"),
        None => (KeyModifiers::NONE, tok),
    };
    let mut code = match name.to_ascii_lowercase().as_str() {
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "enter" => KeyCode::Enter,
        "esc" | "escape" => KeyCode::Esc,
        "tab" => KeyCode::Tab,
        "backtab" => KeyCode::BackTab,
        "space" => KeyCode::Char(' '),
        "backspace" => KeyCode::Backspace,
        // F4 spawns the settings subprocess, F6/Shift+F6 write the clipboard —
        // refused in the render plane (PROP-042 §1).
        "f4" | "f6" => {
            bail!("key `{tok}` has side effects and is refused in the render plane (PROP-042 §1)")
        }
        f if f.starts_with('f') && f.len() > 1 => {
            let n: u8 = f[1..]
                .parse()
                .map_err(|_| anyhow::anyhow!("unknown key `{tok}`"))?;
            if !(1..=12).contains(&n) {
                bail!("unknown function key `{tok}` (F1–F12)");
            }
            KeyCode::F(n)
        }
        _ => bail!("unknown key `{tok}`"),
    };
    // Crossterm emits Shift+Tab as the dedicated BackTab code, and the TUI's
    // handlers match on BackTab — normalise so `Shift+Tab` drives focus backward.
    if code == KeyCode::Tab && mods.contains(KeyModifiers::SHIFT) {
        code = KeyCode::BackTab;
        mods = KeyModifiers::NONE;
    }
    Ok(Event::Key(KeyEvent::new(code, mods)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_names_and_shift() {
        let evs = parse("F2 Down Enter Shift+Left Shift+Tab").expect("ok");
        assert_eq!(evs.len(), 5);
        assert_eq!(
            evs[0],
            Event::Key(KeyEvent::new(KeyCode::F(2), KeyModifiers::NONE))
        );
        assert_eq!(
            evs[3],
            Event::Key(KeyEvent::new(KeyCode::Left, KeyModifiers::SHIFT))
        );
        // Shift+Tab normalises to BackTab (no modifier).
        assert_eq!(
            evs[4],
            Event::Key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::NONE))
        );
    }

    #[test]
    fn empty_script_is_no_events() {
        assert!(parse("").expect("ok").is_empty());
        assert!(parse("   ").expect("ok").is_empty());
    }

    #[test]
    fn refuses_side_effecting_and_unknown_keys() {
        assert!(parse("F4").is_err(), "F4 refused");
        assert!(parse("F6").is_err(), "F6 refused");
        assert!(parse("Shift+F6").is_err(), "Shift+F6 refused");
        assert!(parse("Nope").is_err(), "unknown refused");
        assert!(parse("F13").is_err(), "out-of-range F-key refused");
    }
}
