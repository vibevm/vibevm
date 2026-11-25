//! Identifiers.
//!
//! Run and pod ids are ULIDs (Crockford base32, 26 chars, time-ordered):
//! sorting by id is sorting by creation time, which the registry and the
//! CLI rely on for stable output order (D17). Scope ids name filesystem
//! scopes (D19) and are plain strings minted by mission-control.

use std::fmt;
use std::str::FromStr;

specmark::scope!("spec://fractality/PROP-001#model");

macro_rules! ulid_id {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash,
            serde::Serialize, serde::Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(ulid::Ulid);

        impl $name {
            /// Mints a fresh id.
            pub fn generate() -> Self {
                Self(ulid::Ulid::new())
            }

            /// Milliseconds since the Unix epoch encoded in the id.
            pub fn timestamp_ms(&self) -> u64 {
                self.0.timestamp_ms()
            }

            /// Git-style short-id matching: true when the canonical
            /// (uppercase) rendering starts with `prefix`, case-insensitively.
            pub fn matches_prefix(&self, prefix: &str) -> bool {
                self.0
                    .to_string()
                    .starts_with(&prefix.to_ascii_uppercase())
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }

        impl FromStr for $name {
            type Err = ulid::DecodeError;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Ok(Self(ulid::Ulid::from_string(s)?))
            }
        }
    };
}

ulid_id!(
    /// Identity of one delegated run (one worker lifecycle under one pod).
    RunId
);

ulid_id!(
    /// Identity of one pod process (the per-run supervisor, D3).
    PodId
);

ulid_id!(
    /// Identity of one boss session observed by mission-control
    /// (Campaign 2 D2/D3): the attribution unit the scoreboard and the
    /// initiative engine aggregate by.
    SessionId
);

/// Identity of a filesystem scope (D19). Minted and persisted by
/// mission-control; proven live by the rendezvous beacon, never by the
/// string alone.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(transparent)]
pub struct ScopeId(String);

impl ScopeId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ScopeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for ScopeId {
    fn from(s: &str) -> Self {
        Self(s.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_id_round_trips_through_display_and_parse() {
        let id = RunId::generate();
        let parsed: RunId = id.to_string().parse().expect("canonical form parses");
        assert_eq!(id, parsed);
    }

    #[test]
    fn prefix_matching_is_case_insensitive() {
        let id: RunId = "01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().expect("fixed ulid");
        assert!(id.matches_prefix("01arz3"));
        assert!(id.matches_prefix("01ARZ3"));
        assert!(!id.matches_prefix("zzzz"));
    }

    #[test]
    fn ulid_ids_serialize_as_plain_strings() {
        let id: RunId = "01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().expect("fixed ulid");
        let json = serde_json::to_string(&id).expect("serializes");
        assert_eq!(json, "\"01ARZ3NDEKTSV4RRFFQ69G5FAV\"");
    }
}
