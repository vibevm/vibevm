# CORP-AISETUP-4

Planned at commit `9d8428d07`. Line numbers are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** Site profile accepts an optional [ai] table with key hint and default endpoint

**Write.** `crates/vibe-core/src/site_profile.rs`, `crates/vibe-core/src/site_profile/ai.rs`, `crates/vibe-core/src/site_profile/tests.rs`, `vibevm/vibespecs/common/PROP-019-version-manager.xml`, `specmap.json`, `campaigns/corporate-env-2026-11/FORMAT-CHANGES.md`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-core --lib site_profile::tests::ai_table_carries_hint_and_endpoint_and_refuses_bad_values`

**Test.** crates/vibe-core/src/site_profile/tests.rs::ai_table_carries_hint_and_endpoint_and_refuses_bad_values. A profile with [ai] key_hint_label, key_hint_url and default_endpoint parses into profile.ai. A profile without [ai] has AiHints::default() and serializes without `[ai]`. Each bad value is refused under its own key: key_hint_url without a label, a blank label, a plain-http non-loopback or credentialed default_endpoint, and an unknown key.

**Read first.**
- crates/vibe-core/src/site_profile.rs (SiteProfile, SiteProfileError, SiteProfile::validate, module declarations at the end)
- crates/vibe-core/src/site_profile/windows_paths.rs (child-module pattern)
- crates/vibe-core/src/site_profile/tests.rs (corp, refused)
- vibevm/vibespecs/common/PROP-019-version-manager.xml (SITE-PROFILE-LEAD)
- campaigns/corporate-env-2026-11/FORMAT-CHANGES.md (title line and the `## CORP-FMT-066` section below it)

**Steps.**
1. Check `wc -l crates/vibe-core/src/site_profile.rs` (593 at planning time); the file must end at 600 lines or fewer. Change it only as follows. (a) After the `pub trust: TrustLists,` field of SiteProfile, add
```rust
    /// `[ai]` — the optional API-key hint and default AI endpoint (`ai.rs`).
    #[serde(default, skip_serializing_if = "AiHints::is_empty")]
    pub ai: AiHints,
```
(b) In `fn validate`, replace its final `        Ok(())` (directly after the `[trust]` loop) with `        self.ai.validate(origin)`. (c) In the SiteProfileError::Parse message, change `[path], [trust])"` to `[path], [trust], [ai])"`. (d) After the `#[path = "site_profile/windows_paths.rs"] mod windows_paths;` declaration, add
```rust
#[path = "site_profile/ai.rs"]
mod ai;
pub use ai::AiHints;
```
2. Create crates/vibe-core/src/site_profile/ai.rs with:
```rust
//! `[ai]` of a site profile (`PROP-019#site-profile`): where a developer
//! gets an API key and which endpoint `vibe ai setup` proposes. Never a key.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#site-profile");

use serde::{Deserialize, Serialize};

use super::SiteProfileError;

/// `[ai]` — an optional API-key acquisition hint and default endpoint.
///
/// ```
/// use vibe_core::site_profile::AiHints;
/// let ai: AiHints = toml::from_str(
///     "key_hint_label = 'Example AI Hub'\ndefault_endpoint = 'https://ai.example.com/v1'",
/// )
/// .unwrap();
/// assert_eq!(ai.key_hint_label.as_deref(), Some("Example AI Hub"));
/// assert_eq!(ai.key_hint_url, None);
/// ```
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AiHints {
    /// Where to get a key, as plain text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_hint_label: Option<String>,
    /// An optional page for that hint; never opened automatically.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_hint_url: Option<String>,
    /// The endpoint `vibe ai setup` proposes for a new connection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_endpoint: Option<String>,
}

impl AiHints {
    pub(super) fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    pub(super) fn validate(&self, origin: &str) -> Result<(), SiteProfileError> {
        let invalid = |key: &str, reason: &str, fix: &str| SiteProfileError::Invalid {
            origin: origin.to_owned(),
            key: key.to_owned(),
            reason: reason.to_owned(),
            fix: fix.to_owned(),
        };
        if let Some(label) = &self.key_hint_label
            && (label.trim().is_empty() || label.len() > 256 || label.chars().any(char::is_control))
        {
            return Err(invalid(
                "ai.key_hint_label",
                "it is empty, longer than 256 bytes or has control characters",
                "write a short plain-text hint",
            ));
        }
        if let Some(url) = &self.key_hint_url {
            if self.key_hint_label.is_none() {
                return Err(invalid(
                    "ai.key_hint_url",
                    "a hint URL needs `key_hint_label`",
                    "add `key_hint_label` or drop the URL",
                ));
            }
            if !web_url(url, false) {
                return Err(invalid(
                    "ai.key_hint_url",
                    "it is not an http or https URL without credentials",
                    "give a plain https page address",
                ));
            }
        }
        if let Some(endpoint) = &self.default_endpoint
            && !web_url(endpoint, true)
        {
            return Err(invalid(
                "ai.default_endpoint",
                "it is not an https URL (or http on a loopback address) without credentials, query or fragment",
                "give the endpoint as `vibe ai connections add --endpoint` accepts it",
            ));
        }
        Ok(())
    }
}

/// An `http(s)` URL without credentials; an `endpoint` also refuses a query,
/// a fragment and plain `http` off the loopback address.
fn web_url(text: &str, endpoint: bool) -> bool {
    let Ok(url) = url::Url::parse(text) else {
        return false;
    };
    if !url.username().is_empty() || url.password().is_some() || url.host_str().is_none() {
        return false;
    }
    if !endpoint {
        return matches!(url.scheme(), "https" | "http");
    }
    let local = match url.host() {
        Some(url::Host::Ipv4(ip)) => ip.is_loopback(),
        Some(url::Host::Ipv6(ip)) => ip.is_loopback(),
        _ => false,
    };
    url.query().is_none()
        && url.fragment().is_none()
        && (url.scheme() == "https" || (url.scheme() == "http" && local))
}
```
3. Append to crates/vibe-core/src/site_profile/tests.rs:
```rust
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-019#site-profile")]
fn ai_table_carries_hint_and_endpoint_and_refuses_bad_values() {
    let profile = SiteProfile::parse(
        "schema = 1\n[ai]\nkey_hint_label = 'Example AI Hub'\n\
         key_hint_url = 'https://ai-hub.example.com/keys'\n\
         default_endpoint = 'http://127.0.0.1:32125/v1'\n",
        "corp.toml",
    )
    .unwrap();
    assert_eq!(profile.ai.key_hint_label.as_deref(), Some("Example AI Hub"));
    assert_eq!(profile.ai.key_hint_url.as_deref(), Some("https://ai-hub.example.com/keys"));
    assert_eq!(profile.ai.default_endpoint.as_deref(), Some("http://127.0.0.1:32125/v1"));
    assert_eq!(corp().ai, AiHints::default());
    assert!(!toml::to_string(&corp()).unwrap().contains("[ai]"));
    assert!(refused("schema = 1\n[ai]\nkey_hint_url = 'https://ai-hub.example.com/keys'\n").contains("ai.key_hint_url"));
    assert!(refused("schema = 1\n[ai]\nkey_hint_label = ' '\n").contains("ai.key_hint_label"));
    assert!(refused("schema = 1\n[ai]\ndefault_endpoint = 'http://ai.example.com/v1'\n").contains("ai.default_endpoint"));
    assert!(refused("schema = 1\n[ai]\ndefault_endpoint = 'https://user:pw@ai.example.com/v1'\n").contains("ai.default_endpoint"));
    assert!(SiteProfile::parse("schema = 1\n[ai]\nkey = 'x'\n", "corp.toml").is_err());
}
```
4. Apply the spec edit (see spec_edit) to vibevm/vibespecs/common/PROP-019-version-manager.xml, then run `cargo xtask specmap` to regenerate specmap.json and `cargo xtask specmap --check` to confirm it is fresh.
5. In campaigns/corporate-env-2026-11/FORMAT-CHANGES.md (newest section first), insert this section directly after the title line `# Corporate-env campaign: format and behaviour changes` and its blank line, above `## CORP-FMT-066 — explicit atomic default scope`. The block ends with one blank line, so a blank line separates it from the 066 heading:
```
## CORP-FMT-067 — site profile `[ai]` hint and default endpoint

The site profile (`schema = 1`) accepts an optional `[ai]` table with `key_hint_label`, `key_hint_url` and `default_endpoint`. A profile without `[ai]` serializes unchanged. An older vibe refuses a profile carrying `[ai]` as an unknown key, so update the binary before applying such a profile. `key_hint_url` requires `key_hint_label`; `default_endpoint` follows the connection endpoint rules (https, or http on a loopback address, with no credentials, query or fragment). The table never carries a key.

```
6. Run `cargo fmt --all`, `cargo clippy -p vibe-core --all-targets -- -D warnings`, `cargo test -p vibe-core --doc site_profile` and `cargo xtask conform check`, and confirm with `wc -l` that site_profile.rs is 600 lines or fewer. Then run the check.

**Decisions already made.**
- Table name `[ai]`, with fields key_hint_label, key_hint_url and default_endpoint, all optional strings. The type is `vibe_core::site_profile::AiHints`, re-exported from the site_profile module.
- The schema number stays 1. A profile without [ai] serializes byte-identically (skip_serializing_if).
- key_hint_url without key_hint_label is refused. key_hint_url accepts http or https without credentials. default_endpoint follows the vibe-connections Metadata endpoint rules (https, or http only on loopback; no userinfo, query or fragment). This is reimplemented in vibe-core because vibe-core cannot depend on vibe-connections.
- Validation lives in the child module so site_profile.rs grows by 6 lines only. The existing SiteProfileError::Invalid is reused, with keys `ai.key_hint_label`, `ai.key_hint_url` and `ai.default_endpoint`.
- No real or private values: tests use example.com and loopback only.
- FORMAT-CHANGES.md is ordered newest first for 057 onward (066 sits directly under the title; 001 to 056 follow below 057), so 067 goes directly under the title.
- Hidden prerequisite: a spec text change shifts content hashes and line numbers, so specmap.json must be regenerated with `cargo xtask specmap` in the same change.
- List-level precondition: tools/cargo-test-ran.sh is committed by the lead before the list runs.

**Not in this entry.** Not in this entry: reading the hints in `vibe ai setup`, `self apply --show` rendering of [ai], kit-builder recipe templates, or any real corporate value.

**Spec edit (apply as a step of this entry).**

In vibevm/vibespecs/common/PROP-019-version-manager.xml, unit SITE-PROFILE-LEAD, replace the two lines
```
"ask" | "always" | "never"`, `[versions] select` (PROP-075 `#VERSIONS-SELECT`, the installation's
version-selection policy) and `[trust]`; any other key is refused. It is stored verbatim as
```
with
```
"ask" | "always" | "never"`, `[versions] select` (PROP-075 `#VERSIONS-SELECT`, the installation's
version-selection policy), `[trust]` and `[ai]` (`key_hint_label`, `key_hint_url` and
`default_endpoint`: where a developer gets an API key and the endpoint `vibe ai setup` proposes
for a new connection, never a key); any other key is refused. It is stored verbatim as
```

Entries that write the same file run one after another, never in parallel.
