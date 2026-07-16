//! The page registry — Configurable-EP-style (PROP-041 §2 `#declarative-pages`,
//! §2 `#registry-is-introspectable`, §2 `#stable-id-law`). Settings are
//! organised into **pages** declared up front in a registry; a page carries a
//! stable id, a parent/group, a display name + description, a group weight, a
//! scope flag (application vs project), the preference keys it owns (so the
//! tree can surface an origin hint, §3 `#tree-shows-origin-hint`), and a lazy
//! body built on first open.
//!
//! This is the clean-room IntelliJ `Configurable` EP shape (the research
//! reference, §3.7) — **not** its code. The registry is the enumerable source
//! for both the settings tree (§3) and the future search index (§7): adding a
//! page means registering a declaration, which then appears in the tree with no
//! further wiring. A page `id` is immutable once published (#stable-id-law).
//!
//! Frontend-agnostic metadata only — no rendering here. The tree widget lives
//! in [`super::page_tree`]; the built-in declarations live in
//! [`super::settings`].

specmark::scope!("spec://vibevm/modules/vibe-settings/PROP-041#registry");

use std::collections::{BTreeMap, BTreeSet};

use specmark::spec;

/// The synthetic group id under which pages with an **unresolved** parent land
/// (PROP-041 §2 `#declarative-pages` — "unresolved groups land in 'Other'").
pub const OTHER_GROUP_ID: &str = "__other";

/// The display name of the synthetic catch-all group.
const OTHER_GROUP_NAME: &str = "Other";

/// Whether a page is application-level or project-level (PROP-041 §2
/// `#declarable-pages`, mirroring the IntelliJ `nonDefaultProject` analogue).
///
/// A [`PageScope::Project`] page is hidden in a no-project (L1-only) session
/// (PROP-041 §3 `#tree-context`); an [`PageScope::Application`] page shows in
/// every session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PageScope {
    /// Application-level — shown in every session (L1 + L2 + L3).
    #[default]
    Application,
    /// Project-level — hidden in a no-project (L1-only) session.
    Project,
}

/// The lazy page body (PROP-041 §2 `#declarative-pages` — a lazy page body
/// created on first open; the registry metadata is cheap so the whole tree
/// renders without constructing every form).
///
/// S1 ships the [`PageBody::Placeholder`] — the right pane renders a titled
/// panel; S2 fills the per-type form (§4 `#form-per-type`).
#[derive(Debug, Clone, Default)]
pub enum PageBody {
    /// S1 placeholder — the form is not built yet. S2 replaces this with the
    /// per-type field composition (§4).
    #[default]
    Placeholder,
}

/// The default body constructor — every S1 page uses this. S2 will let each
/// page declare its own form-building function.
fn default_body() -> PageBody {
    PageBody::Placeholder
}

/// One declared settings page (PROP-041 §2 `#declarative-pages`).
///
/// Built with [`PageDecl::new`] (the mandatory id + display name + description)
/// and the `with_*` chain for the optional fields. The `id` is the stable,
/// non-localised join-key (`#stable-id-law`); `parent_id` names a group page
/// this one hangs under (an unresolved parent lands the page in "Other").
#[derive(Debug, Clone)]
pub struct PageDecl {
    /// Stable, non-localised identifier — the join-key (`#stable-id-law`).
    pub id: String,
    /// The parent/group page id, if any. An id no other page declares lands the
    /// page under the synthetic "Other" group (§2 `#declarable-pages`).
    pub parent_id: Option<String>,
    /// The localisable display name (shown in the tree).
    pub display_name: String,
    /// The localisable description (shown in the future search index, §7).
    pub description: String,
    /// Ordering within a parent (`#declarable-pages`); lower sorts first.
    pub group_weight: u32,
    /// Application-level vs project-level (`#declarable-pages`, `#tree-context`).
    pub scope: PageScope,
    /// The preference keys this page owns (dotted paths). Read through
    /// `ResolvedPrefs::inspect` for the origin hint (§3
    /// `#tree-shows-origin-hint`).
    pub keys: Vec<String>,
    /// Lazy body constructor — called on first open (§2 `#declarative-pages`).
    pub body: fn() -> PageBody,
}

impl PageDecl {
    /// Build a page declaration with the mandatory fields. The optional fields
    /// default to: no parent, `group_weight = 0`, `scope = Application`,
    /// no keys, and the default placeholder body.
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        display_name: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            parent_id: None,
            display_name: display_name.into(),
            description: description.into(),
            group_weight: 0,
            scope: PageScope::Application,
            keys: Vec::new(),
            body: default_body,
        }
    }

    /// Set the parent/group id (chains).
    #[must_use]
    pub fn with_parent(mut self, parent_id: impl Into<String>) -> Self {
        self.parent_id = Some(parent_id.into());
        self
    }

    /// Set the ordering weight within the parent (chains).
    #[must_use]
    pub fn with_weight(mut self, weight: u32) -> Self {
        self.group_weight = weight;
        self
    }

    /// Set the scope flag (chains).
    #[allow(dead_code)] // used in tests today; S2's project-scoped pages + the UI use it.
    #[must_use]
    pub fn with_scope(mut self, scope: PageScope) -> Self {
        self.scope = scope;
        self
    }

    /// Set the preference keys this page owns (chains).
    #[must_use]
    pub fn with_keys(mut self, keys: &[&str]) -> Self {
        self.keys = keys.iter().map(|s| (*s).to_owned()).collect();
        self
    }

    /// Set the lazy body constructor (chains).
    #[allow(dead_code)] // S2 fills the form body; S1 ships the placeholder default.
    #[must_use]
    pub fn with_body(mut self, body: fn() -> PageBody) -> Self {
        self.body = body;
        self
    }
}

/// A node in the resolved page hierarchy (PROP-041 §3 `#tree-widget`). A group
/// (`is_group`) carries children; a leaf page carries its [`PageDecl`] and no
/// children. The synthetic "Other" group is a group node with
/// [`PageNode::page`] = `None`.
#[derive(Debug, Clone)]
pub struct PageNode {
    /// The stable id (the page id, or [`OTHER_GROUP_ID`] for the catch-all).
    pub id: String,
    /// The display name.
    pub display_name: String,
    /// Whether this node is a group (non-leaf, foldable) or a leaf page.
    pub is_group: bool,
    /// The page declaration. `Some` for a declared page; `None` for the
    /// synthetic "Other" group (which is not itself a page).
    pub page: Option<PageDecl>,
    /// Child nodes, sorted by `group_weight` then `display_name`. Empty for a
    /// leaf page.
    pub children: Vec<PageNode>,
}

/// The resolved page registry (PROP-041 §2 `#registry-is-introspectable`) —
/// the enumerable source for the settings tree and the future search index.
#[derive(Debug, Clone, Default)]
pub struct PageRegistry {
    /// All declarations, in registration order.
    decls: Vec<PageDecl>,
}

impl PageRegistry {
    /// An empty registry.
    #[allow(dead_code)] // used in tests; S2 plugins register into one.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Build a registry from a ready-made declaration list (the path
    /// [`super::settings`] takes for the built-in `vibe.tree.*` pages).
    #[must_use]
    pub fn from(decls: Vec<PageDecl>) -> Self {
        Self { decls }
    }

    /// Every declaration, in registration order.
    pub fn pages(&self) -> &[PageDecl] {
        &self.decls
    }

    /// The number of declared pages.
    #[allow(dead_code)] // introspection: used in tests + the future AIUI surface.
    pub fn len(&self) -> usize {
        self.decls.len()
    }

    /// Whether no pages are declared.
    #[allow(dead_code)] // introspection: used in tests + the future AIUI surface.
    pub fn is_empty(&self) -> bool {
        self.decls.is_empty()
    }

    /// Resolve the page hierarchy into a tree of groups → pages (PROP-041 §2
    /// `#declarable-pages`, §3 `#tree-widget`). Children are sorted by
    /// `group_weight` then `display_name`; pages whose `parent_id` does not
    /// resolve to a declared page land under the synthetic "Other" group.
    ///
    /// `ctx_has_project` gates [`PageScope::Project`] pages: when `false` (a
    /// no-project / L1-only session) they are omitted and any group left empty
    /// is pruned (§3 `#tree-context`).
    #[spec(implements = "spec://vibevm/modules/vibe-settings/PROP-041#declarative-pages")]
    pub fn tree(&self, ctx_has_project: bool) -> Vec<PageNode> {
        // The set of declared ids — a parent_id that names one of these is a
        // resolved group; any other parent_id is unresolved (→ "Other").
        let declared_ids: BTreeSet<&str> = self.decls.iter().map(|d| d.id.as_str()).collect();

        // Partition into top-level (parent None or a resolved parent) and
        // orphaned (parent set but unresolved). A page whose parent IS declared
        // hangs under that parent (handled below); a page whose parent is not
        // declared lands under "Other".
        let mut by_parent: BTreeMap<&str, Vec<&PageDecl>> = BTreeMap::new();
        let mut top_level: Vec<&PageDecl> = Vec::new();
        let mut orphaned: Vec<&PageDecl> = Vec::new();
        for decl in &self.decls {
            if !ctx_has_project && decl.scope == PageScope::Project {
                continue;
            }
            match &decl.parent_id {
                None => top_level.push(decl),
                Some(parent) if declared_ids.contains(parent.as_str()) => {
                    by_parent.entry(parent.as_str()).or_default().push(decl);
                }
                Some(_) => orphaned.push(decl),
            }
        }

        // A "group" is any page that is referenced as a parent by at least one
        // surviving child. The set of candidate group ids is the parent map's
        // keys; whether a node IS rendered as a group is decided later by
        // `!children.is_empty()` (a group whose every child was filtered out by
        // scope becomes a plain leaf — never an empty folder, §3 #tree-context).
        let group_ids: BTreeSet<&str> = by_parent.keys().copied().collect();

        let mut roots: Vec<PageNode> = Vec::new();
        for decl in &top_level {
            let children = if group_ids.contains(decl.id.as_str()) {
                Self::children_of(decl.id.as_str(), &by_parent, &group_ids, ctx_has_project)
            } else {
                Vec::new()
            };
            roots.push(PageNode {
                id: decl.id.clone(),
                display_name: decl.display_name.clone(),
                is_group: !children.is_empty(),
                page: Some((**decl).clone()),
                children,
            });
        }
        // Sort roots by weight then name.
        roots.sort_by(Self::node_order);

        // The synthetic "Other" group, when any orphaned pages exist.
        if !orphaned.is_empty() {
            let mut other_children: Vec<PageNode> = orphaned
                .iter()
                .map(|d| PageNode {
                    id: d.id.clone(),
                    display_name: d.display_name.clone(),
                    is_group: false,
                    page: Some((*d).clone()),
                    children: Vec::new(),
                })
                .collect();
            other_children.sort_by(Self::node_order);
            roots.push(PageNode {
                id: OTHER_GROUP_ID.to_owned(),
                display_name: OTHER_GROUP_NAME.to_owned(),
                is_group: true,
                page: None,
                children: other_children,
            });
        }
        roots
    }

    /// Recursively build the children of `parent_id`, sorted by weight then
    /// name. A child that is itself referenced as a parent recurses; a child
    /// whose own subtree empties (every grandchild project-scoped and filtered)
    /// becomes a plain leaf — never an empty folder (§3 #tree-context).
    fn children_of(
        parent_id: &str,
        by_parent: &BTreeMap<&str, Vec<&PageDecl>>,
        group_ids: &BTreeSet<&str>,
        ctx_has_project: bool,
    ) -> Vec<PageNode> {
        let Some(siblings) = by_parent.get(parent_id) else {
            return Vec::new();
        };
        let mut nodes: Vec<PageNode> = Vec::new();
        for decl in siblings {
            if !ctx_has_project && decl.scope == PageScope::Project {
                continue;
            }
            let children = if group_ids.contains(decl.id.as_str()) {
                Self::children_of(decl.id.as_str(), by_parent, group_ids, ctx_has_project)
            } else {
                Vec::new()
            };
            nodes.push(PageNode {
                id: decl.id.clone(),
                display_name: decl.display_name.clone(),
                is_group: !children.is_empty(),
                page: Some((**decl).clone()),
                children,
            });
        }
        nodes.sort_by(Self::node_order);
        nodes
    }

    /// The sort key for siblings: `group_weight` ascending, then `display_name`.
    fn node_order(a: &PageNode, b: &PageNode) -> std::cmp::Ordering {
        let wa = a.page.as_ref().map(|p| p.group_weight).unwrap_or(0);
        let wb = b.page.as_ref().map(|p| p.group_weight).unwrap_or(0);
        wa.cmp(&wb)
            .then_with(|| a.display_name.cmp(&b.display_name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(id: &str, name: &str) -> PageDecl {
        PageDecl::new(id, name, "a test page")
    }

    fn page_with_keys(id: &str, name: &str, parent: Option<&str>, keys: &[&str]) -> PageDecl {
        let mut d = PageDecl::new(id, name, "a test page").with_keys(keys);
        if let Some(p) = parent {
            d = d.with_parent(p);
        }
        d
    }

    fn node_ids(nodes: &[PageNode]) -> Vec<String> {
        nodes.iter().map(|n| n.id.clone()).collect()
    }

    fn child_ids(nodes: &[PageNode], parent_id: &str) -> Vec<String> {
        nodes
            .iter()
            .find(|n| n.id == parent_id)
            .map(|n| n.children.iter().map(|c| c.id.clone()).collect())
            .unwrap_or_default()
    }

    #[test]
    fn empty_registry_yields_no_tree() {
        let r = PageRegistry::new();
        assert!(r.is_empty());
        assert!(r.tree(true).is_empty());
    }

    #[test]
    fn top_level_pages_appear_as_roots() {
        let r = PageRegistry::from(vec![page("appearance", "Appearance"), page("tree", "Tree")]);
        let roots = r.tree(true);
        assert_eq!(node_ids(&roots), ["appearance", "tree"]);
        assert!(roots.iter().all(|n| !n.is_group));
    }

    #[test]
    fn a_page_referenced_as_parent_becomes_a_group() {
        // `appearance` is the parent of `palette` → appearance is a group.
        let r = PageRegistry::from(vec![
            page("appearance", "Appearance"),
            page_with_keys("palette", "Palette", Some("appearance"), &[]),
        ]);
        let roots = r.tree(true);
        let appearance = roots.iter().find(|n| n.id == "appearance").unwrap();
        assert!(appearance.is_group);
        assert_eq!(child_ids(&roots, "appearance"), ["palette"]);
        assert!(!appearance.children[0].is_group);
    }

    #[test]
    fn unresolved_parent_lands_under_other() {
        // `palette` names a parent `ghost` that no page declares → "Other".
        let r = PageRegistry::from(vec![page_with_keys(
            "palette",
            "Palette",
            Some("ghost"),
            &[],
        )]);
        let roots = r.tree(true);
        let other = roots.iter().find(|n| n.id == OTHER_GROUP_ID).unwrap();
        assert!(other.is_group);
        assert_eq!(child_ids(&roots, OTHER_GROUP_ID), ["palette"]);
    }

    #[test]
    fn siblings_sort_by_weight_then_name() {
        // weight 10 `b` before weight 20 `a` (weight wins over name).
        let r = PageRegistry::from(vec![
            page("g", "G"),
            page("a", "A").with_parent("g").with_weight(20),
            page("b", "B").with_parent("g").with_weight(10),
        ]);
        let roots = r.tree(true);
        assert_eq!(child_ids(&roots, "g"), ["b", "a"]);
    }

    #[test]
    fn project_scoped_pages_hidden_in_a_no_project_session() {
        let r = PageRegistry::from(vec![
            page("app", "App"),
            page("proj", "Proj").with_scope(PageScope::Project),
        ]);
        // With a project → both visible.
        assert_eq!(node_ids(&r.tree(true)), ["app", "proj"]);
        // No project → proj hidden.
        assert_eq!(node_ids(&r.tree(false)), ["app"]);
    }

    #[test]
    fn project_only_group_demotes_to_a_leaf_in_a_no_project_session() {
        // group `g` has only project-scoped children → pruned when no project.
        let r = PageRegistry::from(vec![
            page("g", "G"),
            page("c", "C")
                .with_parent("g")
                .with_scope(PageScope::Project),
        ]);
        // With a project → g is a group with child c.
        let with_proj = r.tree(true);
        assert_eq!(node_ids(&with_proj), ["g"]);
        let g = with_proj.iter().find(|n| n.id == "g").unwrap();
        assert!(g.is_group);
        // No project → c is filtered out, so g carries no children and is
        // demoted to a plain leaf (never an empty folder, §3 #tree-context).
        let no_proj = r.tree(false);
        assert_eq!(node_ids(&no_proj), ["g"]);
        let g_no_proj = no_proj.iter().find(|n| n.id == "g").unwrap();
        assert!(
            !g_no_proj.is_group,
            "g demotes to a leaf when its children are gone"
        );
        assert!(g_no_proj.children.is_empty());
    }

    #[test]
    fn registry_is_introspectable_via_pages() {
        // #registry-is-introspectable — adding a page means registering a
        // declaration; `pages()` reaches every one without parsing files.
        let r = PageRegistry::from(vec![
            page_with_keys("palette", "Palette", None, &["vibe.tree.palette"]),
            page_with_keys("mode", "Mode", None, &["vibe.tree.mode"]),
        ]);
        let ids: Vec<&str> = r.pages().iter().map(|d| d.id.as_str()).collect();
        assert_eq!(ids, ["palette", "mode"]);
        assert_eq!(r.len(), 2);
    }

    #[test]
    fn keys_round_trip_through_the_builder() {
        let d = page_with_keys(
            "palette",
            "Palette",
            None,
            &["vibe.tree.palette", "vibe.tree.tier"],
        );
        assert_eq!(d.keys, ["vibe.tree.palette", "vibe.tree.tier"]);
    }
}
