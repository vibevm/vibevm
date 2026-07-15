//! The `vibe tree` Search Everywhere providers (PROP-037 §7.3, §13.5): packages
//! by name, every package-card field, and the `vibe.tree` action catalogue. Each
//! materialises **owned** candidates when the window opens (no borrow on the
//! tree), so the [`vibe_actions::search::SearchEngine`] can own them for the
//! window's lifetime. A provider's `on_selected` returns only `Close`/`Stay`; the
//! App applies the reveal / open-card / run effect by `(provider, item)`
//! (`super::apply_effect`), since only the App may mutate the model. The action
//! catalogue itself (and its live `vibe_actions::Registry`) lives in
//! [`super::catalogue`].

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-037#f1-search");

use vibe_actions::search::{
    Candidate, ItemRef, Modifiers, ProviderId, Query, SearchProvider, Selected,
};

use vibe_actions::{ActionAddr, Ctx, Registry};

use super::catalogue::{TreeCtx, key_for};
use crate::commands::tree::model::{
    Condition, DeclaredLink, LoadOrigin, LoadType, PackageTree, Source,
};

/// The three provider ids — the stable keys the App dispatches effects on.
pub const PACKAGES: ProviderId = "packages";
pub const FIELDS: ProviderId = "fields";
pub const ACTIONS: ProviderId = "actions";

// ---------------------------------------------------------------------------
// PackageProvider — packages by name (PROP-037 §7.3).
// ---------------------------------------------------------------------------

/// One pre-built package candidate: the tree index plus its matchable text.
struct PkgCand {
    idx: usize,
    id: String,
    load: &'static str,
    extra: Vec<String>,
}

/// Searches packages by their `group/name` id; navigate = reveal the package.
pub struct PackageProvider {
    cands: Vec<PkgCand>,
}

impl PackageProvider {
    /// Materialise one candidate per package from the analysed tree.
    pub fn build(tree: &PackageTree) -> Self {
        let cands = tree
            .packages
            .iter()
            .enumerate()
            .map(|(idx, p)| PkgCand {
                idx,
                id: p.id.clone(),
                load: load_label(p.load.load_type),
                extra: vec![
                    p.group.clone(),
                    p.name.clone(),
                    p.version.clone(),
                    p.kind.clone(),
                ],
            })
            .collect();
        Self { cands }
    }
}

impl SearchProvider for PackageProvider {
    fn id(&self) -> ProviderId {
        PACKAGES
    }
    fn group_name(&self) -> &str {
        "Packages"
    }
    fn sort_weight(&self) -> i32 {
        100
    }
    fn candidates(&self, _query: &Query) -> Vec<Candidate> {
        self.cands
            .iter()
            .map(|c| Candidate {
                item: ItemRef(c.idx),
                primary: c.id.clone(),
                secondary: Some(c.load.to_string()),
                extra_haystacks: c.extra.clone(),
                enabled: true,
            })
            .collect()
    }
    fn on_selected(&self, _item: ItemRef, _mods: Modifiers) -> Selected {
        Selected::Close
    }
}

// ---------------------------------------------------------------------------
// FieldProvider — inside every field of the package detail cards (PROP-037 §7.3).
// ---------------------------------------------------------------------------

/// One card-field candidate: which package it belongs to plus the field's
/// name and value.
struct FieldCand {
    pkg_idx: usize,
    field: &'static str,
    value: String,
    owner: String,
}

/// Searches inside every detail-card field of every package; navigate = open
/// that package's card. The item is the *package* index (the card shows all its
/// fields).
pub struct FieldProvider {
    cands: Vec<FieldCand>,
}

impl FieldProvider {
    /// Materialise one candidate per (package, non-empty field).
    pub fn build(tree: &PackageTree) -> Self {
        let mut cands = Vec::new();
        for (pkg_idx, p) in tree.packages.iter().enumerate() {
            let mut push = |field: &'static str, value: String| {
                if !value.is_empty() {
                    cands.push(FieldCand {
                        pkg_idx,
                        field,
                        value,
                        owner: p.id.clone(),
                    });
                }
            };
            push("version", p.version.clone());
            push("kind", p.kind.clone());
            push("load", load_label(p.load.load_type).to_string());
            push("origin", origin_label(p.load.origin).to_string());
            if let Some(d) = p.load.declared {
                push("declared", declared_label(d).to_string());
            }
            if let Some(raw) = condition_value(&p.condition) {
                push("condition", raw);
            }
            if let Some(url) = source_value(p.source.as_ref()) {
                push("source", url);
            }
            if let Some(hash) = p.content_hash.clone() {
                push("hash", hash);
            }
            if let Some(boot) = p.load.boot_path.clone() {
                push("boot path", boot);
            }
            for dep in &p.dependencies {
                push("dependency", dep.clone());
            }
        }
        Self { cands }
    }
}

impl SearchProvider for FieldProvider {
    fn id(&self) -> ProviderId {
        FIELDS
    }
    fn group_name(&self) -> &str {
        "Card fields"
    }
    fn sort_weight(&self) -> i32 {
        200
    }
    fn candidates(&self, _query: &Query) -> Vec<Candidate> {
        self.cands
            .iter()
            .map(|c| Candidate {
                item: ItemRef(c.pkg_idx),
                primary: c.value.clone(),
                secondary: Some(format!("{} · {}", c.field, c.owner)),
                extra_haystacks: vec![c.field.to_string(), c.owner.clone()],
                enabled: true,
            })
            .collect()
    }
    fn on_selected(&self, _item: ItemRef, _mods: Modifiers) -> Selected {
        Selected::Close
    }
}

// ---------------------------------------------------------------------------
// ActionProvider — the vibe.tree action catalogue (PROP-037 §13.5). The
// catalogue + its live Registry are in `super::catalogue`.
// ---------------------------------------------------------------------------

/// One materialised action candidate — resolved from a real `vibe_actions`
/// [`vibe_actions::Action`] at open: presentation, synonyms, and the enablement
/// verdict (with its "why disabled" reason).
struct ActionCand {
    name: String,
    desc: String,
    key: String,
    synonyms: Vec<String>,
    enabled: bool,
    reason: Option<String>,
}

/// Searches the `vibe.tree` action [`Registry`] by name, description, address,
/// and synonyms; selecting runs the action in place. Enablement (and its "why
/// disabled" reason) comes from each real Action's predicate over the
/// [`TreeCtx`] snapshot (PROP-039 §6.2).
pub struct ActionProvider {
    cands: Vec<ActionCand>,
}

impl ActionProvider {
    /// Enumerate the registry, resolving each action's presentation + enablement
    /// over `ctx`. Returns the provider and the parallel address list — in the
    /// same order, so an `ItemRef` indexes both — the App dispatches by address.
    pub fn build(registry: &Registry, ctx: TreeCtx) -> (Self, Vec<ActionAddr>) {
        let vctx = Ctx::new().with(ctx);
        let mut cands = Vec::new();
        let mut addrs = Vec::new();
        for action in registry.iter() {
            let en = action.evaluate(&vctx);
            let addr = action.addr().clone();
            cands.push(ActionCand {
                name: action.presentation().name().default_en().to_string(),
                desc: action.presentation().description().default_en().to_string(),
                key: key_for(&addr.to_string()).to_string(),
                synonyms: action.search_meta().synonyms().to_vec(),
                enabled: en.enabled,
                reason: en.reason.map(|r| r.as_str().to_string()),
            });
            addrs.push(addr);
        }
        (Self { cands }, addrs)
    }
}

impl SearchProvider for ActionProvider {
    fn id(&self) -> ProviderId {
        ACTIONS
    }
    fn group_name(&self) -> &str {
        "Actions"
    }
    fn sort_weight(&self) -> i32 {
        300
    }
    fn candidates(&self, _query: &Query) -> Vec<Candidate> {
        self.cands
            .iter()
            .enumerate()
            .map(|(idx, c)| {
                let mut extra = vec![c.desc.clone()];
                extra.extend(c.synonyms.clone());
                // A disabled action surfaces its "why disabled" reason; an enabled
                // one shows its keybinding.
                let secondary = match (c.enabled, &c.reason) {
                    (false, Some(reason)) => Some(reason.clone()),
                    _ => Some(c.key.clone()),
                };
                Candidate {
                    item: ItemRef(idx),
                    primary: c.name.clone(),
                    secondary,
                    extra_haystacks: extra,
                    enabled: c.enabled,
                }
            })
            .collect()
    }
    fn on_selected(&self, _item: ItemRef, _mods: Modifiers) -> Selected {
        Selected::Close
    }
}

// ---------------------------------------------------------------------------
// Small field-label helpers (self-contained so no `modal`/`state` visibility
// is needed).
// ---------------------------------------------------------------------------

fn load_label(t: LoadType) -> &'static str {
    match t {
        LoadType::Static => "static",
        LoadType::Dynamic => "dynamic",
        LoadType::None => "none",
    }
}

fn origin_label(o: LoadOrigin) -> &'static str {
    match o {
        LoadOrigin::Declared => "declared",
        LoadOrigin::Suggested => "suggested",
        LoadOrigin::Default => "default",
        LoadOrigin::StaticTransitive => "static-transitive",
        LoadOrigin::WhenForced => "when-forced",
        LoadOrigin::None => "none",
    }
}

fn declared_label(d: DeclaredLink) -> &'static str {
    match d {
        DeclaredLink::Static => "static",
        DeclaredLink::Dynamic => "dynamic",
        DeclaredLink::StaticTransitive => "static-transitive",
        DeclaredLink::StaticHard => "static-hard",
    }
}

fn condition_value(c: &Condition) -> Option<String> {
    if !c.present {
        return None;
    }
    c.raw.clone().filter(|s| !s.is_empty())
}

fn source_value(source: Option<&Source>) -> Option<String> {
    source.and_then(|s| s.url.clone()).filter(|s| !s.is_empty())
}
