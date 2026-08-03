//! Lockfile provenance strings that are *not* identity.
//!
//! [`SourceUrl`] (where the bytes came from on a given install) and
//! [`TraceId`] (the LLM run that emitted a virtual capability) are
//! informational — a mirror-switch changes `SourceUrl` without changing
//! a package's identity (PROP-002 §2.7). They carry no parse grammar of
//! their own (a `SourceUrl` is a git URL *or*, for a path-source entry,
//! a workspace-relative path); the newtypes exist for type clarity —
//! keeping these provenance strings from being confused with the
//! identity [`ContentHash`](crate::ContentHash) and with each other —
//! not for validation.
//!
//! Spec: [PROP-002 §2.7](../../../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#lockfile).

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#lockfile");

use std::fmt;

use serde::{Deserialize, Serialize};

macro_rules! provenance_newtype {
    ($ty:ident, $doc:literal) => {
        #[doc = $doc]
        ///
        /// `serde(transparent)`; the wire form is the bare string the
        /// lockfile already carries. No parse grammar — see the module
        /// docs for why these are type-clarity wrappers, not validators.
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $ty(String);

        impl $ty {
            /// Wrap a provenance string.
            pub fn new(s: impl Into<String>) -> Self {
                $ty(s.into())
            }

            /// The value as a string slice.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $ty {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl std::ops::Deref for $ty {
            type Target = str;
            fn deref(&self) -> &str {
                &self.0
            }
        }

        impl From<String> for $ty {
            fn from(s: String) -> Self {
                $ty(s)
            }
        }

        impl From<&str> for $ty {
            fn from(s: &str) -> Self {
                $ty(s.to_owned())
            }
        }

        impl From<$ty> for String {
            fn from(v: $ty) -> String {
                v.0
            }
        }

        impl AsRef<str> for $ty {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }

        impl PartialEq<str> for $ty {
            fn eq(&self, other: &str) -> bool {
                self.0 == other
            }
        }

        impl PartialEq<&str> for $ty {
            fn eq(&self, other: &&str) -> bool {
                self.0 == *other
            }
        }
    };
}

provenance_newtype!(
    SourceUrl,
    "Where a package's content came from on the install that produced a lockfile entry — a git URL, or a workspace-relative path for a path-source entry. Informational, never identity (PROP-002 §2.7)."
);
provenance_newtype!(
    TraceId,
    "The trace ID of the LLM run that emitted a virtual capability (PROP-003 §2.5.3) — links a lockfile record into the `vibe build` audit log."
);
