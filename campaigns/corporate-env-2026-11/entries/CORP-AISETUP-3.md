# CORP-AISETUP-3

Planned at commit `9d8428d07`. Line numbers are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** Attended setup shows the existing default, reuses it by default, or replaces it under a reviewed scope

**Write.** `crates/vibe-cli/src/commands/ai/setup.rs`, `crates/vibe-cli/src/commands/ai/setup/onboarding.rs`, `crates/vibe-cli/tests/cli_ai_setup_attended.rs`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-cli --test cli_ai_setup_attended attended_setup_with_default`

**Test.** crates/vibe-cli/tests/cli_ai_setup_attended.rs::attended_setup_with_default_reuses_it_without_writes and ::attended_setup_with_default_replaced_default_only_keeps_explicit_qwen. Reuse (Enter) leaves the connection store revision and the default unchanged. Declining reuse, confirming replacement, entering a new connection, choosing 'Change only the default connection' and Apply makes the new connection the default. Qwen stays explicitly bound to the old connection, Pi stays inherit-default, and the new key never appears in the transcript.

**Read first.**
- crates/vibe-cli/src/commands/ai/setup/onboarding.rs
- crates/vibe-cli/src/commands/ai/setup.rs (default_scope)
- crates/vibe-connections/src/default_scope.rs (DefaultScope)
- crates/vibe-connections/src/lib.rs (Store::details)
- crates/vibe-cli/tests/cli_ai_setup_attended.rs

**Steps.**
1. In onboarding.rs, add `ctx: &output::Context` as the first parameter of `ensure_default`. In setup.rs change the call to `onboarding::ensure_default(ctx, settings, store, cancel)?`. In `ensure_default`, replace
```rust
    if store.default_selection()?.connection.is_some() {
        return Ok(Next::Continue);
    }
```
with
```rust
    if let Some(current) = store.default_selection()?.connection {
        return reuse_or_replace(ctx, settings, store, &current, cancel);
    }
```
2. Add to onboarding.rs:
```rust
/// Show the default (never its key) and keep it, or replace it with a new
/// connection under an explicit scope reviewed by the default-scope preview.
fn reuse_or_replace(
    ctx: &output::Context,
    settings: &Path,
    store: &vibe_connections::Store,
    current: &SecretId,
    cancel: &CancellationToken,
) -> Result<Next> {
    let metadata = store.details(current)?.metadata;
    println!(
        "Default AI connection: {} — {} — {}",
        metadata.label,
        current.as_str(),
        metadata.endpoint
    );
    if output::prompts::confirm("Reuse this default connection?", true)? {
        return Ok(Next::Continue);
    }
    if !output::prompts::confirm(
        "Replace the default connection? Agents that inherit the default will use the new connection",
        false,
    )? {
        println!("Default connection unchanged.");
        return Ok(Next::Continue);
    }
    let Some((id, _)) = create(settings, store, cancel)? else {
        println!("{SKIPPED}");
        return Ok(Next::Stop);
    };
    let scopes = [
        "Change only the default connection",
        "Replace all Vibe-managed agent overrides with this connection",
    ];
    let scope = match output::prompts::choose_opt_interruptible(
        "Default update scope",
        &output::prompts::named(&scopes),
        0,
    )? {
        Some(0) => vibe_connections::DefaultScope::DefaultOnly,
        Some(_) => vibe_connections::DefaultScope::ManagedOverrides,
        None => {
            println!(
                "Default unchanged; new connection {} kept as a non-default connection.",
                id.as_str()
            );
            return Ok(Next::Continue);
        }
    };
    super::default_scope(ctx, store, &id, scope, None, cancel)?;
    Ok(Next::Continue)
}
```
3. Add to the test file a seed helper and two tests:
```rust
fn seed_default(w: &World) -> String {
    let secret = w.json(
        &["secrets", "create", "--kind", "api-key", "--label", "Old", "--value-stdin"],
        Some(b"SYNTHETIC_OLD_KEY_NEVER_OUTPUT"),
    );
    let sid = secret["items"][0]["id"].as_str().unwrap().to_owned();
    let added = w.json(
        &["ai", "connections", "add", "--label", "Old", "--endpoint", "http://127.0.0.1:32123/v1", "--api-kind", "openai-chat-completions", "--default-model", "synthetic", "--secret-ref", &sid],
        None,
    );
    let id = added["items"].as_object().unwrap().keys().next().unwrap().clone();
    let revision = |w: &World| w.json(&["ai", "connections", "list"], None)["store_revision"].to_string();
    w.json(&["ai", "connections", "default", "set", &id, "--expect-store", &revision(w)], None);
    w.json(&["ai", "bindings", "set", "qwen", "--mode", "connection", "--connection", &id, "--expect-store", &revision(w)], None);
    w.json(&["ai", "bindings", "set", "pi", "--mode", "inherit-default", "--expect-store", &revision(w)], None);
    id
}

#[test]
fn attended_setup_with_default_reuses_it_without_writes() {
    let w = World::new();
    let old = seed_default(&w);
    let before = w.json(&["ai", "connections", "list"], None)["store_revision"].clone();
    let report = w.setup(&[
        ("Install AI agent packages now?", "n"),
        ("Reuse this default connection?", "\r"),
        ("native settings", "n"),
        ("Configure existing connection", "\u{1b}"),
    ]);
    let transcript = report["transcript"].as_str().unwrap();
    assert_eq!(report["status"], 0, "{transcript}");
    assert_eq!(report["sent"], 4, "{transcript}");
    assert!(transcript.contains("Default AI connection: Old"), "{transcript}");
    assert_eq!(w.json(&["ai", "connections", "list"], None)["store_revision"], before);
    assert_eq!(w.json(&["ai", "connections", "default", "show"], None)["connection"], old.as_str());
    assert_eq!(w.json(&["secrets", "list"], None)["items"].as_array().unwrap().len(), 1);
}

#[test]
fn attended_setup_with_default_replaced_default_only_keeps_explicit_qwen() {
    let w = World::new();
    let old = seed_default(&w);
    let report = w.setup(&[
        ("Install AI agent packages now?", "n"),
        ("Reuse this default connection?", "n"),
        ("Replace the default connection?", "y"),
        ("Connection label", "New\r"),
        ("Endpoint URL", "http://127.0.0.1:32124/v1\r"),
        ("API kind", "\r"),
        ("Default model", "synthetic\r"),
        ("API key (hidden)", "SYNTHETIC_REPLACEMENT_KEY_NEVER_OUTPUT\r"),
        ("Default update scope", "\r"),
        ("Review default and managed overrides", "\u{1b}[B\r"),
        ("native settings", "n"),
        ("Configure existing connection", "\u{1b}"),
    ]);
    let transcript = report["transcript"].as_str().unwrap();
    assert_eq!(report["status"], 0, "{transcript}");
    assert_eq!(report["sent"], 12, "{transcript}");
    assert!(!transcript.contains("SYNTHETIC_REPLACEMENT_KEY_NEVER_OUTPUT"));
    let new = w.json(&["ai", "connections", "default", "show"], None)["connection"].as_str().unwrap().to_owned();
    assert_ne!(new, old);
    assert_eq!(w.json(&["ai", "connections", "list"], None)["items"][&new]["metadata"]["label"], "New");
    let qwen = w.json(&["ai", "bindings", "show", "qwen"], None);
    assert_eq!(qwen["binding"]["mode"], "connection");
    assert_eq!(qwen["binding"]["connection"], old.as_str());
    assert_eq!(w.json(&["ai", "bindings", "show", "pi"], None)["binding"]["mode"], "inherit-default");
}
```
4. Run `cargo fmt --all`, `cargo clippy -p vibe-cli --all-targets -- -D warnings` (no `allow`) and `cargo xtask conform check`. Run the check. Then run the required second command of this entry, the whole-file regression run: `bash tools/cargo-test-ran.sh -p vibe-cli --test cli_ai_setup_attended`.

**Decisions already made.**
- The existing default is shown as `Default AI connection: <label> — <id> — <endpoint>`; the key is never read.
- Reuse defaults to yes and writes nothing.
- Replacement needs two explicit answers: declining reuse, then confirming 'Replace the default connection? ...' (default No). Declining the confirmation prints 'Default connection unchanged.' and continues.
- The new connection is created by the same `create` function as CORP-AISETUP-2.
- Scope choice: menu index 0 'Change only the default connection' is the default and maps to DefaultScope::DefaultOnly; index 1 maps to DefaultScope::ManagedOverrides. The existing `setup::default_scope` then shows the full per-agent preview and asks Keep or Apply. Keep or Escape there leaves the old default and keeps the new connection as non-default.
- Escaping the scope menu prints 'Default unchanged; new connection <id> kept as a non-default connection.' and continues. No connection is deleted automatically.
- Existing tests need no edit: a fresh world still has no default, so the earlier tests never reach the new reuse prompt. The whole-file regression run is required to show that.
- The metadata menu is matched by 'Configure existing connection' (see CORP-AISETUP-1).

**Not in this entry.** Not in this entry: deleting the orphaned new connection after Keep, changing default_scope's preview or wording, native credential import, or site-profile hints.

**Depends on.** CORP-AISETUP-2

**Spec edit (apply as a step of this entry).**

none

Entries that write the same file run one after another, never in parallel.
