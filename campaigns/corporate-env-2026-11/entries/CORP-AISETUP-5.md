# CORP-AISETUP-5

Planned at commit `9d8428d07`. Line numbers are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** Connection creation in setup shows the profile's key hint and proposes its default endpoint

**Write.** `crates/vibe-cli/src/commands/ai/setup/onboarding.rs`, `crates/vibe-cli/tests/cli_ai_setup_attended.rs`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-cli --test cli_ai_setup_attended attended_setup_shows_profile_key_hint_and_proposes_profile_endpoint`

**Test.** crates/vibe-cli/tests/cli_ai_setup_attended.rs::attended_setup_shows_profile_key_hint_and_proposes_profile_endpoint. With <settings>/site-profile.toml carrying [ai], the creation form prints 'Get an API key: Example AI Hub (https://ai-hub.example.com/keys)'. Pressing Enter at 'Endpoint URL' stores the profile's default_endpoint as the new connection's endpoint, and the key never appears in the transcript.

**Read first.**
- crates/vibe-cli/src/commands/ai/setup/onboarding.rs (create)
- crates/vibe-core/src/site_profile.rs (installed, InstalledSiteProfile)
- crates/vibe-core/src/site_profile/ai.rs (AiHints)
- crates/vibe-cli/tests/cli_ai_setup_attended.rs

**Steps.**
1. In onboarding.rs `create`, first line, add:
```rust
    let hints = vibe_core::site_profile::installed()?
        .map(|site| site.profile.ai)
        .unwrap_or_default();
```
Change the endpoint prompt to `output::prompts::text("Endpoint URL", hints.default_endpoint.as_deref().unwrap_or(""), false)?`. Directly before the `output::prompts::secret("API key (hidden)")` call, add:
```rust
    if let Some(hint) = &hints.key_hint_label {
        match &hints.key_hint_url {
            Some(url) => println!("Get an API key: {hint} ({url})"),
            None => println!("Get an API key: {hint}"),
        }
    }
```
2. Add the test:
```rust
#[test]
fn attended_setup_shows_profile_key_hint_and_proposes_profile_endpoint() {
    let w = World::new();
    fs::write(
        w.path("settings/site-profile.toml"),
        "schema = 1\n\n[ai]\nkey_hint_label = \"Example AI Hub\"\nkey_hint_url = \"https://ai-hub.example.com/keys\"\ndefault_endpoint = \"http://127.0.0.1:32125/v1\"\n",
    )
    .unwrap();
    let report = w.setup(&[
        ("Install AI agent packages now?", "n"),
        ("Create a default AI connection now?", "y"),
        ("Connection label", "\r"),
        ("Endpoint URL", "\r"),
        ("API kind", "\r"),
        ("Default model", "synthetic\r"),
        ("API key (hidden)", "SYNTHETIC_PROFILE_KEY_NEVER_OUTPUT\r"),
        ("native settings", "n"),
        ("Configure existing connection", "\u{1b}"),
    ]);
    let transcript = report["transcript"].as_str().unwrap();
    assert_eq!(report["status"], 0, "{transcript}");
    assert_eq!(report["sent"], 9, "{transcript}");
    assert!(transcript.contains("Get an API key: Example AI Hub (https://ai-hub.example.com/keys)"), "{transcript}");
    assert!(!transcript.contains("SYNTHETIC_PROFILE_KEY_NEVER_OUTPUT"));
    let connections = w.json(&["ai", "connections", "list"], None);
    let (_, connection) = connections["items"].as_object().unwrap().iter().next().unwrap();
    assert_eq!(connection["metadata"]["endpoint"], "http://127.0.0.1:32125/v1");
}
```
3. Run `cargo fmt --all`, `cargo clippy -p vibe-cli --all-targets -- -D warnings` and `cargo xtask conform check`, then the check. Then run the required second command of this entry, the whole-file regression run: `bash tools/cargo-test-ran.sh -p vibe-cli --test cli_ai_setup_attended`.

**Decisions already made.**
- Hints come only from the applied site profile layer, `vibe_core::site_profile::installed()`. An invalid stored profile propagates its named error, matching every other owner. No profile, or a profile without [ai], behaves exactly as before.
- The hint is printed as plain text immediately before the hidden key prompt: `Get an API key: <label>`, or `Get an API key: <label> (<url>)`. No browser is opened.
- default_endpoint is only the prompt's default; typed input overrides it.
- The test writes <settings>/site-profile.toml directly. `vibe self apply` is not needed, because installed() reads that file.
- The whole-file regression run is the entry's required second command; it replaces the earlier two overlapping filtered reruns.

**Not in this entry.** Not in this entry: a default model or label from the profile, opening URLs, `self apply` changes, or real corporate values.

**Depends on.** CORP-AISETUP-3, CORP-AISETUP-4

**Spec edit (apply as a step of this entry).**

none

Entries that write the same file run one after another, never in parallel.
