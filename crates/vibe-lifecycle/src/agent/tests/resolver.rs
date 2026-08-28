//! Mutation-grade REDs over the real credential-free resolver.
//!
//! Every fixture below is a real tree on disk with a wrong answer planted in
//! every place a weaker resolver might look: a fresher unselected slot beside
//! the selected one, a colliding document at the workspace root, an installed
//! coordinate the lock never chose. The assertions name the exact bytes the
//! selected instance holds, so a resolver that scans, falls through or
//! rediscovers answers with the planted bytes and fails.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use vibe_spec::SelectedPackage;

use crate::agent::{PromptRequest, SelectedWorldPromptResolver};

const GROUP: &str = "org.demo";
const PROMPT: &str = "spec://org.demo/tools/common/PROMPT-001#root";

/// A materialised world: the executing provider instance at 1.0.0, a fresher
/// 2.0.0 slot installed beside it that the lock did NOT choose, a colliding
/// document in the workspace root's own authored tree, and a selected `lib`
/// package whose fresher slot is likewise unselected.
struct World {
    ws: tempfile::TempDir,
    provider_root: PathBuf,
    selected: BTreeMap<(String, String), SelectedPackage>,
}

impl World {
    fn build() -> Self {
        let ws = tempfile::tempdir().unwrap();
        let slots = |name: &str| -> [String; 2] {
            ["1.0.0".to_string(), "2.0.0".to_string()]
                .map(|version| format!("vibedeps/{GROUP}.{name}/{version}"))
        };
        // The executing provider instance (1.0.0) and its fresher sibling.
        let [provider, newer] = slots("tools");
        Self::write(
            ws.path(),
            &format!("{provider}/vibevm/vibespecs/common/PROMPT-001.md"),
            "# Prompt {#root}\n\nWrite the guide from the executing instance.\n",
        );
        Self::write(
            ws.path(),
            &format!("{newer}/vibevm/vibespecs/common/PROMPT-001.md"),
            "# Prompt {#root}\n\nWrite the guide from the newer installed instance.\n",
        );
        // The colliding document at the workspace root itself.
        Self::write(
            ws.path(),
            "vibevm/vibespecs/common/PROMPT-001.md",
            "# Prompt {#root}\n\nWrite the guide from the workspace root copy.\n",
        );
        // The selected lib (1.0.0) and its fresher unselected sibling.
        let [lib_one, lib_two] = slots("lib");
        Self::write(
            ws.path(),
            &format!("{lib_one}/vibevm/vibespecs/common/NOTE-001.md"),
            "# Note {#root}\n\nlib selected one\n",
        );
        Self::write(
            ws.path(),
            &format!("{lib_two}/vibevm/vibespecs/common/NOTE-001.md"),
            "# Note {#root}\n\nlib newer two\n",
        );
        // Installed, but never selected: reachable by no address.
        Self::write(
            ws.path(),
            "vibedeps/org.demo.ghost/1.0.0/vibevm/vibespecs/common/NOTE-001.md",
            "# Note {#root}\n\nghost bytes\n",
        );
        let mut selected = BTreeMap::new();
        selected.insert(
            (GROUP.to_string(), "tools".to_string()),
            SelectedPackage::new("1.0.0", ws.path().join(&provider)),
        );
        selected.insert(
            (GROUP.to_string(), "lib".to_string()),
            SelectedPackage::new("1.0.0", ws.path().join(&lib_one)),
        );
        let provider_root = ws.path().join(provider);
        Self {
            ws,
            provider_root,
            selected,
        }
    }

    fn write(root: &Path, rel: &str, body: &str) {
        let path = root.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, body).unwrap();
    }

    fn request(&self, address: &str) -> PromptRequest {
        self.request_with(address, self.selected.clone())
    }

    /// The same request with a different selected world, so a test can prove
    /// which arm answered.
    fn request_with(
        &self,
        address: &str,
        selected: BTreeMap<(String, String), SelectedPackage>,
    ) -> PromptRequest {
        PromptRequest {
            address: address.to_string(),
            provider_root: self.provider_root.clone(),
            provider_group: GROUP.to_string(),
            provider_name: "tools".to_string(),
            selected_world: selected,
        }
    }

    fn resolver(&self) -> SelectedWorldPromptResolver {
        SelectedWorldPromptResolver::new(self.ws.path())
    }
}

/// The self prompt answers from the executing provider instance — the
/// selected slot itself — never from the workspace root's colliding document
/// and never from the fresher installed sibling.
#[test]
fn a_self_prompt_resolves_from_the_executing_provider_instance() {
    let world = World::build();
    let resolved = world.resolver().resolve(&world.request(PROMPT)).unwrap();
    assert!(
        resolved.text.contains("executing instance"),
        "the executing instance must answer: {}",
        resolved.text
    );
    assert!(
        !resolved.text.contains("newer installed instance"),
        "a fresher installed slot must never substitute for the selected one",
    );
    assert!(
        !resolved.text.contains("workspace root copy"),
        "the workspace root's colliding document must never answer",
    );
}

/// The self arm is pinned to `provider_root`, not to the selected map: with
/// the provider's own coordinate ABSENT from the world, the prompt still
/// resolves — from the executing instance. A resolver that reached for the
/// map (or scanned) would have nothing to find.
#[test]
fn the_self_arm_answers_from_the_instance_even_unselected() {
    let world = World::build();
    let request = world.request_with(PROMPT, BTreeMap::new());
    let resolved = world.resolver().resolve(&request).unwrap();
    assert!(resolved.text.contains("executing instance"));
}

/// A workspace member's prompt cannot fall through to the colliding document
/// at the workspace root: the member node is the self root, and the root's
/// authored tree is not consulted for it.
#[test]
fn a_workspace_member_prompt_never_falls_through_to_the_root_document() {
    let ws = tempfile::tempdir().unwrap();
    let member = ws.path().join("member");
    World::write(
        &member,
        "vibevm/vibespecs/common/PROMPT-001.md",
        "# Prompt {#root}\n\nWrite the guide from the member copy.\n",
    );
    World::write(
        ws.path(),
        "vibevm/vibespecs/common/PROMPT-001.md",
        "# Prompt {#root}\n\nWrite the guide from the workspace root copy.\n",
    );
    let resolver = SelectedWorldPromptResolver::new(ws.path());
    let request = PromptRequest {
        address: PROMPT.to_string(),
        provider_root: member,
        provider_group: GROUP.to_string(),
        provider_name: "tools".to_string(),
        selected_world: BTreeMap::new(),
    };
    let resolved = resolver.resolve(&request).unwrap();
    assert!(
        resolved.text.contains("member copy") && !resolved.text.contains("root copy"),
        "the member's own document must answer: {}",
        resolved.text
    );
}

/// A cross-package `#embed` resolves through the lock-selected map only: the
/// selected 1.0.0 lib answers even with a fresher 2.0.0 sitting on disk
/// beside it, because no slot is ever scanned.
#[test]
fn a_cross_package_embed_resolves_only_the_lock_selected_version() {
    let world = World::build();
    World::write(
        &world.provider_root,
        "vibevm/vibespecs/common/PROMPT-002.md",
        "# Embedder {#root}\n\n#embed spec://org.demo/lib/common/NOTE-001#root\n",
    );
    let resolved = world
        .resolver()
        .resolve(&world.request("spec://org.demo/tools/common/PROMPT-002#root"))
        .unwrap();
    assert!(
        resolved.text.contains("lib selected one"),
        "the lock-selected instance must answer: {}",
        resolved.text
    );
    assert!(
        !resolved.text.contains("lib newer two"),
        "a fresher installed slot must never substitute for the selected one",
    );
}

/// An explicit `@version` on an embedded address is checked against the lock,
/// never dropped: a disagreeing pin refuses naming both numbers.
#[test]
fn an_embed_pin_disagreeing_with_the_lock_refuses() {
    let world = World::build();
    World::write(
        &world.provider_root,
        "vibevm/vibespecs/common/PROMPT-003.md",
        "# Pinner {#root}\n\n#embed spec://org.demo/lib@2.0.0/common/NOTE-001#root\n",
    );
    let error = world
        .resolver()
        .resolve(&world.request("spec://org.demo/tools/common/PROMPT-003#root"))
        .unwrap_err();
    assert!(
        error.contains("2.0.0") && error.contains("1.0.0"),
        "the refusal must name both the pin and the lock: {error}"
    );
}

/// A coordinate the lock did not choose is refused, never scanned for — even
/// when its slot is sitting installed on disk.
#[test]
fn an_embed_into_an_unselected_coordinate_refuses() {
    let world = World::build();
    World::write(
        &world.provider_root,
        "vibevm/vibespecs/common/PROMPT-004.md",
        "# Ghost {#root}\n\n#embed spec://org.demo.ghost/common/NOTE-001#root\n",
    );
    let error = world
        .resolver()
        .resolve(&world.request("spec://org.demo/tools/common/PROMPT-004#root"))
        .unwrap_err();
    assert!(
        error.contains("selected world"),
        "an unselected coordinate must be refused as such: {error}"
    );
}

/// Nested `#use`/`#source` — inside an embedded document, so only the scan of
/// the EXPANDED closure can find them — are reported in document order, while
/// the same syntax fenced or commented (documentation, not composition) is
/// masked by the shared directive parser and reported nowhere.
#[test]
fn nested_use_and_source_refuse_while_documented_syntax_does_not() {
    let world = World::build();
    World::write(
        &world.provider_root,
        "vibevm/vibespecs/common/NOTE-002.md",
        concat!(
            "# Mixed {#root}\n",
            "\n",
            "Real composition this handler does not perform:\n",
            "\n",
            "#use spec://org.demo/tools/common/OTHER-001\n",
            "\n",
            "#source spec://org.demo/tools/common/OTHER-002\n",
            "\n",
            "Documented, never executed:\n",
            "\n",
            "```text\n",
            "#use spec://org.demo/tools/common/FENCED-001\n",
            "#source spec://org.demo/tools/common/FENCED-002\n",
            "```\n",
            "\n",
            "<!-- #use spec://org.demo/tools/common/COMMENTED-001 -->\n",
            "<!-- #source spec://org.demo/tools/common/COMMENTED-002 -->\n",
        ),
    );
    World::write(
        &world.provider_root,
        "vibevm/vibespecs/common/PROMPT-005.md",
        "# Nester {#root}\n\n#embed spec://org.demo/tools/common/NOTE-002#root\n",
    );
    let resolved = world
        .resolver()
        .resolve(&world.request("spec://org.demo/tools/common/PROMPT-005#root"))
        .unwrap();
    assert_eq!(
        resolved.unsupported,
        [
            "#use spec://org.demo/tools/common/OTHER-001",
            "#source spec://org.demo/tools/common/OTHER-002",
        ],
        "exactly the two live directives, in document order — nothing fenced \
         or commented: {}",
        resolved.text
    );
}

/// The same refusal for a directive in the prompt document itself, not nested
/// through an embed.
#[test]
fn a_use_directive_in_the_prompt_document_itself_refuses() {
    let world = World::build();
    World::write(
        &world.provider_root,
        "vibevm/vibespecs/common/PROMPT-006.md",
        "# User {#root}\n\n#use spec://org.demo/lib/common/NOTE-001\n",
    );
    let resolved = world
        .resolver()
        .resolve(&world.request("spec://org.demo/tools/common/PROMPT-006#root"))
        .unwrap();
    assert_eq!(
        resolved.unsupported,
        ["#use spec://org.demo/lib/common/NOTE-001"]
    );
}

/// A prompt that resolves to nothing readable refuses: a missing document
/// with the resolver's own reason, a blank one with the blank refusal.
#[test]
fn blank_and_missing_prompts_refuse() {
    let world = World::build();
    let missing = world
        .resolver()
        .resolve(&world.request("spec://org.demo/tools/common/PROMPT-099#root"))
        .unwrap_err();
    assert!(
        missing.contains("not found"),
        "a missing document refuses as not found: {missing}"
    );

    // A whole blank document addressed without an anchor: the section text is
    // whitespace, so the expanded closure is blank and must refuse — an empty
    // prompt must never reach the fingerprint or a provider.
    World::write(
        &world.provider_root,
        "vibevm/vibespecs/common/PROMPT-007.md",
        " \n\t\n",
    );
    let blank = world
        .resolver()
        .resolve(&world.request("spec://org.demo/tools/common/PROMPT-007"))
        .unwrap_err();
    assert!(
        blank.contains("empty"),
        "a blank prompt refuses as empty: {blank}"
    );
}

/// An address that does not parse refuses before any tree is touched.
#[test]
fn a_malformed_address_refuses_before_resolution() {
    let world = World::build();
    let error = world
        .resolver()
        .resolve(&world.request("common/PROMPT-001#root"))
        .unwrap_err();
    assert!(!error.is_empty());
}

/// The paid/config fence: the lower resolver cell carries none of the
/// vocabulary a completion surface would need — no provider construction, no
/// user config, no credential reader, no transport, no discovery. Prose
/// about the split (comments) is not the subject; code is.
///
/// The mutation this kills is reaching for `vibe-llm`/`reqwest`/`UserConfig`
/// (or a `Workspace::discover`) from the shared cell, which would drag the
/// paid half into every hosted surface that composes the resolver.
#[test]
fn the_agent_cell_carries_no_paid_or_config_vocabulary() {
    let cell = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/agent");
    let mut offenders: Vec<String> = Vec::new();
    let mut stack = vec![cell.clone()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                // The test cells script their own backends; the fence is over
                // production sources.
                if path.file_name().and_then(|n| n.to_str()) == Some("tests") {
                    continue;
                }
                stack.push(path);
                continue;
            }
            if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
                continue;
            }
            let body = fs::read_to_string(&path)
                .unwrap()
                .replace(char::from(13), "");
            for line in body.lines() {
                let trimmed = line.trim_start();
                if trimmed.starts_with("//") {
                    continue;
                }
                for token in [
                    "vibe_llm",
                    "vibe-llm",
                    "vibe_llm::",
                    "UserConfig",
                    "LlmSection",
                    "resolve_effective_config",
                    "SystemCredentialReader",
                    "ReqwestChatTransport",
                    "LLMProvider",
                    "reqwest",
                    "Workspace::discover",
                    "user_config_path",
                    "api_key",
                ] {
                    if trimmed.contains(token) {
                        offenders.push(format!(
                            "{} carries `{token}` outside a comment",
                            path.display(),
                        ));
                    }
                }
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "prompt resolution is credential-free shared behavior: {offenders:#?}",
    );

    // The dependency half of the same fence: the crate must not depend on the
    // paid half at all, or the vocabulary above could creep back in silently.
    let manifest = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
        .unwrap()
        .replace(char::from(13), "");
    for line in manifest.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("#") {
            continue;
        }
        for dep in ["vibe-llm", "reqwest"] {
            assert!(
                !trimmed.contains(dep),
                "vibe-lifecycle must not depend on {dep}: {trimmed}",
            );
        }
    }
}
