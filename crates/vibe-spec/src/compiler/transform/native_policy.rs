//! Pure pending-state values for compiler-native execution.
//!
//! Public values expose only the frozen pending tuple and successful replay
//! counts. The sibling `session` cell retains conflict members and mutable
//! policy state; neither cell invokes a handler or owns an artifact.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#BOOTSTRAP-ORDER");

use std::fmt;
use vibe_core::manifest::ExtensionKey;

#[path = "native_policy/outcome.rs"]
mod outcome;
#[path = "native_policy/session.rs"]
pub(crate) mod session;
pub use outcome::{
    CompilerNativeOutcome, CompilerNativeStatus, CompilerPendingArtifact, CompilerReadyArtifact,
};
pub use session::CompilerNativePolicyError;
use session::{PendingCapture, Receipt};

/// Closed native-manager policy. Resolve consumes a genuine collected set.
pub struct CompilerNativePolicy {
    pub(super) mode: PolicyMode,
}

pub(super) enum PolicyMode {
    Fail,
    Collect,
    Resolve(CompilerPendingSet),
}

impl fmt::Debug for CompilerNativePolicy {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.mode {
            PolicyMode::Fail => formatter.write_str("CompilerNativePolicy::Fail"),
            PolicyMode::Collect => formatter.write_str("CompilerNativePolicy::Collect"),
            PolicyMode::Resolve(expected) => formatter
                .debug_tuple("CompilerNativePolicy::Resolve")
                .field(expected)
                .finish(),
        }
    }
}

impl CompilerNativePolicy {
    #[must_use]
    pub const fn fail() -> Self {
        Self {
            mode: PolicyMode::Fail,
        }
    }

    #[must_use]
    pub const fn collect() -> Self {
        Self {
            mode: PolicyMode::Collect,
        }
    }

    #[must_use]
    pub fn resolve(expected: CompilerPendingSet) -> Self {
        Self {
            mode: PolicyMode::Resolve(expected),
        }
    }
}

/// Public, read-only `(plan digest, dense order, qualified key)` identity.
#[derive(PartialEq, Eq)]
pub struct CompilerPendingRef {
    pub(super) plan_digest: [u8; 32],
    pub(super) order: u32,
    pub(super) key: ExtensionKey,
}

impl fmt::Debug for CompilerPendingRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CompilerPendingRef")
            .field("plan_digest_hex", &self.plan_digest_hex())
            .field("order", &self.order)
            .field("key", &self.key.as_str())
            .finish()
    }
}

impl CompilerPendingRef {
    #[must_use]
    pub const fn plan_digest_bytes(&self) -> &[u8; 32] {
        &self.plan_digest
    }

    #[must_use]
    pub fn plan_digest_hex(&self) -> String {
        lower_hex(&self.plan_digest)
    }

    #[must_use]
    pub const fn order(&self) -> u32 {
        self.order
    }

    #[must_use]
    pub const fn key(&self) -> &ExtensionKey {
        &self.key
    }
}

/// Ordered, unique and non-Clone pending state returned only by Collect.
pub struct CompilerPendingSet {
    pub(super) plan_digest: Option<[u8; 32]>,
    pub(super) entries: Box<[PendingCapture]>,
}

impl fmt::Debug for CompilerPendingSet {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CompilerPendingSet")
            .field("entries", &PendingRefList(&self.entries))
            .finish()
    }
}

impl CompilerPendingSet {
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &CompilerPendingRef> {
        self.entries.iter().map(|entry| &entry.reference)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Ordered successful replay references with positive invocation counts.
pub struct CompilerInvocationReceipts {
    pub(super) entries: Box<[Receipt]>,
}

impl fmt::Debug for CompilerInvocationReceipts {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CompilerInvocationReceipts")
            .field("entries", &ReceiptList(&self.entries))
            .finish()
    }
}

impl CompilerInvocationReceipts {
    pub(crate) fn empty() -> Self {
        Self {
            entries: Box::new([]),
        }
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = (&CompilerPendingRef, u64)> {
        self.entries
            .iter()
            .map(|entry| (&entry.capture.reference, entry.count))
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

struct PendingRefList<'a>(&'a [PendingCapture]);

impl fmt::Debug for PendingRefList<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_list()
            .entries(self.0.iter().map(|entry| &entry.reference))
            .finish()
    }
}

struct ReceiptList<'a>(&'a [Receipt]);

impl fmt::Debug for ReceiptList<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut list = formatter.debug_list();
        for receipt in self.0 {
            list.entry(&(&receipt.capture.reference, receipt.count));
        }
        list.finish()
    }
}

fn lower_hex(bytes: &[u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut text = String::with_capacity(64);
    for byte in bytes {
        text.push(HEX[(byte >> 4) as usize] as char);
        text.push(HEX[(byte & 0x0f) as usize] as char);
    }
    text
}
