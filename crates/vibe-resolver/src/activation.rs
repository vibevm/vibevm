//! Subskill activation evaluator — PROP-003 §2.5.2.
//!
//! Given a resolved dep graph, the project tree, and a per-subskill
//! [`ActivationRules`] block, decide whether each subskill activates.
//! Multiple channels — manual (via parent feature, handled upstream),
//! `if_present`, `if_provides`, `if_files`, `if_command`, `if_env`,
//! `if_describes_match`, `if_language` — compose orthogonally; ANY
//! match wins.
//!
//! `if_command`, `if_env`, and `if_describes_match` are land-now /
//! light-evaluation: simple PATH probe, env-var presence, PURL type
//! match. The full LLM-emitted virtual-capability channel
//! (PROP-003 §2.5.3) is orthogonal and lives in a future module
//! once `vibe-llm` is real.

use std::borrow::Borrow;
use std::collections::BTreeSet;
use std::path::Path;

use specmark::spec;
use thiserror::Error;
use vibe_core::manifest::ActivationRules;

/// A context tag in the closed `<namespace>:<name>` form the
/// activation channels match against — `flow:wal`, `stack:rust`,
/// `capability:wal-protocol`, `interface:build-system`.
///
/// The seam used to take bare `String`s, so a caller could feed
/// `"rust"` where `"stack:rust"` was meant and the probe would just
/// silently never match (card scaffold-b-typed-builders: the
/// statistically-likely wrong call must not type-check). Parsing is
/// the only constructor; the namespace separator is the invariant.
///
/// ```
/// use vibe_resolver::activation::CapabilityTag;
///
/// let tag = CapabilityTag::parse("stack:rust").unwrap();
/// assert_eq!(tag.as_str(), "stack:rust");
/// assert!(CapabilityTag::parse("rust").is_err());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[spec(implements = "spec://vibevm/modules/vibe-resolver/PROP-003#subskill-activation")]
pub struct CapabilityTag(String);

/// Why a raw string is not a [`CapabilityTag`].
#[derive(Debug, Error)]
#[spec(implements = "spec://vibevm/modules/vibe-resolver/PROP-003#subskill-activation")]
pub enum TagError {
    #[error(
        "context tag `{0}` is missing the `<namespace>:` prefix \
         (examples: `stack:rust`, `capability:wal-protocol`) \
         (violates spec://vibevm/modules/vibe-resolver/PROP-003#subskill-activation; \
         fix: write the tag in the closed `<namespace>:<name>` form)"
    )]
    MissingNamespace(String),

    #[error(
        "context tag `{0}` has an empty namespace or name half \
         (violates spec://vibevm/modules/vibe-resolver/PROP-003#subskill-activation; \
         fix: fill both halves of the `<namespace>:<name>` form)"
    )]
    EmptyHalf(String),
}

impl CapabilityTag {
    /// Parse the closed `<namespace>:<name>` form. Both halves must be
    /// non-empty; no other shape exists.
    pub fn parse(raw: impl Into<String>) -> Result<Self, TagError> {
        let raw = raw.into();
        let Some((ns, name)) = raw.split_once(':') else {
            return Err(TagError::MissingNamespace(raw));
        };
        if ns.is_empty() || name.is_empty() {
            return Err(TagError::EmptyHalf(raw));
        }
        Ok(CapabilityTag(raw))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for CapabilityTag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Lets `BTreeSet<CapabilityTag>::contains(&str)` work — the rule
/// strings in a manifest's `[activation]` block stay plain strings
/// (schema surface), and the lookup bridges the two.
impl Borrow<str> for CapabilityTag {
    fn borrow(&self) -> &str {
        &self.0
    }
}

/// Snapshot of project / machine state used to evaluate context probes.
/// Built once per `vibe install` invocation; re-used across every
/// candidate subskill.
#[derive(Debug, Clone, Default)]
#[spec(implements = "spec://vibevm/modules/vibe-resolver/PROP-003#subskill-activation")]
pub struct ActivationContext {
    /// Capabilities and pkgrefs present in the resolved graph.
    /// Examples: `flow:wal`, `stack:rust`, `capability:wal-protocol`.
    pub present: BTreeSet<CapabilityTag>,

    /// Interface tags provided by some package in the graph.
    /// Examples: `interface:build-system`, `interface:auth-provider`.
    pub provides: BTreeSet<CapabilityTag>,

    /// Project root absolute path. Used for `if_files` glob matching.
    pub project_root: Option<std::path::PathBuf>,

    /// Resolved language preference chain (first entry = primary).
    pub language_chain: Vec<String>,

    /// PURL types present in the graph (extracted from `[package].describes`
    /// of every package). E.g. `["cargo", "pypi"]`.
    pub describes_types: BTreeSet<String>,
}

impl ActivationContext {
    pub fn add_present(&mut self, tag: CapabilityTag) {
        self.present.insert(tag);
    }

    pub fn add_provides(&mut self, tag: CapabilityTag) {
        self.provides.insert(tag);
    }
}

/// Outcome of evaluating one subskill's activation rules. `Active`
/// carries the channels that fired (for diagnostic output).
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(implements = "spec://vibevm/modules/vibe-resolver/PROP-003#subskill-activation")]
pub struct ActivationOutcome {
    pub active: bool,
    pub channels_matched: Vec<&'static str>,
}

/// Evaluate one subskill's `[activation]` block against the context.
///
/// Channel semantics: ANY match activates. An empty rule block (no
/// probes specified) returns `active = false` — manual activation via
/// parent feature is handled at a higher layer and is not modelled
/// here. `subskill_describes_type` is the lowercased PURL type of the
/// subskill's own `describes` field (if any) — used by
/// `if_describes_match`.
#[spec(implements = "spec://vibevm/modules/vibe-resolver/PROP-003#subskill-activation")]
#[spec(
    deviates = "spec://vibevm/modules/vibe-resolver/PROP-003#subskill-activation",
    reason = "the §2.5.2 `if_os` channel is not probed here: the spec reserves it as \
              schema-only, \"inert until the activation engine is built\" — the engine \
              now exists and the probe is still unimplemented; recorded per the owner \
              APPROVE of PRP-0042 so the gap stays visible"
)]
pub fn evaluate(
    rules: &ActivationRules,
    ctx: &ActivationContext,
    subskill_describes_type: Option<&str>,
) -> ActivationOutcome {
    let mut channels: Vec<&'static str> = Vec::new();

    if !rules.if_present.is_empty()
        && rules
            .if_present
            .iter()
            .any(|tag| ctx.present.contains(tag.as_str()))
    {
        channels.push("if_present");
    }
    if !rules.if_provides.is_empty()
        && rules
            .if_provides
            .iter()
            .any(|tag| ctx.provides.contains(tag.as_str()))
    {
        channels.push("if_provides");
    }
    if !rules.if_files.is_empty()
        && let Some(root) = ctx.project_root.as_deref()
        && any_file_matches(root, &rules.if_files)
    {
        channels.push("if_files");
    }
    if !rules.if_command.is_empty() && rules.if_command.iter().any(|c| command_resolves_on_path(c))
    {
        channels.push("if_command");
    }
    if !rules.if_env.is_empty()
        && rules
            .if_env
            .iter()
            .any(|name| std::env::var_os(name).is_some())
    {
        channels.push("if_env");
    }
    if rules.if_describes_match
        && let Some(t) = subskill_describes_type
        && ctx.describes_types.contains(t)
    {
        channels.push("if_describes_match");
    }
    if !rules.if_language.is_empty()
        && rules
            .if_language
            .iter()
            .any(|lang| ctx.language_chain.iter().any(|c| c == lang))
    {
        channels.push("if_language");
    }

    ActivationOutcome {
        active: !channels.is_empty(),
        channels_matched: channels,
    }
}

/// Walk the project tree (bounded depth, ignoring common noise) and
/// return true if at least one file matches any of the given glob
/// patterns.
fn any_file_matches(root: &Path, patterns: &[String]) -> bool {
    let matchers: Vec<glob_match::Matcher> = patterns
        .iter()
        .map(|p| glob_match::Matcher::new(p))
        .collect();
    if matchers.is_empty() {
        return false;
    }
    for entry in walkdir::WalkDir::new(root)
        .max_depth(8)
        .into_iter()
        .filter_entry(|e| {
            // Skip well-known noise dirs that bloat probe time.
            let name = e.file_name().to_string_lossy();
            !matches!(
                name.as_ref(),
                ".git" | "node_modules" | "target" | ".tessl" | ".vibe" | "refs"
            )
        })
    {
        let Ok(entry) = entry else { continue };
        if !entry.file_type().is_file() {
            continue;
        }
        let rel = match entry.path().strip_prefix(root) {
            Ok(p) => p,
            Err(_) => continue,
        };
        let s = rel.to_string_lossy().replace('\\', "/");
        if matchers.iter().any(|m| m.matches(&s)) {
            return true;
        }
    }
    false
}

#[spec(
    deviates = "spec://discipline-core/mechanisms/ENGINE-CONFORM-v0.1#rules",
    reason = "ambient-env: the `if_command` activation channel (PROP-003 §2.5.2) resolves \
              a command against PATH — reading `PATH` is the predicate's definition, \
              inherent-env domain, not config a composition root could thread in ahead of \
              the user's `[activation]` rule"
)]
fn command_resolves_on_path(name: &str) -> bool {
    let path_var = match std::env::var_os("PATH") {
        Some(v) => v,
        None => return false,
    };
    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return true;
        }
        // Windows: also try with .exe / .cmd / .bat suffixes.
        if cfg!(windows) {
            for suffix in [".exe", ".cmd", ".bat"] {
                let mut with_suffix = candidate.clone();
                let new_name = format!(
                    "{}{}",
                    candidate.file_name().and_then(|n| n.to_str()).unwrap_or(""),
                    suffix
                );
                with_suffix.set_file_name(new_name);
                if with_suffix.is_file() {
                    return true;
                }
            }
        }
    }
    false
}

mod glob_match {
    /// Tiny glob matcher — supports `*`, `**`, `?`. Sufficient for
    /// activation probes; full glob (brace expansion, character
    /// classes) is not pulled in to avoid a heavy dep.
    pub struct Matcher {
        pattern: String,
    }

    impl Matcher {
        pub fn new(pattern: &str) -> Self {
            Matcher {
                pattern: pattern.to_string(),
            }
        }

        pub fn matches(&self, path: &str) -> bool {
            glob_match(&self.pattern, path)
        }
    }

    /// Recursive glob match. `**` matches any number of path segments
    /// (including zero); `*` matches any character within a single
    /// segment; `?` matches one character.
    fn glob_match(pattern: &str, text: &str) -> bool {
        // Special-case bare `**` — match anything.
        if pattern == "**" {
            return true;
        }
        glob_match_inner(pattern.as_bytes(), text.as_bytes())
    }

    fn glob_match_inner(pattern: &[u8], text: &[u8]) -> bool {
        let mut pi = 0;
        let mut ti = 0;
        let mut star: Option<(usize, usize)> = None; // (pi, ti) when last `*` seen
        let mut double_star: Option<(usize, usize)> = None;

        while ti < text.len() {
            if pi < pattern.len() {
                let pc = pattern[pi];
                let tc = text[ti];
                if pc == b'*' {
                    // ** absorbs any number of segments INCLUDING `/`.
                    if pi + 1 < pattern.len() && pattern[pi + 1] == b'*' {
                        double_star = Some((pi, ti));
                        pi += 2;
                        // Consume optional trailing `/` after `**`.
                        if pi < pattern.len() && pattern[pi] == b'/' {
                            pi += 1;
                        }
                        continue;
                    }
                    star = Some((pi, ti));
                    pi += 1;
                    continue;
                }
                if pc == b'?' {
                    if tc == b'/' {
                        // `?` doesn't match path separator.
                    } else {
                        pi += 1;
                        ti += 1;
                        continue;
                    }
                }
                if pc == tc {
                    pi += 1;
                    ti += 1;
                    continue;
                }
                // No match — backtrack.
            }
            if let Some((sp, st)) = star {
                // Single `*` does not cross `/`.
                if text[st] == b'/' {
                    star = None;
                } else {
                    pi = sp + 1;
                    ti = st + 1;
                    star = Some((sp, st + 1));
                    continue;
                }
            }
            if let Some((sp, st)) = double_star {
                pi = sp + 2;
                if pi < pattern.len() && pattern[pi] == b'/' {
                    pi += 1;
                }
                ti = st + 1;
                double_star = Some((sp, st + 1));
                continue;
            }
            return false;
        }
        // Consume trailing `*` / `**` in pattern.
        while pi < pattern.len() {
            match pattern[pi] {
                b'*' => pi += 1,
                b'/' => pi += 1,
                _ => return false,
            }
        }
        true
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn matches_double_star_glob() {
            assert!(glob_match("**/Cargo.toml", "Cargo.toml"));
            assert!(glob_match("**/Cargo.toml", "subdir/Cargo.toml"));
            assert!(glob_match("**/Cargo.toml", "deep/nested/path/Cargo.toml"));
            assert!(!glob_match("**/Cargo.toml", "Cargo.lock"));
        }

        #[test]
        fn matches_single_star_glob() {
            assert!(glob_match("*.toml", "Cargo.toml"));
            assert!(!glob_match("*.toml", "sub/Cargo.toml"));
        }

        #[test]
        fn matches_question() {
            assert!(glob_match("a?c", "abc"));
            assert!(!glob_match("a?c", "ac"));
        }

        #[test]
        fn matches_literal_path() {
            assert!(glob_match("src/lib.rs", "src/lib.rs"));
            assert!(!glob_match("src/lib.rs", "src/main.rs"));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use specmark::verifies;

    use super::*;

    #[test]
    #[verifies("spec://vibevm/modules/vibe-resolver/PROP-003#subskill-activation")]
    fn empty_rules_inactive() {
        let rules = ActivationRules::default();
        let ctx = ActivationContext::default();
        let outcome = evaluate(&rules, &ctx, None);
        assert!(!outcome.active);
        assert!(outcome.channels_matched.is_empty());
    }

    #[test]
    #[verifies("spec://vibevm/modules/vibe-resolver/PROP-003#subskill-activation")]
    fn if_present_match() {
        let rules = ActivationRules {
            if_present: vec!["stack:rust".into()],
            ..Default::default()
        };
        let mut ctx = ActivationContext::default();
        ctx.add_present(CapabilityTag::parse("stack:rust").unwrap());
        let outcome = evaluate(&rules, &ctx, None);
        assert!(outcome.active);
        assert_eq!(outcome.channels_matched, vec!["if_present"]);
    }

    #[test]
    fn if_present_no_match_no_other_probes() {
        let rules = ActivationRules {
            if_present: vec!["stack:rust".into()],
            ..Default::default()
        };
        let ctx = ActivationContext::default();
        let outcome = evaluate(&rules, &ctx, None);
        assert!(!outcome.active);
    }

    #[test]
    #[verifies("spec://vibevm/modules/vibe-resolver/PROP-003#subskill-activation")]
    fn if_provides_match() {
        let rules = ActivationRules {
            if_provides: vec!["interface:build-system".into()],
            ..Default::default()
        };
        let mut ctx = ActivationContext::default();
        ctx.add_provides(CapabilityTag::parse("interface:build-system").unwrap());
        let outcome = evaluate(&rules, &ctx, None);
        assert!(outcome.active);
        assert_eq!(outcome.channels_matched, vec!["if_provides"]);
    }

    #[test]
    #[verifies("spec://vibevm/modules/vibe-resolver/PROP-003#subskill-activation")]
    fn if_files_match_via_glob() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir_all(tmp.path().join("src")).unwrap();
        fs::write(tmp.path().join("Cargo.toml"), "x").unwrap();
        let rules = ActivationRules {
            if_files: vec!["**/Cargo.toml".into()],
            ..Default::default()
        };
        let ctx = ActivationContext {
            project_root: Some(tmp.path().to_path_buf()),
            ..Default::default()
        };
        let outcome = evaluate(&rules, &ctx, None);
        assert!(outcome.active);
        assert_eq!(outcome.channels_matched, vec!["if_files"]);
    }

    #[test]
    fn if_files_no_match() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("README.md"), "x").unwrap();
        let rules = ActivationRules {
            if_files: vec!["**/Cargo.toml".into()],
            ..Default::default()
        };
        let ctx = ActivationContext {
            project_root: Some(tmp.path().to_path_buf()),
            ..Default::default()
        };
        let outcome = evaluate(&rules, &ctx, None);
        assert!(!outcome.active);
    }

    #[test]
    #[verifies("spec://vibevm/modules/vibe-resolver/PROP-003#subskill-activation")]
    fn if_describes_match_uses_subskill_type() {
        let rules = ActivationRules {
            if_describes_match: true,
            ..Default::default()
        };
        let mut ctx = ActivationContext::default();
        ctx.describes_types.insert("cargo".into());
        let outcome = evaluate(&rules, &ctx, Some("cargo"));
        assert!(outcome.active);
    }

    #[test]
    fn if_describes_no_match_when_type_absent() {
        let rules = ActivationRules {
            if_describes_match: true,
            ..Default::default()
        };
        let mut ctx = ActivationContext::default();
        ctx.describes_types.insert("pypi".into());
        let outcome = evaluate(&rules, &ctx, Some("cargo"));
        assert!(!outcome.active);
    }

    #[test]
    #[verifies("spec://vibevm/modules/vibe-resolver/PROP-003#subskill-activation")]
    fn if_language_match() {
        let rules = ActivationRules {
            if_language: vec!["ru".into()],
            ..Default::default()
        };
        let ctx = ActivationContext {
            language_chain: vec!["ru".into(), "en".into()],
            ..Default::default()
        };
        let outcome = evaluate(&rules, &ctx, None);
        assert!(outcome.active);
    }

    #[test]
    #[verifies("spec://vibevm/modules/vibe-resolver/PROP-003#subskill-activation")]
    fn multiple_channels_compose() {
        let rules = ActivationRules {
            if_present: vec!["stack:rust".into()],
            if_language: vec!["en".into()],
            ..Default::default()
        };
        let mut ctx = ActivationContext {
            language_chain: vec!["en".into()],
            ..Default::default()
        };
        ctx.add_present(CapabilityTag::parse("stack:rust").unwrap());
        let outcome = evaluate(&rules, &ctx, None);
        assert!(outcome.active);
        assert_eq!(outcome.channels_matched.len(), 2);
    }
}
