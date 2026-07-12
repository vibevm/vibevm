//! Compile-and-run usage tests: every carrier form of the tag grammar
//! applied to real items. The tags MUST be inert — the items behave
//! exactly as untagged ones — and the test compiling at all is the
//! grammar-acceptance assertion. Grammar *rejections* are unit-tested in
//! `core-ai-native-specmark-grammar`; a rejected form here would fail the
//! build, which is the wrong place to assert it.

// The alias every consumer takes in its Cargo.toml (`specmark = { package
// = "core-ai-native-specmark", … }`) — here spelled as a use-alias, so the
// carrier forms below exercise the exact `specmark::…` paths consumers write.
use core_ai_native_specmark as specmark;
use core_ai_native_specmark::{spec, verifies};

const URI: &str = "spec://org.vibevm.ai-native.core-ai-native/mechanisms/PROP-014#addressing-code";

specmark::scope!("spec://org.vibevm.ai-native.core-ai-native/mechanisms/PROP-014#addressing-code");

#[spec(
    implements = "spec://org.vibevm.ai-native.core-ai-native/mechanisms/PROP-014#addressing-code",
    r = 1
)]
struct Tagged {
    value: u8,
}

#[spec(
    implements = "spec://org.vibevm.ai-native.core-ai-native/mechanisms/PROP-014#addressing-code"
)]
#[spec(
    documents = "spec://org.vibevm.ai-native.core-ai-native/mechanisms/PROP-014#addressing-spec",
    r = 1
)]
enum MultiTagged {
    A,
    B,
}

#[spec(
    deviates = "spec://org.vibevm.ai-native.core-ai-native/mechanisms/PROP-014#addressing-code",
    r = 1,
    reason = "test fixture exercising the deviates carrier form"
)]
impl Tagged {
    fn doubled(&self) -> u8 {
        self.value * 2
    }
}

#[spec(informs = "spec://org.vibevm.ai-native.core-ai-native/mechanisms/PROP-014#edges~r1")]
fn helper(x: u8) -> u8 {
    x + 1
}

mod inner {
    use core_ai_native_specmark as specmark;

    specmark::scope!(
        "spec://org.vibevm.ai-native.core-ai-native/mechanisms/PROP-014#addressing-code",
        r = 1
    );

    pub fn inherited() -> &'static str {
        "covered by the module scope marker"
    }
}

#[test]
#[verifies(
    "spec://org.vibevm.ai-native.core-ai-native/mechanisms/PROP-014#addressing-code",
    r = 1
)]
fn tags_are_inert() {
    let t = Tagged { value: 21 };
    assert_eq!(t.doubled(), 42);
    assert_eq!(helper(1), 2);
    let _ = (MultiTagged::A, MultiTagged::B);
    assert_eq!(inner::inherited(), "covered by the module scope marker");
    assert!(URI.starts_with("spec://"));
}

#[test]
#[verifies("spec://org.vibevm.ai-native.core-ai-native/mechanisms/PROP-014#addressing-code")]
fn verifies_without_pin_compiles_and_runs() {
    assert_eq!(1 + 1, 2);
}
