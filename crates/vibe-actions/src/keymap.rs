//! The keymap resolver (PROP-039 §9) — a **frontend-agnostic** binding map and
//! the pure 3-state resolver a surface adapter drives.
//!
//! The resolver owns the *logic* of "which action does this key sequence mean":
//! it is a pure function over a `&[Key]` prefix and an enablement decision,
//! returning [`Match::NoMatch`], [`Match::NeedMoreChords`], or
//! [`Match::Found`]. Everything terminal-specific — chord timers, IME, focus
//! walking, the `crossterm::event::KeyEvent → Key` conversion — lives in the
//! surface adapter, never here (PROP-039 §9.2). That is what keeps this module
//! free of `crossterm`/`ratatui` and the crate's `#no-render-dep` invariant
//! intact (PROP-039 §1).
//!
//! # Model
//!
//! - A [`Key`] is an abstract key event: a [`KeyCode`] plus a [`KeyModifiers`]
//!   bitset. It is *not* a `crossterm` type — a surface converts its native
//!   event into one.
//! - A [`Binding`] ties a *chord* (a `Vec<Key>`, length ≥ 1) to an
//!   `(`[`ActionAddr`]`, `[`ParamValues`]`)` pair, plus a `weight` used only to
//!   break an exact-key tie.
//! - A [`Keymap`] holds the bindings; [`Keymap::resolve`] is the resolver.
//!
//! Spec: [PROP-039 §9](../../../../spec/modules/vibe-actions/PROP-039-action-system.md#keymap).

specmark::scope!("spec://vibevm/modules/vibe-actions/PROP-039#keymap");

use crate::address::ActionAddr;
use crate::params::ParamValues;

/// A modifier-key bitset, abstracted away from any terminal toolkit.
///
/// Compose with the `with_*` builders; test with [`Self::contains`]. The bit
/// layout is an internal implementation detail — use the named constants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct KeyModifiers(u8);

impl KeyModifiers {
    /// No modifiers held.
    pub const NONE: Self = Self(0);
    /// A `Shift` modifier.
    pub const SHIFT: Self = Self(1 << 0);
    /// A `Control` modifier.
    pub const CTRL: Self = Self(1 << 1);
    /// An `Alt` / `Option` / `Meta` modifier.
    pub const ALT: Self = Self(1 << 2);

    /// The raw bitset (for serialization / debugging surfaces).
    pub const fn bits(self) -> u8 {
        self.0
    }

    /// Whether `self` carries every bit `other` does.
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Set the `Shift` bit.
    #[must_use]
    pub const fn with_shift(self) -> Self {
        Self(self.0 | Self::SHIFT.0)
    }

    /// Set the `Control` bit.
    #[must_use]
    pub const fn with_ctrl(self) -> Self {
        Self(self.0 | Self::CTRL.0)
    }

    /// Set the `Alt` bit.
    #[must_use]
    pub const fn with_alt(self) -> Self {
        Self(self.0 | Self::ALT.0)
    }
}

/// The non-modifier half of a [`Key`] — the set of physical keys a vibevm surface
/// cares about. Closed on purpose; a new key is a deliberate addition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
    /// A printable character. The surface applies its layout/IME result here.
    Char(char),
    /// `Space`.
    Space,
    /// `Enter` / `Return`.
    Enter,
    /// `Tab`.
    Tab,
    /// `Shift+Tab` (surfaced as its own code so a keymap can bind it directly).
    BackTab,
    /// `Escape`.
    Esc,
    /// `Backspace`.
    Backspace,
    /// `Delete`.
    Delete,
    /// `Insert`.
    Insert,
    /// `Home`.
    Home,
    /// `End`.
    End,
    /// `PageUp`.
    PageUp,
    /// `PageDown`.
    PageDown,
    /// `↑`.
    Up,
    /// `↓`.
    Down,
    /// `←`.
    Left,
    /// `→`.
    Right,
    /// A function key `F1`..`F12` (the argument is 1-indexed).
    F(u8),
}

/// An abstract key event — a [`KeyCode`] plus the held [`KeyModifiers`].
///
/// ```
/// use vibe_actions::keymap::{Key, KeyCode, KeyModifiers};
///
/// let plain_f1 = Key::new(KeyCode::F(1));
/// let shift_f6 = Key::new(KeyCode::F(6)).with_mods(KeyModifiers::SHIFT);
/// assert!(!plain_f1.mods.contains(KeyModifiers::SHIFT));
/// assert!(shift_f6.mods.contains(KeyModifiers::SHIFT));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Key {
    /// The physical key.
    pub code: KeyCode,
    /// The modifiers held alongside it.
    pub mods: KeyModifiers,
}

impl Key {
    /// A key with no modifiers.
    pub const fn new(code: KeyCode) -> Self {
        Self {
            code,
            mods: KeyModifiers::NONE,
        }
    }

    /// A plain printable character with no modifiers.
    pub const fn char(c: char) -> Self {
        Self::new(KeyCode::Char(c))
    }

    /// Attach a modifier set.
    #[must_use]
    pub const fn with_mods(self, mods: KeyModifiers) -> Self {
        Self { mods, ..self }
    }
}

/// The 3-state result of resolving a key sequence (PROP-039 §9.2).
///
/// Modeled on VSCode's `KeybindingResolverResult`: a sequence either matches a
/// binding exactly ([`Match::Found`]), is a prefix of one or more longer chords
/// ([`Match::NeedMoreChords`]), or matches nothing ([`Match::NoMatch`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Match {
    /// No enabled binding matches the sequence or extends it.
    NoMatch,
    /// The sequence is a strict prefix of at least one enabled chord — the
    /// surface should keep the partial sequence and wait for the next key
    /// (its own timer governs the wait; PROP-039 §9.2).
    NeedMoreChords,
    /// The sequence matches an enabled binding exactly. Carries the resolved
    /// address and its parameter values (cloned from the binding).
    Found(ActionAddr, ParamValues),
}

/// One keymap entry: a chord, the action it invokes, its parameters, and a
/// tie-breaking weight.
///
/// `weight` is consulted **only** when two enabled bindings share the exact same
/// chord — the higher weight wins, and the loser is reported by
/// [`Keymap::conflicts`] (PROP-039 §9.3). Default weight is 0.
#[derive(Debug, Clone)]
pub struct Binding {
    /// The key sequence (length ≥ 1) that triggers this binding.
    pub chord: Vec<Key>,
    /// The invoked action's address.
    pub addr: ActionAddr,
    /// The invocation parameters.
    pub params: ParamValues,
    /// Higher wins on an exact-chord tie.
    pub weight: u32,
}

/// A chord two or more **enabled** bindings compete for (PROP-039 §9.3). The
/// resolver resolves it by weight; this struct surfaces it so a surface can
/// report the ambiguity rather than swallowing it silently.
#[derive(Debug, Clone)]
pub struct Conflict {
    /// The contested chord.
    pub chord: Vec<Key>,
    /// Indices into [`Keymap::bindings`] of the competing enabled bindings.
    pub bindings: Vec<usize>,
}

/// A keymap: an ordered collection of [`Binding`]s plus the pure resolver.
///
/// ```
/// use vibe_actions::keymap::{Key, KeyCode, Keymap, Match};
/// use vibe_actions::ActionAddr;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let quit: ActionAddr = "action://vibe.tree/quit".parse()?;
/// let copy: ActionAddr = "action://vibe.tree/copy".parse()?;
/// let mut km = Keymap::new();
/// km.bind([Key::new(KeyCode::Esc)], quit.clone(), Default::default(), 0);
/// km.bind([Key::char('c'), Key::char('c')], copy.clone(), Default::default(), 0);
///
/// fn yes(_: &ActionAddr) -> bool { true }
///
/// // exact single-key match
/// assert_eq!(km.resolve(&[Key::new(KeyCode::Esc)], yes),
///            Match::Found(quit, Default::default()));
///
/// // prefix of a two-key chord
/// assert_eq!(km.resolve(&[Key::char('c')], yes), Match::NeedMoreChords);
///
/// // nothing
/// assert_eq!(km.resolve(&[Key::char('z')], yes), Match::NoMatch);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Default)]
pub struct Keymap {
    bindings: Vec<Binding>,
}

impl Keymap {
    /// An empty keymap.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a binding. The chord is taken in any iterable form (array, `Vec`).
    /// An empty chord is rejected — a binding must carry at least one key.
    pub fn bind(
        &mut self,
        chord: impl IntoIterator<Item = Key>,
        addr: ActionAddr,
        params: ParamValues,
        weight: u32,
    ) -> &mut Self {
        let chord: Vec<Key> = chord.into_iter().collect();
        if chord.is_empty() {
            return self;
        }
        self.bindings.push(Binding {
            chord,
            addr,
            params,
            weight,
        });
        self
    }

    /// All bindings, in insertion order.
    pub fn bindings(&self) -> &[Binding] {
        &self.bindings
    }

    /// Resolve a pressed key sequence (PROP-039 §9.2).
    ///
    /// `enabled` decides whether a candidate binding is live in the current
    /// context — the resolver stays pure by taking that decision as a function
    /// rather than reaching into a typed `Ctx`. The rule:
    ///
    /// 1. among **enabled** bindings whose chord equals `seq` exactly, the
    ///    highest-`weight` one wins → [`Match::Found`] (ties go to the earliest);
    /// 2. else if any **enabled** binding has `seq` as a strict prefix →
    ///    [`Match::NeedMoreChords`];
    /// 3. else → [`Match::NoMatch`].
    pub fn resolve(&self, seq: &[Key], enabled: impl Fn(&ActionAddr) -> bool) -> Match {
        // (1) exact enabled matches — highest weight, earliest on tie.
        let mut best: Option<&Binding> = None;
        for b in &self.bindings {
            if b.chord.len() == seq.len() && b.chord == seq && enabled(&b.addr) {
                match best {
                    None => best = Some(b),
                    Some(cur) if b.weight > cur.weight => best = Some(b),
                    _ => {}
                }
            }
        }
        if let Some(b) = best {
            return Match::Found(b.addr.clone(), b.params.clone());
        }

        // (2) a strict prefix of some enabled longer chord.
        let deeper = self
            .bindings
            .iter()
            .any(|b| b.chord.len() > seq.len() && enabled(&b.addr) && b.chord.starts_with(seq));
        if deeper {
            Match::NeedMoreChords
        } else {
            // (3) nothing.
            Match::NoMatch
        }
    }

    /// Surface every chord that two or more **enabled** bindings contest
    /// (PROP-039 §9.3). The resolver breaks the tie by weight; this is the
    /// introspection API so a surface can report the ambiguity instead of
    /// hiding it.
    pub fn conflicts(&self, enabled: impl Fn(&ActionAddr) -> bool) -> Vec<Conflict> {
        let live: Vec<(usize, &Binding)> = self
            .bindings
            .iter()
            .enumerate()
            .filter(|(_, b)| enabled(&b.addr))
            .collect();
        let mut out: Vec<Conflict> = Vec::new();
        for (i, &(ia, a)) in live.iter().enumerate() {
            for &(ib, b) in live.iter().skip(i + 1) {
                if a.chord == b.chord {
                    match out.iter_mut().find(|c| c.chord == a.chord) {
                        Some(existing) => {
                            if !existing.bindings.contains(&ib) {
                                existing.bindings.push(ib);
                            }
                        }
                        None => out.push(Conflict {
                            chord: a.chord.clone(),
                            bindings: vec![ia, ib],
                        }),
                    }
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    fn addr(s: &str) -> ActionAddr {
        s.parse().unwrap()
    }

    fn always(_: &ActionAddr) -> bool {
        true
    }

    fn never(_: &ActionAddr) -> bool {
        false
    }

    #[test]
    fn exact_single_key_match() {
        let mut km = Keymap::new();
        km.bind(
            [Key::new(KeyCode::Esc)],
            addr("action://vibe.tree/quit"),
            ParamValues::new(),
            0,
        );
        assert_eq!(
            km.resolve(&[Key::new(KeyCode::Esc)], always),
            Match::Found(addr("action://vibe.tree/quit"), ParamValues::new())
        );
    }

    #[test]
    fn chord_prefix_needs_more() {
        let mut km = Keymap::new();
        km.bind(
            [Key::char('g'), Key::char('g')],
            addr("action://vibe.tree/top"),
            ParamValues::new(),
            0,
        );
        assert_eq!(km.resolve(&[Key::char('g')], always), Match::NeedMoreChords);
        assert_eq!(
            km.resolve(&[Key::char('g'), Key::char('g')], always),
            Match::Found(addr("action://vibe.tree/top"), ParamValues::new())
        );
    }

    #[test]
    fn no_match_for_unknown() {
        let km = Keymap::new();
        assert_eq!(km.resolve(&[Key::char('z')], always), Match::NoMatch);
    }

    #[test]
    fn disabled_binding_is_invisible_and_no_prefix() {
        let mut km = Keymap::new();
        // A two-key chord whose first key is unique to the disabled binding.
        km.bind(
            [Key::char('q'), Key::char('q')],
            addr("action://vibe.tree/quit"),
            ParamValues::new(),
            0,
        );
        // Everything disabled: 'q' alone is NoMatch (not NeedMoreChords).
        assert_eq!(km.resolve(&[Key::char('q')], never), Match::NoMatch);
        assert_eq!(
            km.resolve(&[Key::char('q'), Key::char('q')], never),
            Match::NoMatch
        );
    }

    #[test]
    fn weight_breaks_an_exact_tie() {
        let mut km = Keymap::new();
        km.bind(
            [Key::new(KeyCode::F(1))],
            addr("action://vibe.tree/help"),
            ParamValues::new(),
            1,
        );
        km.bind(
            [Key::new(KeyCode::F(1))],
            addr("action://vibe.tree/search"),
            ParamValues::new(),
            5,
        );
        assert_eq!(
            km.resolve(&[Key::new(KeyCode::F(1))], always),
            Match::Found(addr("action://vibe.tree/search"), ParamValues::new())
        );
    }

    #[test]
    fn conflicts_surfaced_for_same_chord() {
        let mut km = Keymap::new();
        km.bind(
            [Key::new(KeyCode::F(2))],
            addr("action://vibe.tree/a"),
            ParamValues::new(),
            0,
        );
        km.bind(
            [Key::new(KeyCode::F(2))],
            addr("action://vibe.tree/b"),
            ParamValues::new(),
            0,
        );
        let cs = km.conflicts(always);
        assert_eq!(cs.len(), 1);
        assert_eq!(cs[0].chord, vec![Key::new(KeyCode::F(2))]);
        assert_eq!(cs[0].bindings.len(), 2);
    }

    #[test]
    fn empty_chord_binding_is_dropped() {
        let mut km = Keymap::new();
        km.bind([], addr("action://vibe.tree/x"), ParamValues::new(), 0);
        assert!(km.bindings().is_empty());
    }

    #[test]
    fn modifiers_compose_and_test() {
        let m = KeyModifiers::NONE.with_shift().with_ctrl();
        assert!(m.contains(KeyModifiers::SHIFT));
        assert!(m.contains(KeyModifiers::CTRL));
        assert!(!m.contains(KeyModifiers::ALT));
        assert_eq!(
            m.with_alt().bits(),
            KeyModifiers::SHIFT.bits() | KeyModifiers::CTRL.bits() | KeyModifiers::ALT.bits()
        );
    }

    #[test]
    fn shift_f6_roundtrips_through_key() {
        let k = Key::new(KeyCode::F(6)).with_mods(KeyModifiers::SHIFT);
        let again = Key {
            code: KeyCode::F(6),
            mods: KeyModifiers::SHIFT,
        };
        assert_eq!(k, again);
    }
}
