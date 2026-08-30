//! The declaration-fingerprint REDs (R7.5 P2/A4b).
//!
//! The declaration fingerprint is the evidence sibling of the execution
//! fingerprint: it answers «is this the same DECLARED work?», so the mutation
//! axes below are exactly the spec's exclusions and inclusions
//! (PROP-054 `##DECLARATION-FINGERPRINT`). One case also recomputes the
//! digest longhand from the frozen recipe — an independent implementation
//! of the framing, not a call into production — and pins the vector.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use vibe_core::ContentHash;
use vibe_core::manifest::{
    ExtensionAppliesTo, ExtensionDecl, ExtensionHandler, ExtensionIrLevel, ExtensionPass,
    ExtensionPassKind, ExtensionWhen, ExtensionsControl,
};
use vibe_wire::generated::lifecycle::e1::context::SlotTarget;

use super::support::{config, context};
use crate::HandlerExecution;
use crate::state::fingerprint::legacy;
use crate::state::prepare_handler_execution_with;
use crate::{
    ExtensionRegistryRow, ExtensionWorld, HostExtensionSource, HostIdentity, HostProvider,
    SelectorSubject, collect_extensions,
};

fn project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("vibe.toml"),
        "[project]\nname='demo'\nversion='0.1.0'\n",
    )
    .unwrap();
    fs::write(dir.path().join("vibe.lock"), "lock").unwrap();
    fs::write(dir.path().join("a.txt"), "one").unwrap();
    dir
}

fn base_decl(point: &str) -> ExtensionDecl {
    ExtensionDecl {
        id: "announce".into(),
        point: point.parse().unwrap(),
        handler: ExtensionHandler::Builtin { name: "log".into() },
        config: Some(config("one")),
        auto: None,
        inputs: None,
        applies_to: None,
        compiler_internals: None,
        pass: None,
        when: None,
    }
}

/// One planned host declaration under the standard ungrouped-`demo` host.
fn host_row(declaration: ExtensionDecl) -> ExtensionRegistryRow {
    host_row_with_content(declaration, None)
}

/// The same host row with an explicit precomputed host content hash — the
/// `provider_content_present` axis.
fn host_row_with_content(
    declaration: ExtensionDecl,
    content: Option<&str>,
) -> ExtensionRegistryRow {
    let point = declaration.point;
    let registry = collect_extensions(ExtensionWorld {
        installed: vec![],
        host: HostExtensionSource {
            provider: HostProvider {
                identity: HostIdentity::ungrouped_project("demo"),
                root: PathBuf::from("."),
                version: "0.1.0".into(),
                kind: None,
                content_hash: content.map(|value| ContentHash::parse(value).unwrap()),
            },
            declarations: vec![declaration],
            controls: ExtensionsControl::default(),
            mechanisms: Vec::new(),
        },
        effective_stack: None,
    })
    .unwrap();
    registry.plan(point, SelectorSubject::unscoped())[0].clone()
}

fn prepared(root: &Path, row: &ExtensionRegistryRow) -> String {
    let ctx = context(root, row.effective_config().unwrap());
    prepare_handler_execution_with(&HandlerExecution::from_row(row), &ctx, None)
        .unwrap()
        .declaration_fingerprint
}

/// The frozen longhand recipe — an INDEPENDENT framing implementation.
fn frame(hash: &mut Sha256, label: &str, value: &[u8]) {
    hash.update((label.len() as u64).to_be_bytes());
    hash.update(label.as_bytes());
    hash.update((value.len() as u64).to_be_bytes());
    hash.update(value);
}

fn present(hash: &mut Sha256, label: &str, is_present: bool) {
    frame(hash, label, if is_present { b"1" } else { b"0" });
}

/// The longhand digest of the standard fixture row: everything AFTER the
/// optional slot coordinate is shared by both vectors, so the tail lives
/// here exactly once.
fn longhand_standard_tail(hash: &mut Sha256) {
    frame(hash, "handler_kind", b"builtin");
    frame(hash, "handler_name", b"log");
    frame(hash, "effective_config", br#"{"message":"one"}"#);
    frame(hash, "provider_kind", b"host");
    frame(hash, "provider_id", b"__host__/demo");
    frame(hash, "provider_version", b"0.1.0");
    present(hash, "provider_content_present", false);
    present(hash, "inputs_present", true);
    frame(hash, "pattern_count", b"1");
    frame(hash, "pattern", b"*.txt");
    present(hash, "pass_present", false);
    present(hash, "compiler_internals_present", false);
    present(hash, "prompt_present", false);
}

/// The production digest must equal an independent longhand recompute of the
/// frozen recipe, for both a plain phase row and a slot-targeted one.
#[test]
fn the_declaration_framing_matches_an_independent_longhand_recompute() {
    let dir = project();
    let mut declaration = base_decl("phase:build");
    declaration.inputs = Some(vec!["*.txt".into()]);
    let row = host_row(declaration);
    let ctx = context(dir.path(), row.effective_config().unwrap());
    let production = prepare_handler_execution_with(&HandlerExecution::from_row(&row), &ctx, None)
        .unwrap()
        .declaration_fingerprint;

    let mut hash = Sha256::new();
    hash.update(b"vibe-execution-declaration-v1\0epoch=1\0");
    frame(&mut hash, "execution", b"__host__/demo#announce");
    frame(&mut hash, "phase", b"build");
    frame(&mut hash, "point", b"phase:build");
    present(&mut hash, "slot_present", false);
    longhand_standard_tail(&mut hash);
    assert_eq!(
        production,
        format!("sha256:{:x}", hash.finalize()),
        "the declaration framing must equal the frozen recipe",
    );

    // The slot-targeted variant: same context, descriptor coordinate bound,
    // the absolute machine root deliberately invisible.
    let targeted = HandlerExecution::from_row(&row).with_slot_target(SlotTarget {
        group: "org.demo".into(),
        kind: "stack".into(),
        name: "rust-stack".into(),
        root: "D:/wherever/the/slot/lives".into(),
        version: "1.0.0".into(),
    });
    let production = prepare_handler_execution_with(&targeted, &ctx, None)
        .unwrap()
        .declaration_fingerprint;
    let mut hash = Sha256::new();
    hash.update(b"vibe-execution-declaration-v1\0epoch=1\0");
    frame(
        &mut hash,
        "execution",
        b"__host__/demo#announce@slot(org.demo/rust-stack@1.0.0)",
    );
    frame(&mut hash, "phase", b"build");
    frame(&mut hash, "point", b"phase:build");
    present(&mut hash, "slot_present", true);
    frame(&mut hash, "slot_group", b"org.demo");
    frame(&mut hash, "slot_kind", b"stack");
    frame(&mut hash, "slot_name", b"rust-stack");
    frame(&mut hash, "slot_version", b"1.0.0");
    longhand_standard_tail(&mut hash);
    assert_eq!(
        production,
        format!("sha256:{:x}", hash.finalize()),
        "the slot-targeted framing must equal the frozen recipe",
    );
}

/// Invocation/world/artifact/input-byte changes are execution-freshness or
/// independent-witness material: the declaration identity must not move.
#[test]
fn invocation_world_and_input_bytes_do_not_move_the_declaration_identity() {
    let dir = project();
    let mut declaration = base_decl("phase:build");
    declaration.inputs = Some(vec!["*.txt".into()]);
    let row = host_row(declaration);
    let base_ctx = context(dir.path(), row.effective_config().unwrap());
    let base =
        prepare_handler_execution_with(&HandlerExecution::from_row(&row), &base_ctx, None).unwrap();

    let mut moved = base_ctx.clone();
    moved.run.requested = "test".into();
    moved.run.chain.push("test".into());
    moved.run.offline = true;
    moved.run.agent_mode = vibe_wire::generated::lifecycle::e1::context::RunAgentMode::Agent;
    moved
        .artifacts
        .push(vibe_wire::generated::lifecycle::e1::context::Artifact {
            id: "a".into(),
            kind: "file".into(),
            path: "docs/a.md".into(),
            phase: "build".into(),
        });
    moved
        .world
        .packages
        .push(vibe_wire::generated::lifecycle::e1::context::WorldPackage {
            group: "org.demo".into(),
            kind: "tool".into(),
            name: "tools".into(),
            slot: "vibedeps/org.demo.tools".into(),
            version: "1.0.0".into(),
        });
    let moved_prepared =
        prepare_handler_execution_with(&HandlerExecution::from_row(&row), &moved, None).unwrap();
    assert_ne!(moved_prepared.fingerprint, base.fingerprint);
    assert_eq!(
        moved_prepared.declaration_fingerprint, base.declaration_fingerprint,
        "requested/chain/offline/agent-mode/world/artifacts are not declaration material",
    );

    fs::write(dir.path().join("a.txt"), "two").unwrap();
    let after =
        prepare_handler_execution_with(&HandlerExecution::from_row(&row), &base_ctx, None).unwrap();
    assert_ne!(after.fingerprint, base.fingerprint);
    assert_ne!(after.input_manifest, base.input_manifest);
    assert_eq!(
        after.declaration_fingerprint, base.declaration_fingerprint,
        "the current input BYTES belong to the manifest witness, never the declaration",
    );
}

/// Effective config and the provider pin ARE the delivered work: both move
/// the declaration, including the host content-hash presence bit.
#[test]
fn effective_config_and_provider_move_the_declaration_identity() {
    let dir = project();
    let plain = prepared(dir.path(), &host_row(base_decl("phase:build")));

    let mut reconfigured = base_decl("phase:build");
    reconfigured.config = Some(config("two"));
    assert_ne!(prepared(dir.path(), &host_row(reconfigured)), plain);

    let with_content = prepared(
        dir.path(),
        &host_row_with_content(base_decl("phase:build"), Some("sha256:aa")),
    );
    assert_ne!(with_content, plain);
    assert_ne!(
        prepared(
            dir.path(),
            &host_row_with_content(base_decl("phase:build"), Some("sha256:bb"))
        ),
        with_content,
        "the content hash value itself is bound",
    );

    // The shared dependency fixture: another provider kind/id/version pins
    // a different declaration than the ungrouped host's.
    let dependency = super::support::dependency_row(
        Path::new("vibedeps/org.demo.tools"),
        "announce",
        None,
        "sha256:aa",
    );
    assert_ne!(prepared(dir.path(), &dependency), plain);
}

/// Point and the exhaustive handler payload move the declaration; native
/// option/map presence keeps `None` distinct from authored values.
#[test]
fn point_and_handler_payload_move_the_declaration_identity() {
    let dir = project();
    let base = prepared(dir.path(), &host_row(base_decl("phase:build")));

    let mut other_point = base_decl("phase:test");
    other_point.inputs = None;
    assert_ne!(prepared(dir.path(), &host_row(other_point)), base);

    let mut scripted = base_decl("phase:build");
    // The interpreter ladder owns the extension: an authored base omits it.
    scripted.handler = ExtensionHandler::Script {
        base: PathBuf::from("scripts/run"),
    };
    assert_ne!(prepared(dir.path(), &host_row(scripted)), base);

    let native = |crate_dir: Option<&str>, prebuilt: Option<BTreeMap<String, PathBuf>>| {
        let mut declaration = base_decl("phase:build");
        declaration.handler = ExtensionHandler::Native {
            crate_dir: crate_dir.map(PathBuf::from),
            prebuilt,
        };
        prepared(dir.path(), &host_row(declaration))
    };
    // A native handler must carry crate_dir or prebuilt; the presence bits
    // are exercised across the legal shapes.
    let with_crate = native(Some("ext/squeeze"), None);
    let prebuilt_only = native(
        None,
        Some(BTreeMap::from([(
            "windows-x86_64".to_string(),
            PathBuf::from("bin/squeeze.dll"),
        )])),
    );
    assert_ne!(
        prebuilt_only, with_crate,
        "handler_crate_present is explicit, so which half carries the handler is bound",
    );
    let both = native(
        Some("ext/squeeze"),
        Some(BTreeMap::from([(
            "windows-x86_64".to_string(),
            PathBuf::from("bin/squeeze.dll"),
        )])),
    );
    assert_ne!(both, prebuilt_only);
    let two_platforms = native(
        Some("ext/squeeze"),
        Some(BTreeMap::from([
            (
                "windows-x86_64".to_string(),
                PathBuf::from("bin/squeeze.dll"),
            ),
            ("linux-x86_64".to_string(), PathBuf::from("bin/squeeze.so")),
        ])),
    );
    assert_ne!(
        two_platforms, both,
        "handler_prebuilt_count and the sorted pairs are bound",
    );
    assert_eq!(
        native(
            Some("ext/squeeze"),
            Some(BTreeMap::from([
                ("linux-x86_64".to_string(), PathBuf::from("bin/squeeze.so")),
                (
                    "windows-x86_64".to_string(),
                    PathBuf::from("bin/squeeze.dll")
                ),
            ]))
        ),
        two_platforms,
        "map iteration order is normalised by sorted platforms",
    );
}

/// Absent, authored-empty and ordered pattern lists are three distinct
/// declarations; a reorder alone moves the digest.
#[test]
fn input_pattern_presence_and_order_move_the_declaration_identity() {
    let dir = project();
    let absent = prepared(dir.path(), &host_row(base_decl("phase:build")));
    let mut empty = base_decl("phase:build");
    empty.inputs = Some(vec![]);
    assert_ne!(
        prepared(dir.path(), &host_row(empty)),
        absent,
        "inputs_present keeps `None` and `Some([])` distinct",
    );

    let patterns = |list: Vec<&str>| {
        let mut declaration = base_decl("phase:build");
        declaration.inputs = Some(list.iter().map(|pattern| (*pattern).to_string()).collect());
        prepared(dir.path(), &host_row(declaration))
    };
    assert_ne!(patterns(vec!["*.txt"]), absent);
    assert_ne!(patterns(vec!["*.txt"]), patterns(vec!["*.log"]));
    assert_ne!(
        patterns(vec!["*.txt", "*.log"]),
        patterns(vec!["*.log", "*.txt"]),
        "declaration order is bound",
    );
}

/// The pass placement and the `compiler_internals` capability bit are
/// declaration material at their one legal point.
#[test]
fn pass_fields_and_compiler_internals_move_the_declaration_identity() {
    let dir = project();
    let pass = |pass: Option<ExtensionPass>| {
        let mut declaration = base_decl("compile:pass");
        declaration.pass = pass;
        declaration.compiler_internals = Some(true);
        prepared(dir.path(), &host_row(declaration))
    };

    let minimal = pass(Some(ExtensionPass {
        kind: ExtensionPassKind::Transform,
        level: None,
        from: None,
        to: None,
        after: None,
        before: None,
        replace: None,
        formats: None,
        artifact: None,
    }));
    let no_pass = pass(None);
    assert_ne!(
        minimal, no_pass,
        "pass_present separates a pass declaration from a bare internals flag"
    );

    let with_level = pass(Some(ExtensionPass {
        kind: ExtensionPassKind::Transform,
        level: Some(ExtensionIrLevel::Closure),
        from: None,
        to: None,
        after: Some("qualify".into()),
        before: None,
        replace: None,
        formats: Some(vec!["xml".into(), "md".into()]),
        artifact: Some("static-xml".into()),
    }));
    assert_ne!(with_level, minimal);
    let reordered_formats = pass(Some(ExtensionPass {
        kind: ExtensionPassKind::Transform,
        level: Some(ExtensionIrLevel::Closure),
        from: None,
        to: None,
        after: Some("qualify".into()),
        before: None,
        replace: None,
        formats: Some(vec!["md".into(), "xml".into()]),
        artifact: Some("static-xml".into()),
    }));
    assert_ne!(
        reordered_formats, with_level,
        "formats keep declaration order",
    );
    let lowered = pass(Some(ExtensionPass {
        kind: ExtensionPassKind::Lowering,
        level: Some(ExtensionIrLevel::Closure),
        from: None,
        to: None,
        after: Some("qualify".into()),
        before: None,
        replace: None,
        formats: Some(vec!["xml".into(), "md".into()]),
        artifact: Some("static-xml".into()),
    }));
    assert_ne!(lowered, with_level, "pass_kind is bound");
}

/// The resolved agent prompt is declaration material: presence, address and
/// the exact resolved bytes all move the digest.
#[test]
fn resolved_prompt_bytes_move_the_declaration_identity() {
    use crate::agent::tests::support::{
        PROMPT, RecordingBackend, TWO_OUTPUTS, context as agent_context, row as agent_row,
    };

    let dir = project();
    let row = agent_row(TWO_OUTPUTS, PROMPT);
    let ctx = agent_context(dir.path(), &row);

    let unprepared = prepare_handler_execution_with(&HandlerExecution::from_row(&row), &ctx, None)
        .unwrap()
        .declaration_fingerprint;
    let backend = RecordingBackend::answering_prompt("Write v1.", r#"{"outputs":[]}"#);
    let prepared_agent = crate::agent::prepare(&backend, &row, &ctx)
        .unwrap()
        .unwrap();
    let first = prepare_handler_execution_with(
        &HandlerExecution::from_row(&row),
        &ctx,
        Some(&prepared_agent),
    )
    .unwrap()
    .declaration_fingerprint;
    assert_ne!(
        first, unprepared,
        "prompt_present separates a prepared agent row from a bare one",
    );

    let changed = RecordingBackend::answering_prompt("Write v2.", r#"{"outputs":[]}"#);
    let changed_agent = crate::agent::prepare(&changed, &row, &ctx)
        .unwrap()
        .unwrap();
    let second = prepare_handler_execution_with(
        &HandlerExecution::from_row(&row),
        &ctx,
        Some(&changed_agent),
    )
    .unwrap()
    .declaration_fingerprint;
    assert_ne!(
        second, first,
        "the exact resolved prompt bytes are bound, not just the address",
    );
}

/// The slot coordinate binds group/kind/name/version from the DESCRIPTOR and
/// never the absolute machine root: a root-only move leaves declaration
/// identity stable while the legacy execution fingerprint still sees the
/// environment-bearing context; a kind move changes the declaration.
#[test]
fn slot_kind_moves_the_declaration_but_the_machine_root_never_does() {
    let dir = project();
    let mut declaration = base_decl("phase:build");
    declaration.inputs = Some(vec!["*.txt".into()]);
    let row = host_row(declaration);
    let ctx = context(dir.path(), row.effective_config().unwrap());

    let target = |kind: &str, root: &str| SlotTarget {
        group: "org.demo".into(),
        kind: kind.into(),
        name: "rust-stack".into(),
        root: root.into(),
        version: "1.0.0".into(),
    };
    let prepare = |target: SlotTarget| {
        let execution = HandlerExecution::from_row(&row).with_slot_target(target.clone());
        let mut target_context = ctx.clone();
        target_context.slot_target = Some(target);
        prepare_handler_execution_with(&execution, &target_context, None).unwrap()
    };
    let here = prepare(target("stack", "C:/first/root"));
    let relocated = prepare(target("stack", "D:/second/root"));
    assert_eq!(
        relocated.declaration_fingerprint, here.declaration_fingerprint,
        "the absolute machine slot root is environment, not declaration",
    );
    assert_ne!(
        relocated.fingerprint, here.fingerprint,
        "the execution fingerprint keeps environment-bearing slot context separate",
    );

    let rekinded = prepare(target("feat", "C:/first/root"));
    assert_ne!(
        rekinded.declaration_fingerprint, here.declaration_fingerprint,
        "slot_kind is bound from the descriptor",
    );
}

/// Selection-only fields — `when`, `auto`, an empty selector — decide plan
/// membership, not delivered work: identical effective material keeps the
/// declaration byte-identical.
#[test]
fn excluded_selection_fields_do_not_move_the_declaration_identity() {
    let dir = project();

    let mut guarded = base_decl("phase:build");
    guarded.when = Some(ExtensionWhen::from_table(
        toml::from_str("future = true").unwrap(),
    ));
    assert_eq!(
        prepared(dir.path(), &host_row(guarded)),
        prepared(dir.path(), &host_row(base_decl("phase:build"))),
        "an opaque pre-selection guard is not delivered work",
    );

    let mut compile_base = base_decl("compile:source");
    compile_base.config = Some(config("one"));
    let mut automated = compile_base.clone();
    automated.auto = Some(false);
    let mut selected = compile_base.clone();
    selected.applies_to = Some(ExtensionAppliesTo {
        packages: None,
        paths: None,
    });
    let base = prepared(dir.path(), &host_row(compile_base));
    assert_eq!(
        prepared(dir.path(), &host_row(automated)),
        base,
        "`auto` is activation policy, not work",
    );
    assert_eq!(
        prepared(dir.path(), &host_row(selected)),
        base,
        "an authored-but-empty selector scopes nothing and binds nothing",
    );
}

/// The legacy per-pattern reference agrees that none of the declaration
/// axes above ever moved the EXECUTION fingerprint's own bytes: the two
/// identities are siblings, computed beside each other, never aliases.
#[test]
fn the_execution_fingerprint_still_matches_the_legacy_reference() {
    let dir = project();
    let mut declaration = base_decl("phase:build");
    declaration.inputs = Some(vec!["*.txt".into()]);
    let row = host_row(declaration);
    let ctx = context(dir.path(), row.effective_config().unwrap());
    assert_eq!(
        crate::state::fingerprint_execution(&row, &ctx).unwrap(),
        legacy::execution_fingerprint_with(&row, &ctx, None).unwrap(),
    );
}
