//! Which package groups' package-authored code runs without a question —
//! the one trust mechanism of vibevm (PROP-064 `##consent`, PROP-065
//! `##ordinary-trust`, PROP-020 `##trust-gate`).
//!
//! Three planes used to answer this question, and two of them carried their
//! own list: an agent step read `trusted_groups` of `<settings>/agents.toml`
//! by exact string, and a `[hooks]` entry read `vibe-workspace`'s own
//! `DEFAULT_ALLOWED_GROUPS` constant. The owner's decision of 2026-11-10
//! makes everything under `org.vibevm.*` trusted by default and the
//! mechanism configurable, which only one list can be: this module is it,
//! and every consumer reads a [`TrustPolicy`] resolved here.
//!
//! Nothing in this module reads a file, an environment variable or a
//! terminal. It is pure over the written entries of the three layers — the
//! base list vibe ships, the site profile's `[trust]` table and the user's
//! `agents.toml` — so the law is tested over values rather than over
//! whatever this machine's home happens to hold.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#ordinary-trust");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-064#consent");

use serde::{Deserialize, Serialize};
use specmark::spec;

use crate::Group;

/// What one written entry may be, and what it covers — the grammar half
/// of this mechanism, in its own cell.
mod pattern;

pub use pattern::{GroupPattern, TrustError};

/// The trusted groups vibe ships — the base list of PROP-065
/// `##ordinary-trust`, and the whole of what used to be PROP-020
/// `##trust-gate`'s `DEFAULT_ALLOWED_GROUPS`.
///
/// It is data in exactly one place, and a subtree pattern rather than a
/// group, because the owner's decision is about everything vibevm
/// publishes and not only about the one group that happens to hold the
/// core packages.
///
/// ```
/// assert_eq!(vibe_core::trust::BASE_TRUSTED_GROUPS, ["org.vibevm.*"]);
/// ```
pub const BASE_TRUSTED_GROUPS: &[&str] = &["org.vibevm.*"];

/// Which layer an entry was written in, so a reader can see why a group is
/// trusted and which file to edit.
///
/// ```
/// use vibe_core::trust::TrustLayer;
///
/// assert_eq!(TrustLayer::Base.as_str(), "base");
/// assert_eq!(TrustLayer::User.as_str(), "user");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#ordinary-trust")]
pub enum TrustLayer {
    /// [`BASE_TRUSTED_GROUPS`] — what vibe itself ships.
    Base,
    /// The `[trust]` table of the applied site profile
    /// (`<settings>/site-profile.toml`, PROP-019 `#site-profile`).
    Site,
    /// `trusted_groups` / `untrusted_groups` of `<settings>/agents.toml`.
    User,
    /// Granted for this run alone — an operator who answered the question,
    /// or `--allow-hooks`. Never written to a file.
    Run,
}

impl TrustLayer {
    /// The layer's name as a report prints it.
    ///
    /// ```
    /// assert_eq!(vibe_core::trust::TrustLayer::Site.as_str(), "site");
    /// ```
    pub fn as_str(self) -> &'static str {
        match self {
            TrustLayer::Base => "base",
            TrustLayer::Site => "site",
            TrustLayer::User => "user",
            TrustLayer::Run => "run",
        }
    }
}

/// The two written lists one layer may carry.
///
/// The site profile spells them as a `[trust]` table and `agents.toml` as
/// two top-level arrays; the shape the resolution reads is the same, which
/// is why it is one type.
///
/// ```
/// use vibe_core::trust::TrustLists;
///
/// let site: TrustLists =
///     toml::from_str("trusted_groups = ['org.corp.*']\nuntrusted_groups = ['org.corp.legacy']")?;
/// assert_eq!(site.trusted_groups, ["org.corp.*"]);
/// assert!(!site.is_empty());
/// assert!(TrustLists::default().is_empty());
/// # Ok::<(), toml::de::Error>(())
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#ordinary-trust")]
pub struct TrustLists {
    /// Groups and subtrees whose packages' authored code runs unasked.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trusted_groups: Vec<String>,
    /// Groups and subtrees that are never trusted, whatever any layer —
    /// including this one — says. This is how the base list is refused.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub untrusted_groups: Vec<String>,
}

impl TrustLists {
    /// The two lists of one layer, cloned from what their file holds.
    ///
    /// ```
    /// use vibe_core::trust::TrustLists;
    ///
    /// let lists = TrustLists::new(&["org.example".to_string()], &[]);
    /// assert_eq!(lists.trusted_groups, ["org.example"]);
    /// ```
    pub fn new(trusted: &[String], untrusted: &[String]) -> Self {
        Self {
            trusted_groups: trusted.to_vec(),
            untrusted_groups: untrusted.to_vec(),
        }
    }

    /// Whether the layer says nothing about trust at all.
    ///
    /// ```
    /// assert!(vibe_core::trust::TrustLists::default().is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.trusted_groups.is_empty() && self.untrusted_groups.is_empty()
    }
}

/// One entry of a resolved policy with the layer that wrote it.
///
/// ```
/// use vibe_core::trust::{TrustLayer, TrustPolicy};
///
/// let policy = TrustPolicy::base();
/// assert_eq!(policy.trusted()[0].written(), "org.vibevm.*");
/// assert_eq!(policy.trusted()[0].layer(), TrustLayer::Base);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#ordinary-trust")]
pub struct TrustEntry {
    pattern: GroupPattern,
    layer: TrustLayer,
}

impl TrustEntry {
    /// The entry as written.
    ///
    /// ```
    /// assert_eq!(vibe_core::trust::TrustPolicy::base().trusted()[0].written(), "org.vibevm.*");
    /// ```
    pub fn written(&self) -> &str {
        self.pattern.written()
    }

    /// The layer that wrote it.
    ///
    /// ```
    /// use vibe_core::trust::{TrustLayer, TrustPolicy};
    /// assert_eq!(TrustPolicy::base().trusted()[0].layer(), TrustLayer::Base);
    /// ```
    pub fn layer(&self) -> TrustLayer {
        self.layer
    }

    /// Whether the entry covers `group`.
    ///
    /// ```
    /// assert!(vibe_core::trust::TrustPolicy::base().trusted()[0].matches("org.vibevm.node"));
    /// ```
    pub fn matches(&self, group: &str) -> bool {
        self.pattern.matches(group)
    }

    /// The pattern itself, for an intersection against another entry.
    pub fn pattern(&self) -> &GroupPattern {
        &self.pattern
    }
}

/// What a resolved policy answers about one group, and why.
///
/// ```
/// use vibe_core::trust::{TrustLists, TrustPolicy, TrustVerdict};
///
/// let user = TrustLists::new(&[], &["org.vibevm.ai-native".to_string()]);
/// let policy = TrustPolicy::effective(&TrustLists::default(), &user).unwrap();
/// assert!(matches!(policy.verdict("org.vibevm.node"), TrustVerdict::Trusted { .. }));
/// assert!(matches!(
///     policy.verdict("org.vibevm.ai-native"),
///     TrustVerdict::Untrusted { .. },
/// ));
/// assert!(matches!(policy.verdict("org.example"), TrustVerdict::Unlisted));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#ordinary-trust")]
pub enum TrustVerdict<'a> {
    /// A trusted entry covers the group and no deny entry does.
    Trusted { by: &'a TrustEntry },
    /// A deny entry covers the group. It wins over every trusted entry,
    /// whichever layer wrote either one.
    Untrusted { by: &'a TrustEntry },
    /// No entry of either list covers the group: it is not trusted, and
    /// nothing was said about it.
    Unlisted,
}

impl TrustVerdict<'_> {
    /// Whether the group's packages run unasked.
    ///
    /// ```
    /// use vibe_core::trust::TrustVerdict;
    /// assert!(!TrustVerdict::Unlisted.trusted());
    /// ```
    pub fn trusted(&self) -> bool {
        matches!(self, TrustVerdict::Trusted { .. })
    }
}

/// The effective trust of one run: the trusted entries of every layer, and
/// the deny entries that override them.
///
/// **The rule, in one sentence:** a group is trusted when some entry of the
/// trusted list covers it and **no** entry of the deny list does. A deny
/// wins over the base list, over the site's own trusted list and over the
/// user's own — which is what keeps the answer independent of the order the
/// layers happen to be read in.
///
/// ```
/// use vibe_core::trust::{TrustLists, TrustPolicy};
///
/// // The base list alone: everything vibevm publishes is trusted.
/// let shipped = TrustPolicy::effective(&TrustLists::default(), &TrustLists::default()).unwrap();
/// assert!(shipped.trusts("org.vibevm.ai-native"));
/// assert!(!shipped.trusts("org.example"));
///
/// // The user refuses one subtree of it, and keeps the rest.
/// let user = TrustLists::new(&["org.example".to_string()], &["org.vibevm.*".to_string()]);
/// let policy = TrustPolicy::effective(&TrustLists::default(), &user).unwrap();
/// assert!(!policy.trusts("org.vibevm"));
/// assert!(!policy.trusts("org.vibevm.ai-native"));
/// assert!(policy.trusts("org.example"));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#ordinary-trust")]
pub struct TrustPolicy {
    trusted: Vec<TrustEntry>,
    untrusted: Vec<TrustEntry>,
}

/// The one borrowable empty policy, so a caller that states no trust can
/// borrow one rather than keep a value alive for it. A `static` and not a
/// promoted `const`, because `Vec` has a destructor.
static EMPTY_POLICY: TrustPolicy = TrustPolicy {
    trusted: Vec::new(),
    untrusted: Vec::new(),
};

impl Default for &TrustPolicy {
    /// A borrowed policy with nothing said in it trusts nothing — the
    /// posture a plane that supplies its own consent starts from.
    ///
    /// ```
    /// let borrowed: &vibe_core::trust::TrustPolicy = Default::default();
    /// assert!(!borrowed.trusts("org.vibevm"));
    /// ```
    fn default() -> Self {
        &EMPTY_POLICY
    }
}

impl TrustPolicy {
    /// A policy that trusts nothing and denies nothing — the value a caller
    /// states rather than defaults into, for a plane that supplies its own
    /// consent.
    pub const EMPTY: Self = Self {
        trusted: Vec::new(),
        untrusted: Vec::new(),
    };

    /// The base list alone — what vibe ships, with no file read.
    ///
    /// ```
    /// assert!(vibe_core::trust::TrustPolicy::base().trusts("org.vibevm.node"));
    /// ```
    pub fn base() -> Self {
        Self {
            trusted: BASE_TRUSTED_GROUPS
                .iter()
                .map(|entry| TrustEntry {
                    // The shipped list is this crate's own data, so this is
                    // infallible by construction rather than by `expect` —
                    // and the test beside this module proves each entry
                    // parses to exactly the pattern built here, so the base
                    // can never mean something an operator cannot write.
                    pattern: GroupPattern::shipped(entry),
                    layer: TrustLayer::Base,
                })
                .collect(),
            untrusted: Vec::new(),
        }
    }

    /// Resolve the effective policy: the base list, then the site profile's
    /// `[trust]` table, then the user's `agents.toml`, with the deny
    /// entries of both files overriding all of it.
    ///
    /// ```
    /// use vibe_core::trust::{TrustLayer, TrustLists, TrustPolicy};
    ///
    /// let site = TrustLists::new(&["org.corp.*".to_string()], &[]);
    /// let user = TrustLists::new(&["org.example".to_string()], &[]);
    /// let policy = TrustPolicy::effective(&site, &user).unwrap();
    /// assert_eq!(
    ///     policy.trusted().iter().map(|e| (e.written(), e.layer())).collect::<Vec<_>>(),
    ///     [
    ///         ("org.vibevm.*", TrustLayer::Base),
    ///         ("org.corp.*", TrustLayer::Site),
    ///         ("org.example", TrustLayer::User),
    ///     ],
    /// );
    /// ```
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-065#ordinary-trust")]
    pub fn effective(site: &TrustLists, user: &TrustLists) -> Result<Self, TrustError> {
        let mut policy = Self::base();
        for (layer, lists) in [(TrustLayer::Site, site), (TrustLayer::User, user)] {
            policy
                .trusted
                .extend(entries(layer, &lists.trusted_groups)?);
            policy
                .untrusted
                .extend(entries(layer, &lists.untrusted_groups)?);
        }
        Ok(policy)
    }

    /// A policy that trusts exactly these written entries and nothing else
    /// — no base list. The door a fixture, a test and a legacy caller that
    /// states its own trust come through.
    ///
    /// ```
    /// use vibe_core::trust::TrustPolicy;
    ///
    /// let policy = TrustPolicy::trusting(["org.example"]).unwrap();
    /// assert!(policy.trusts("org.example"));
    /// assert!(!policy.trusts("org.vibevm"));
    /// ```
    pub fn trusting<S: AsRef<str>>(
        written: impl IntoIterator<Item = S>,
    ) -> Result<Self, TrustError> {
        Ok(Self {
            trusted: written
                .into_iter()
                .map(|entry| {
                    GroupPattern::parse(entry.as_ref()).map(|pattern| TrustEntry {
                        pattern,
                        layer: TrustLayer::User,
                    })
                })
                .collect::<Result<_, _>>()?,
            untrusted: Vec::new(),
        })
    }

    /// Trust one group for this run alone — the operator answered the
    /// question, or `--allow-hooks` was passed. A validated [`Group`] is a
    /// valid exact entry, so this cannot fail.
    ///
    /// ```
    /// use vibe_core::trust::{TrustLayer, TrustPolicy};
    ///
    /// let mut policy = TrustPolicy::EMPTY;
    /// policy.allow_for_this_run(&vibe_core::Group::parse("org.example").unwrap());
    /// assert!(policy.trusts("org.example"));
    /// assert_eq!(policy.trusted()[0].layer(), TrustLayer::Run);
    /// ```
    pub fn allow_for_this_run(&mut self, group: &Group) {
        self.trusted.push(TrustEntry {
            pattern: GroupPattern::exact(group),
            layer: TrustLayer::Run,
        });
    }

    /// Whether this group's package-authored code runs unasked.
    ///
    /// ```
    /// assert!(vibe_core::trust::TrustPolicy::base().trusts("org.vibevm"));
    /// ```
    pub fn trusts(&self, group: &str) -> bool {
        self.verdict(group).trusted()
    }

    /// The verdict with the entry that decided it, for a report and a
    /// refusal that can name what denied the group.
    ///
    /// ```
    /// use vibe_core::trust::{TrustPolicy, TrustVerdict};
    ///
    /// match TrustPolicy::base().verdict("org.vibevm.node") {
    ///     TrustVerdict::Trusted { by } => assert_eq!(by.written(), "org.vibevm.*"),
    ///     other => panic!("{other:?}"),
    /// }
    /// ```
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-065#ordinary-trust")]
    pub fn verdict(&self, group: &str) -> TrustVerdict<'_> {
        // The deny list is consulted first and wins unconditionally, so the
        // answer does not depend on the order the layers were read in.
        if let Some(by) = self.untrusted.iter().find(|entry| entry.matches(group)) {
            return TrustVerdict::Untrusted { by };
        }
        match self.trusted.iter().find(|entry| entry.matches(group)) {
            Some(by) => TrustVerdict::Trusted { by },
            None => TrustVerdict::Unlisted,
        }
    }

    /// The trusted entries, base first, in the order their layers are read.
    ///
    /// ```
    /// assert_eq!(vibe_core::trust::TrustPolicy::base().trusted().len(), 1);
    /// ```
    pub fn trusted(&self) -> &[TrustEntry] {
        &self.trusted
    }

    /// The deny entries, in the order their layers are read.
    ///
    /// ```
    /// assert!(vibe_core::trust::TrustPolicy::base().untrusted().is_empty());
    /// ```
    pub fn untrusted(&self) -> &[TrustEntry] {
        &self.untrusted
    }

    /// The layers whose trusted entries a deny entry overrides — what lets
    /// a listing say «untrusted (overrides base)» rather than print two
    /// lists and leave the reader to intersect them.
    ///
    /// ```
    /// use vibe_core::trust::{TrustLayer, TrustLists, TrustPolicy};
    ///
    /// let user = TrustLists::new(&[], &["org.vibevm.ai-native".to_string()]);
    /// let policy = TrustPolicy::effective(&TrustLists::default(), &user).unwrap();
    /// let deny = &policy.untrusted()[0];
    /// assert_eq!(policy.overridden_by(deny), [TrustLayer::Base]);
    /// ```
    pub fn overridden_by(&self, deny: &TrustEntry) -> Vec<TrustLayer> {
        let mut layers: Vec<TrustLayer> = self
            .trusted
            .iter()
            .filter(|entry| entry.pattern().intersects(deny.pattern()))
            .map(TrustEntry::layer)
            .collect();
        layers.dedup();
        layers
    }
}

fn entries(layer: TrustLayer, written: &[String]) -> Result<Vec<TrustEntry>, TrustError> {
    written
        .iter()
        .map(|entry| GroupPattern::parse(entry).map(|pattern| TrustEntry { pattern, layer }))
        .collect()
}

#[cfg(test)]
mod tests;
