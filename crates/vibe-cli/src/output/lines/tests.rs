use super::refusal_of;

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
            "vibe remap accept org.corp/intranet, or vibe remap reject org.corp/intranet; \
             vibe remap list shows every row involved",
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
    assert_eq!(
        refusal.fix,
        [
            "act on the cause",
            "use -g for the user home, or check vibe app list for the installed keys",
            "vibe app list"
        ]
    );
}

#[test]
fn a_plain_error_is_its_own_headline() {
    let err = anyhow::anyhow!("inner").context("outer");
    let refusal = refusal_of(&err);
    assert_eq!(refusal.code, None);
    assert_eq!(refusal.headline, "outer: inner");
    assert!(refusal.fix.is_empty() && refusal.see.is_none());
}
