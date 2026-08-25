use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use specmark::verifies;

use super::*;

struct CountingMapSource {
    texts: HashMap<String, String>,
    loads: RefCell<HashMap<String, usize>>,
}

impl CountingMapSource {
    fn new(entries: impl IntoIterator<Item = (String, String)>) -> Self {
        Self {
            texts: entries.into_iter().collect(),
            loads: RefCell::new(HashMap::new()),
        }
    }
}

impl SectionSource for CountingMapSource {
    fn section_text(&self, address: &SpecAddress) -> Result<String, String> {
        let key = address.without_pin();
        *self.loads.borrow_mut().entry(key.clone()).or_default() += 1;
        self.texts
            .get(&key)
            .cloned()
            .ok_or_else(|| "not in counting embed source".to_string())
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
fn named_embed_runs_once_while_nested_repeated_roots_share_one_observation() {
    let a = "spec://org.demo/pkg/boot/a#root";
    let b = "spec://org.demo/pkg/boot/b#root";
    let piece = "spec://org.demo/pkg/common/piece#root";
    let leaf = "spec://org.demo/pkg/common/leaf#root";
    let source = CountingMapSource::new([
        (
            a.to_string(),
            format!("# A {{#root}}\n#use {b}\n#embed {piece}\n#embed {piece}\n"),
        ),
        (b.to_string(), format!("# B {{#root}}\n#embed {piece}\n")),
        (
            piece.to_string(),
            format!("# Piece {{#piece}}\nPIECE\n#embed {leaf}\n"),
        ),
        (leaf.to_string(), "# Leaf {#leaf}\nLEAF\n".to_string()),
    ]);

    crate::compiler::builtin::reset_parse_invocations();
    crate::compiler::embed::reset_embed_invocations();
    let output = compile_static(&SpecAddress::parse(a).unwrap(), &source).unwrap();

    assert_eq!(crate::compiler::builtin::parse_invocations(), 4);
    assert_eq!(crate::compiler::embed::embed_invocations(), 1);
    assert_eq!(
        source.loads.into_inner(),
        HashMap::from([
            (a.to_string(), 1),
            (b.to_string(), 1),
            (piece.to_string(), 1),
            (leaf.to_string(), 1),
        ])
    );
    assert_eq!(output.matches("PIECE").count(), 3, "{output}");
    assert_eq!(output.matches("LEAF").count(), 3, "{output}");
    assert_eq!(
        output.matches(&format!("<!-- embed: {piece} -->")).count(),
        3,
        "{output}"
    );
    assert!(!output.contains("#embed"), "{output}");
}

struct StatefulTarget {
    root: String,
    target: String,
    target_loads: Cell<usize>,
}

impl SectionSource for StatefulTarget {
    fn section_text(&self, address: &SpecAddress) -> Result<String, String> {
        if address.without_pin() != self.target {
            return Ok(self.root.clone());
        }
        let load = self.target_loads.get() + 1;
        self.target_loads.set(load);
        Ok(if load == 1 {
            "# First {#first}\nFIRST-OBSERVATION\n".to_string()
        } else {
            "# Second {#second}\nSECOND-OBSERVATION\n".to_string()
        })
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#WHOLE-IR-WIRE")]
fn stateful_target_is_loaded_once_and_first_observation_drives_every_splice() {
    let seed = "spec://org.demo/pkg/boot/root#root";
    let target = "spec://org.demo/pkg/common/stateful#root";
    let source = StatefulTarget {
        root: format!("# Root {{#root}}\n#embed {target}\n#embed {target}\n"),
        target: target.to_string(),
        target_loads: Cell::new(0),
    };

    let output = compile_static(&SpecAddress::parse(seed).unwrap(), &source).unwrap();

    assert_eq!(source.target_loads.get(), 1);
    assert_eq!(output.matches("FIRST-OBSERVATION").count(), 2, "{output}");
    assert!(!output.contains("SECOND-OBSERVATION"), "{output}");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-LEVELS")]
fn differently_pinned_occurrences_share_one_without_pin_load_and_named_parse() {
    let seed = "spec://org.demo/pkg/boot/root#root";
    let target = "spec://org.demo/pkg/common/pinned#root";
    let source = StatefulTarget {
        root: format!("# Root {{#root}}\n#embed {target}~r1\n#embed {target}~r2\n"),
        target: target.to_string(),
        target_loads: Cell::new(0),
    };

    crate::compiler::builtin::reset_parse_invocations();
    let output = compile_static(&SpecAddress::parse(seed).unwrap(), &source).unwrap();

    assert_eq!(source.target_loads.get(), 1);
    assert_eq!(crate::compiler::builtin::parse_invocations(), 2);
    assert_eq!(output.matches("FIRST-OBSERVATION").count(), 2, "{output}");
    assert!(!output.contains("SECOND-OBSERVATION"), "{output}");
}

struct FailingTarget {
    root: String,
    target: String,
    calls: Cell<usize>,
}

impl SectionSource for FailingTarget {
    fn section_text(&self, address: &SpecAddress) -> Result<String, String> {
        if address.without_pin() != self.target {
            return Ok(self.root.clone());
        }
        self.calls.set(self.calls.get() + 1);
        Err("embed text observation failed".to_string())
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
fn target_failure_is_observed_once_and_replayed_as_the_exact_public_error() {
    let seed = "spec://org.demo/pkg/boot/root#root";
    let target = "spec://org.demo/pkg/common/missing#root";
    let source = FailingTarget {
        root: format!("# Root {{#root}}\n#embed {target}\n"),
        target: target.to_string(),
        calls: Cell::new(0),
    };

    let error = compile_static(&SpecAddress::parse(seed).unwrap(), &source).unwrap_err();

    assert_eq!(source.calls.get(), 1);
    assert!(matches!(
        error,
        CompileError::Embed(EmbedError::Unresolved { addr, reason })
            if addr == target && reason == "embed text observation failed"
    ));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
fn dependency_first_replay_selects_current_pinned_failure_spelling() {
    let a = "spec://org.demo/pkg/boot/a#root";
    let b = "spec://org.demo/pkg/boot/b#root";
    let missing = "spec://org.demo/pkg/common/missing#root";
    let replay_address = format!("{missing}~r2");
    let source = CountingMapSource::new([
        (
            a.to_string(),
            format!("# A {{#root}}\n#use {b}\n#embed {missing}~r1\n"),
        ),
        (
            b.to_string(),
            format!("# B {{#root}}\n#embed {replay_address}\n"),
        ),
    ]);

    let error = compile_static(&SpecAddress::parse(a).unwrap(), &source).unwrap_err();
    let message = error.to_string();

    assert!(matches!(
        error,
        CompileError::Embed(EmbedError::Unresolved { addr, reason })
            if addr == replay_address && reason == "not in counting embed source"
    ));
    assert_eq!(
        message,
        format!("cannot resolve embed {replay_address}: not in counting embed source")
    );
    assert_eq!(
        source.loads.into_inner(),
        HashMap::from([
            (a.to_string(), 1),
            (b.to_string(), 1),
            (missing.to_string(), 1),
        ])
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
fn recursive_cycle_keeps_the_exact_public_path_and_loads_each_target_once() {
    let seed = "spec://org.demo/pkg/boot/root#root";
    let a = "spec://org.demo/pkg/common/a#root";
    let b = "spec://org.demo/pkg/common/b#root";
    let source = CountingMapSource::new([
        (seed.to_string(), format!("# Root {{#root}}\n#embed {a}\n")),
        (a.to_string(), format!("# A {{#root}}\n#embed {b}\n")),
        (b.to_string(), format!("# B {{#root}}\n#embed {a}\n")),
    ]);

    let error = compile_static(&SpecAddress::parse(seed).unwrap(), &source).unwrap_err();

    assert!(matches!(
        error,
        CompileError::Embed(EmbedError::Cycle(path))
            if path == vec![a.to_string(), b.to_string(), a.to_string()]
    ));
    assert_eq!(
        source.loads.into_inner(),
        HashMap::from([
            (seed.to_string(), 1),
            (a.to_string(), 1),
            (b.to_string(), 1),
        ])
    );
}

struct ReplaceDropsFailedEmbed {
    root: String,
    replacement: String,
    replacement_key: String,
    missing: String,
    missing_calls: Cell<usize>,
}

impl SectionSource for ReplaceDropsFailedEmbed {
    fn section_text(&self, address: &SpecAddress) -> Result<String, String> {
        let key = address.without_pin();
        if key == self.replacement_key {
            return Ok(self.replacement.clone());
        }
        if key == self.missing {
            self.missing_calls.set(self.missing_calls.get() + 1);
            return Err("unused conservative observation failed".to_string());
        }
        Ok(self.root.clone())
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#WHOLE-IR-WIRE")]
fn failed_observation_removed_by_merge_is_unused_and_does_not_fail_embed() {
    let seed = "spec://org.demo/pkg/contract/api#root";
    let replacement = "spec://org.demo/pkg/source/replacement#root";
    let missing = "spec://org.demo/pkg/common/unused#root";
    let source = ReplaceDropsFailedEmbed {
        root: format!("# API {{#root}}\n#source {replacement}\n#embed {missing}\nCONTRACT\n"),
        replacement: "# Replacement {#root} :replace\nREPLACEMENT\n".to_string(),
        replacement_key: replacement.to_string(),
        missing: missing.to_string(),
        missing_calls: Cell::new(0),
    };

    let output = compile_static(&SpecAddress::parse(seed).unwrap(), &source).unwrap();

    assert_eq!(source.missing_calls.get(), 1);
    assert!(output.contains("REPLACEMENT"), "{output}");
    assert!(!output.contains("CONTRACT"), "{output}");
    assert!(!output.contains("#embed"), "{output}");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
fn embed_directive_copied_by_merge_is_replayed_by_the_named_embed_pass() {
    let seed = "spec://org.demo/pkg/contract/api#root";
    let source_key = "spec://org.demo/pkg/source/impl#root";
    let target = "spec://org.demo/pkg/common/piece#root";
    let source = CountingMapSource::new([
        (
            seed.to_string(),
            format!("# API {{#root}}\n#source {source_key}\nCONTRACT\n"),
        ),
        (
            source_key.to_string(),
            format!("# API {{#root}}\nSOURCE\n#embed {target}\n"),
        ),
        (
            target.to_string(),
            "# Piece {#piece}\nMERGED-EMBED\n".to_string(),
        ),
    ]);

    let output = compile_static(&SpecAddress::parse(seed).unwrap(), &source).unwrap();

    assert!(output.contains("SOURCE"), "{output}");
    assert!(output.contains("MERGED-EMBED"), "{output}");
    assert!(
        output.contains(&format!("<!-- embed: {target} -->")),
        "{output}"
    );
    assert!(!output.contains("#embed"), "{output}");
}
