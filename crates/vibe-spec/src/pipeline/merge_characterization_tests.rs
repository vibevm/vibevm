use std::cell::Cell;

use specmark::verifies;

use super::*;

struct ExpansionFailure {
    seed: String,
    pattern: String,
    calls: Cell<usize>,
}

impl SectionSource for ExpansionFailure {
    fn section_text(&self, _address: &SpecAddress) -> Result<String, String> {
        Ok(self.seed.clone())
    }

    fn expand_pattern(&self, address: &SpecAddress) -> Result<Vec<SpecAddress>, String> {
        assert_eq!(address.without_pin(), self.pattern);
        self.calls.set(self.calls.get() + 1);
        Err("pattern observation failed".to_string())
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
fn pattern_failure_is_observed_once_and_replayed_as_the_existing_public_error() {
    let seed = "spec://org.demo/pkg/contract/api#root";
    let pattern = "spec://org.demo/plugin-*/source/impl#root";
    let source = ExpansionFailure {
        seed: format!("# API {{#root}}\n#source {pattern}\n"),
        pattern: pattern.to_string(),
        calls: Cell::new(0),
    };

    let error = compile_static(&SpecAddress::parse(seed).unwrap(), &source).unwrap_err();

    assert_eq!(source.calls.get(), 1);
    assert!(matches!(
        error,
        CompileError::Unresolved { addr, reason }
            if addr == pattern && reason == "pattern observation failed"
    ));
}

struct TextFailure {
    seed: String,
    target: String,
    target_calls: Cell<usize>,
}

impl SectionSource for TextFailure {
    fn section_text(&self, address: &SpecAddress) -> Result<String, String> {
        if address.without_pin() == self.target {
            self.target_calls.set(self.target_calls.get() + 1);
            Err("source text observation failed".to_string())
        } else {
            Ok(self.seed.clone())
        }
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
fn text_failure_is_observed_once_and_replayed_without_a_retry() {
    let seed = "spec://org.demo/pkg/contract/api#root";
    let target = "spec://org.demo/pkg/source/missing#root";
    let source = TextFailure {
        seed: format!("# API {{#root}}\n#source {target}\n"),
        target: target.to_string(),
        target_calls: Cell::new(0),
    };

    let error = compile_static(&SpecAddress::parse(seed).unwrap(), &source).unwrap_err();

    assert_eq!(source.target_calls.get(), 1);
    assert!(matches!(
        error,
        CompileError::Unresolved { addr, reason }
            if addr == target && reason == "source text observation failed"
    ));
}
