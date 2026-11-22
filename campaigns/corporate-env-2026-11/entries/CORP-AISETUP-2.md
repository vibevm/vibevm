# CORP-AISETUP-2

Planned at commit `9d8428d07`. Line numbers are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** Attended setup with no default connection offers to create one with a hidden key and selects it

**Write.** `crates/vibe-cli/src/commands/ai/setup.rs`, `crates/vibe-cli/src/commands/ai/setup/onboarding.rs`, `crates/vibe-cli/tests/cli_ai_setup_attended.rs`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-cli --test cli_ai_setup_attended attended_setup_without_default`

**Test.** crates/vibe-cli/tests/cli_ai_setup_attended.rs::attended_setup_without_default_creates_and_selects_connection and ::attended_setup_without_default_declined_writes_nothing. Accepting creates one keyring item (kind api-key), one connection with the typed label/endpoint/kind/model, and selects it as default, and the key never appears in the transcript. Declining prints the skip line, and connections and secrets stay empty.

**Read first.**
- crates/vibe-cli/src/commands/ai/setup.rs
- crates/vibe-cli/src/output/prompts/ask.rs (text, choose_opt_interruptible, secret, confirm)
- crates/vibe-connections/src/lib.rs (Metadata, Metadata::validate, Store::add, Store::default_selection, Store::set_default, Store::list)
- crates/vibe-secrets/src/lib.rs (Store::at_settings, Store::create, ProtectedValue::new)
- crates/vibe-cli/tests/cli_ai_setup_attended.rs

**Steps.**
1. In setup.rs add `mod onboarding;`. Inside the `if bare {` block, directly after `install::offer(cancel)?;`, insert:
```rust
        if matches!(
            onboarding::ensure_default(settings, store, cancel)?,
            onboarding::Next::Stop
        ) {
            return Ok(());
        }
```
2. Create crates/vibe-cli/src/commands/ai/setup/onboarding.rs with exactly:
```rust
//! Connection onboarding inside attended `vibe ai setup`: a usable default
//! connection exists before agents are configured. The key is read only
//! through the protected hidden prompt and goes straight into the keyring.
use crate::output;
use anyhow::{Result, anyhow, bail};
use std::path::Path;
use vibe_core::cancellation::CancellationToken;
use vibe_secrets::SecretId;

/// Whether attended setup continues after onboarding.
pub(super) enum Next {
    Continue,
    Stop,
}

const SKIPPED: &str = "AI setup skipped; connections and bindings unchanged.";

/// With no default connection, offer (default No) to create one and select it.
pub(super) fn ensure_default(
    settings: &Path,
    store: &vibe_connections::Store,
    cancel: &CancellationToken,
) -> Result<Next> {
    if store.default_selection()?.connection.is_some() {
        return Ok(Next::Continue);
    }
    println!("No default AI connection exists.");
    if !output::prompts::confirm("Create a default AI connection now?", false)? {
        println!("{SKIPPED}");
        return Ok(Next::Stop);
    }
    let Some((id, metadata)) = create(settings, store, cancel)? else {
        println!("{SKIPPED}");
        return Ok(Next::Stop);
    };
    store.set_default(&id, store.list()?.store_revision)?;
    println!(
        "Default AI connection created: {} — {} — {}",
        metadata.label,
        id.as_str(),
        metadata.endpoint
    );
    Ok(Next::Continue)
}

/// Ask for a new OpenAI-compatible connection and store it; `None` when the
/// API-kind menu is escaped. Metadata is validated before the key is read.
fn create(
    settings: &Path,
    store: &vibe_connections::Store,
    cancel: &CancellationToken,
) -> Result<Option<(SecretId, vibe_connections::Metadata)>> {
    let label = output::prompts::text("Connection label", "Default", false)?;
    let endpoint = output::prompts::text("Endpoint URL", "", false)?;
    let kinds = ["openai-chat-completions", "openai-responses"];
    let Some(kind) =
        output::prompts::choose_opt_interruptible("API kind", &output::prompts::named(&kinds), 0)?
    else {
        return Ok(None);
    };
    let default_model = output::prompts::text("Default model", "", false)?;
    let metadata = vibe_connections::Metadata {
        label,
        endpoint,
        api_kind: kinds[kind].to_owned(),
        default_model,
    };
    if metadata.validate().is_err() {
        bail!("connection metadata is invalid; nothing was written");
    }
    cancel.checkpoint()?;
    let key = output::prompts::secret("API key (hidden)")
        .map_err(|_| anyhow!("protected input cancelled or unavailable; nothing was written"))?;
    let value = vibe_secrets::ProtectedValue::new(key.into_bytes())?;
    let secret =
        vibe_secrets::Store::at_settings(settings)?.create("api-key", &metadata.label, value)?;
    let id = store.add(metadata.clone(), &secret)?;
    Ok(Some((id, metadata)))
}
```
3. In crates/vibe-cli/tests/cli_ai_setup_attended.rs, update the CORP-AISETUP-1 test `attended_setup_installs_selected_agent_packages_with_independent_outcomes` for the new prompt. A fresh world has no default connection, so onboarding asks after the installs. Replace the two tuples `("native settings", "n"),` and `("Configure existing connection", "\u{1b}"),` with the single tuple `("Create a default AI connection now?", "n"),`, and change `assert_eq!(report["sent"], 6, "{transcript}");` to `assert_eq!(report["sent"], 5, "{transcript}");`. Keep every other assertion of that test unchanged.
4. Add the two tests:
```rust
const KEY: &str = "SYNTHETIC_ONBOARDING_KEY_NEVER_OUTPUT";

#[test]
fn attended_setup_without_default_creates_and_selects_connection() {
    let w = World::new();
    let key_line = format!("{KEY}\r");
    let report = w.setup(&[
        ("Install AI agent packages now?", "n"),
        ("Create a default AI connection now?", "y"),
        ("Connection label", "\r"),
        ("Endpoint URL", "http://127.0.0.1:32123/v1\r"),
        ("API kind", "\r"),
        ("Default model", "synthetic\r"),
        ("API key (hidden)", key_line.as_str()),
        ("native settings", "n"),
        ("Configure existing connection", "\u{1b}"),
    ]);
    let transcript = report["transcript"].as_str().unwrap();
    assert_eq!(report["status"], 0, "{transcript}");
    assert_eq!(report["sent"], 9, "{transcript}");
    assert!(transcript.contains("Default AI connection created: Default"), "{transcript}");
    assert!(!transcript.contains(KEY));
    let connections = w.json(&["ai", "connections", "list"], None);
    let items = connections["items"].as_object().unwrap();
    assert_eq!(items.len(), 1);
    let (id, connection) = items.iter().next().unwrap();
    assert_eq!(connection["metadata"]["label"], "Default");
    assert_eq!(connection["metadata"]["endpoint"], "http://127.0.0.1:32123/v1");
    assert_eq!(connection["metadata"]["api_kind"], "openai-chat-completions");
    assert_eq!(connection["metadata"]["default_model"], "synthetic");
    assert_eq!(w.json(&["ai", "connections", "default", "show"], None)["connection"], id.as_str());
    let secrets = w.json(&["secrets", "list"], None);
    assert_eq!(secrets["items"].as_array().unwrap().len(), 1);
    assert_eq!(secrets["items"][0]["kind"], "api-key");
}

#[test]
fn attended_setup_without_default_declined_writes_nothing() {
    let w = World::new();
    let report = w.setup(&[
        ("Install AI agent packages now?", "n"),
        ("Create a default AI connection now?", "n"),
    ]);
    let transcript = report["transcript"].as_str().unwrap();
    assert_eq!(report["status"], 0, "{transcript}");
    assert!(transcript.contains("AI setup skipped; connections and bindings unchanged."), "{transcript}");
    assert_eq!(w.json(&["ai", "connections", "list"], None)["items"], serde_json::json!({}));
    assert_eq!(w.json(&["secrets", "list"], None)["items"], serde_json::json!([]));
}
```
5. Run `cargo fmt --all`, then `cargo clippy -p vibe-cli --all-targets -- -D warnings` (fix without `allow`) and `cargo xtask conform check`. Run the check. Then run the required second command of this entry, the whole-file regression run: `bash tools/cargo-test-ran.sh -p vibe-cli --test cli_ai_setup_attended`.

**Decisions already made.**
- Onboarding runs in the bare attended branch after the install step and before the native-settings question.
- The trigger is `store.default_selection()?.connection` being None, even when other non-default connections exist.
- Only the two OpenAI API kinds are offered (`openai-chat-completions`, `openai-responses`); `anthropic-messages-bearer` is not offered, because Claude is excluded from OpenAI key setup.
- Metadata is validated before the key prompt, so an invalid endpoint, label or model refuses with 'connection metadata is invalid; nothing was written' and leaves no keyring item.
- The key is read only through output::prompts::secret and stored with keyring kind `api-key`, labelled with the connection label. It is never printed, logged or placed in argv.
- Declining creation, or escaping the API-kind menu, prints 'AI setup skipped; connections and bindings unchanged.' and ends setup with exit 0.
- If `store.add` fails after the keyring item was created, the error propagates and the item stays. There is no rollback.
- The edit to the CORP-AISETUP-1 test is part of this entry's required behaviour change: bare setup on a fresh world now stops after a declined onboarding question, so the native-settings and metadata prompts are no longer reachable there. It replaces unreachable prompt steps and does not weaken the test; its 'pi: installed', 'qwen: failed' and app-list assertions stay. Without the edit that test waits for its 90-second driver limit and then fails.
- The whole-file regression run is the entry's second required command, not wider work: it is the only command that covers the edited CORP-AISETUP-1 test.
- List-level precondition: tools/cargo-test-ran.sh is committed by the lead before CORP-AISETUP-1.

**Not in this entry.** Not in this entry: showing or reusing an existing default, the replace/scope choice, site-profile hints or default endpoint, a scriptable non-interactive creation flag, or picking an existing non-default connection as the default.

**Depends on.** CORP-AISETUP-1

**Spec edit (apply as a step of this entry).**

none

Entries that write the same file run one after another, never in parallel.
