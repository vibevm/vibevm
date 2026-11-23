use super::refusal_of;
use crate::output::language::Refusal;

/// Identifiers and copyable commands survive a narrow viewport intact.
#[test]
fn refusal_chain_preserves_unbroken_values() {
    let see = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#pkgref";
    let command = "vibe registry alias corp add bad-name org.x/y";
    let refusal = Refusal {
        headline: "a refusal".into(),
        why: Some("ordinary prose wraps under its label without cutting any words".into()),
        fix: vec![command.into()],
        see: Some(see.into()),
        ..Refusal::default()
    };
    assert_eq!(
        refusal.block().render_at(40),
        format!(
            "✗ a refusal\n  why   ordinary prose wraps under its\n        label without cutting any words\n  fix   {command}\n  see   {see}"
        )
    );
}

#[test]
fn an_error_with_a_violates_tail_reads_as_code_headline_fix_and_see() {
    let err = anyhow::anyhow!(
        "consent_required: 1 remap package offer(s) need(s) an answer — org.corp/intranet \
         (violates spec://org.vibevm.core/vibevm/common/PROP-066#consent; \
          fix: `vibe remap accept org.corp/intranet`, or `vibe remap reject org.corp/intranet`; \
          `vibe remap list` shows every row involved)"
    );
    let refusal = refusal_of(&err);
    assert_eq!(refusal.code.as_deref(), Some("consent_required"));
    assert_eq!(
        refusal.headline,
        "1 remap package offer(s) need(s) an answer — org.corp/intranet"
    );
    assert_eq!(
        refusal.see.as_deref(),
        Some("spec://org.vibevm.core/vibevm/common/PROP-066#consent")
    );
    assert_eq!(
        refusal.fix,
        [
            "vibe remap accept org.corp/intranet",
            "vibe remap reject org.corp/intranet",
            "vibe remap list"
        ]
    );
}

#[test]
fn commands_joined_by_or_alone_need_no_sentence() {
    let err = anyhow::anyhow!("x (violates spec://s#a; fix: `vibe a`, or `vibe b`)");
    assert_eq!(refusal_of(&err).fix, ["vibe a", "vibe b"]);
}

#[test]
fn a_cause_that_is_only_a_tail_lends_its_fix_and_no_why() {
    let err = anyhow::anyhow!(
        "not_installed: application `org.example/editor` is not installed in the project home p \
         (violates spec://s#commands; fix: use -g for the user home, or check `vibe app list` \
         for the installed keys): (violates spec://s#cause; fix: act on the cause)"
    );
    let refusal = refusal_of(&err);
    assert_eq!(refusal.code.as_deref(), Some("not_installed"));
    assert_eq!(refusal.why, None);
    assert_eq!(refusal.see.as_deref(), Some("spec://s#commands"));
    assert_eq!(refusal.fix, ["act on the cause", "vibe app list"]);
}

#[test]
fn a_plain_error_is_its_own_headline() {
    let err = anyhow::anyhow!("inner").context("outer");
    let refusal = refusal_of(&err);
    assert_eq!(refusal.code, None);
    assert_eq!(refusal.headline, "outer: inner");
    assert!(refusal.fix.is_empty() && refusal.see.is_none());
}

#[test]
fn a_cause_behind_a_bare_tail_still_reaches_the_why_and_fixes_are_not_repeated() {
    // The shape of a drop miss whose declared origin source policy excludes
    // (`PROP-066#drops`, `PROP-073#HTTP-ADMISSION-LAW`): the origin's
    // refusal sits behind a cause that is only a tail, and both causes carry
    // the same installer-source fix.
    let err = anyhow::anyhow!(
        "step_failed: step `codkit` (archive) failed \
         (violates spec://s#failure; fix: correct the cause below): \
         fetching `https://github.com/o/r/a.tar.gz` failed: the drop holds no `a.tar.gz` \
         (violates spec://s#drops; `vibe artifacts where` prints the expected locations) \
         (violates spec://s#source; fix: check the URL or the package-relative path); \
         declared origin: excluded HTTP endpoint https://github.com \
         (violates spec://s#admission; fix: use an admitted HTTP endpoint) \
         (violates spec://s#source; fix: check the URL or the package-relative path)"
    );
    let refusal = refusal_of(&err);
    assert_eq!(refusal.code.as_deref(), Some("step_failed"));
    assert_eq!(
        refusal.why.as_deref(),
        Some(
            "fetching `https://github.com/o/r/a.tar.gz` failed: the drop holds no `a.tar.gz`; \
             declared origin: excluded HTTP endpoint https://github.com"
        )
    );
    assert_eq!(
        refusal.fix,
        [
            "check the URL or the package-relative path",
            "use an admitted HTTP endpoint",
            "correct the cause below"
        ]
    );
}

#[test]
fn a_cancelled_question_is_one_line() {
    let line = super::cancelled_line();
    assert!(line.ends_with(" cancelled"), "{line}");
    assert!(!line.contains('\n'), "{line}");
}

/// Reads a cause nested in parentheses.
#[test]
fn refusal_chain_reads_a_cause_nested_in_parentheses() {
    let err = anyhow::anyhow!(
        "the slot `node`'s ecosystems could not be read (slot `node` has no installed provider \
         (violates spec://org.vibevm.core/vibevm/common/PROP-059#alternatives; fix: check \
         `vibe app list` for the installed slots)); name one with `--ecosystem ID`"
    );
    let refusal = refusal_of(&err);
    assert_eq!(refusal.code, None);
    assert_eq!(
        refusal.headline,
        "the slot `node`'s ecosystems could not be read; name one with `--ecosystem ID`"
    );
    assert_eq!(
        refusal.why.as_deref(),
        Some("slot `node` has no installed provider")
    );
    assert_eq!(refusal.fix, ["vibe app list"]);
    assert_eq!(
        refusal.see.as_deref(),
        Some("spec://org.vibevm.core/vibevm/common/PROP-059#alternatives")
    );
}

/// Reads a nested cause with parentheses of its own.
#[test]
fn refusal_chain_reads_a_nested_cause_with_parentheses_of_its_own() {
    let err = anyhow::anyhow!(
        "the alias table of registry `corp` would not stand:\n  `Bad_Name` is not a legal package name \
         (invalid package name `Bad_Name` — must be kebab-case (lowercase letters, digits, \
         and internal hyphens only) \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#pkgref; \
         fix: rename to kebab-case)) — an alias is typed exactly where a package name is typed, \
         so it obeys the same grammar: lowercase letters, digits and inner single hyphens"
    );
    let refusal = refusal_of(&err);
    assert_eq!(
        refusal.headline,
        "the alias table of registry `corp` would not stand:\n  `Bad_Name` is not a legal package \
         name — an alias is typed exactly where a package name is typed, so it obeys the same \
         grammar: lowercase letters, digits and inner single hyphens"
    );
    assert_eq!(
        refusal.why.as_deref(),
        Some(
            "invalid package name `Bad_Name` — must be kebab-case (lowercase letters, digits, \
             and internal hyphens only)"
        )
    );
    assert_eq!(refusal.fix, ["rename to kebab-case"]);
    assert_eq!(
        refusal.see.as_deref(),
        Some("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#pkgref")
    );
}

/// The `{err:#}` of `vibe install org.acme/x` against a git registry whose
/// URL does not exist, captured from vibe 1.0.12.
const RESOLVER_CHAIN: &str = "violates REQ spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#INSTALL-CONSUMER-CONSTRAINTS:   - brokengit  (file:///nonexistent/gitreg)  → not found (auth=none)\n (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#failure-discriminator; fix: check the name, or add the registry that hosts it to [[registry]]); consumer declarations: [\"./vibe.toml (explicit request): org.acme/x\"]; fix surface: reconcile the named consumers' [requires]:   - brokengit  (file:///nonexistent/gitreg)  → not found (auth=none)\n (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#failure-discriminator; fix: check the name, or add the registry that hosts it to [[registry]])";

/// Reads a req lead with a tail and drops the repeat.
#[test]
fn refusal_chain_reads_a_req_lead_with_a_tail_and_drops_the_repeat() {
    let refusal = refusal_of(&anyhow::anyhow!(RESOLVER_CHAIN));
    assert_eq!(refusal.code, None);
    assert_eq!(
        refusal.headline,
        "- brokengit  (file:///nonexistent/gitreg)  → not found (auth=none)"
    );
    assert_eq!(
        refusal.why.as_deref(),
        Some("consumer declarations: [\"./vibe.toml (explicit request): org.acme/x\"]")
    );
    assert_eq!(
        refusal.fix,
        ["check the name, or add the registry that hosts it to [[registry]]"]
    );
    assert_eq!(
        refusal.see.as_deref(),
        Some("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#failure-discriminator")
    );
}

/// Reads nested req leads and fix surfaces.
#[test]
fn refusal_chain_reads_nested_req_leads_and_fix_surfaces() {
    let err = anyhow::anyhow!(
        "violates REQ spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#INSTALL-CONSUMER-CONSTRAINTS: violates REQ spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#solver: source metadata is incomplete for [\"org.acme/x\"]; no unsatisfiability proof is claimed; fix surface: admitted registry observations; consumer declarations: [\"./vibe.toml (explicit request): org.acme/x\"]; fix surface: reconcile the named consumers' [requires]: violates REQ spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#solver: source metadata is incomplete for [\"org.acme/x\"]; no unsatisfiability proof is claimed; fix surface: admitted registry observations"
    );
    let refusal = refusal_of(&err);
    assert_eq!(
        refusal.headline,
        "source metadata is incomplete for [\"org.acme/x\"]"
    );
    assert_eq!(
        refusal.why.as_deref(),
        Some(
            "no unsatisfiability proof is claimed; consumer declarations: \
             [\"./vibe.toml (explicit request): org.acme/x\"]"
        )
    );
    assert_eq!(refusal.fix, ["admitted registry observations"]);
    assert_eq!(
        refusal.see.as_deref(),
        Some("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#solver")
    );
}

/// Reads a fix surface when nothing else fixes.
#[test]
fn refusal_chain_reads_a_fix_surface_when_nothing_else_fixes() {
    let err = anyhow::anyhow!(
        "violates REQ spec://org.vibevm.core/vibevm/common/PROP-059#declaration: source/prebuilt \
         requires --app-source-url; fix surface: init --app-* authoring inputs"
    );
    let refusal = refusal_of(&err);
    assert_eq!(
        refusal.headline,
        "source/prebuilt requires --app-source-url"
    );
    assert_eq!(refusal.why, None);
    assert_eq!(refusal.fix, ["init --app-* authoring inputs"]);
    assert_eq!(
        refusal.see.as_deref(),
        Some("spec://org.vibevm.core/vibevm/common/PROP-059#declaration")
    );
}

/// Reads an embedded code as the why.
#[test]
fn refusal_chain_reads_an_embedded_code_as_the_why() {
    let err = anyhow::anyhow!(
        "the version domain of `org.vibevm.node/node` could not be read: template_cache_missing: \
         `org.vibevm.node/node` has no cached version list at `/x/versions.json` and the offline \
         posture forbids fetching it \
         (violates spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-CACHE; \
          fix: run `vibe app versions org.vibevm.node/node` once with network access, then retry offline) \
         (violates spec://org.vibevm.core/vibevm/common/PROP-075#VERSIONS-LIST; \
         fix: correct the registry configuration, or run `vibe app versions --installed` for what \
         this home holds)"
    );
    let refusal = refusal_of(&err);
    assert_eq!(
        refusal.headline,
        "the version domain of `org.vibevm.node/node` could not be read"
    );
    assert_eq!(
        refusal.why.as_deref(),
        Some(
            "template_cache_missing: `org.vibevm.node/node` has no cached version list at \
             `/x/versions.json` and the offline posture forbids fetching it"
        )
    );
    assert_eq!(
        refusal.fix,
        [
            "vibe app versions --installed",
            "vibe app versions org.vibevm.node/node"
        ]
    );
    assert_eq!(
        refusal.see.as_deref(),
        Some("spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-CACHE")
    );
}

/// Strips req from see and reads a fix after the tail.
#[test]
fn refusal_chain_strips_req_from_see_and_reads_a_fix_after_the_tail() {
    let err = anyhow::anyhow!(
        "putting `/s` on the durable PATH at /p failed: denied \
         (violates REQ spec://org.vibevm.core/vibevm/common/PROP-019#path); fix: make /p writable \
         by this user and re-run, or add `/s` to PATH yourself"
    );
    let refusal = refusal_of(&err);
    assert_eq!(
        refusal.headline,
        "putting `/s` on the durable PATH at /p failed: denied"
    );
    assert_eq!(refusal.why, None);
    assert_eq!(
        refusal.fix,
        ["make /p writable by this user and re-run, or add /s to PATH yourself"]
    );
    assert_eq!(
        refusal.see.as_deref(),
        Some("spec://org.vibevm.core/vibevm/common/PROP-019#path")
    );
}

/// Keeps only the commands of a fix that names them.
#[test]
fn refusal_chain_keeps_only_the_commands_of_a_fix_that_names_them() {
    let usage = anyhow::anyhow!(
        "dl_usage: a destination is required \
         (violates spec://org.vibevm.core/vibevm/common/PROP-068#dl; \
         fix: run `vibe dl <url> <dest>`, or print with `--show` / `--headers`)"
    );
    assert_eq!(refusal_of(&usage).fix, ["vibe dl <url> <dest>"]);
    let unknown = anyhow::anyhow!(
        "agent `myag` is neither defined in agents.toml nor a built-in preset; known:  \
         (violates spec://org.vibevm.core/vibevm/common/PROP-064#selection; fix: name a listed \
         agent, see `vibe agent list` and `vibe agent presets`)"
    );
    assert_eq!(
        refusal_of(&unknown).fix,
        ["vibe agent list", "vibe agent presets"]
    );
}

/// Quiet line names the code once.
#[test]
fn refusal_chain_quiet_line_names_the_code_once() {
    let usage = anyhow::anyhow!(
        "dl_usage: a destination is required \
         (violates spec://org.vibevm.core/vibevm/common/PROP-068#dl; fix: run `vibe dl <url> <dest>`)"
    );
    assert_eq!(
        super::quiet_line(&usage, None),
        "✗ dl_usage  a destination is required \
         (violates spec://org.vibevm.core/vibevm/common/PROP-068#dl; fix: run `vibe dl <url> <dest>`)"
    );
    assert_eq!(
        super::quiet_line(&anyhow::anyhow!("plain"), Some(" · trace t")),
        "✗ plain · trace t"
    );
}

/// Wraps values under the label column.
#[test]
fn refusal_chain_wraps_values_under_the_label_column() {
    let refusal = Refusal {
        code: Some("invalid_alias".into()),
        headline: "the alias table of registry `corp` would not stand:\n  `Bad_Name` is not a \
                   legal package name"
            .into(),
        why: Some(
            "invalid package name `Bad_Name` — must be kebab-case (lowercase letters, digits, \
             and internal hyphens only)"
                .into(),
        ),
        fix: vec!["vibe registry alias corp add bad-name org.x/y".into()],
        see: Some("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#pkgref".into()),
    };
    assert_eq!(
        refusal.block().render_at(60),
        "✗ invalid_alias  the alias table of registry `corp` would\n\
         \x20 not stand:\n\
         \x20 `Bad_Name` is not a legal package name\n\
         \x20 why   invalid package name `Bad_Name` — must be kebab-case\n\
         \x20       (lowercase letters, digits, and internal hyphens\n\
         \x20       only)\n\
         \x20 fix   vibe registry alias corp add bad-name org.x/y\n\
         \x20 see   spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#pkgref"
    );
}
