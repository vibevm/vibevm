//! Claim-check file references (plan D19).
//!
//! Bulk data never rides the mission-control bus inline: above a threshold
//! a message carries a `FileRef` — scope-qualified path plus a byte range —
//! and the reader dereferences it locally **only** when the filesystem
//! scope is proven shared (the rendezvous beacon, [`crate::node`]);
//! otherwise the bus serves the bytes. A reference is an optimization,
//! never a requirement (invariant I2 survives intact).
//!
//! The range vocabulary is deliberately RFC 7233 (the S3 model, owner
//! directive): `{offset, len}` ↔ `bytes=a-b`, and the head/tail trim
//! covers the suffix form (`bytes=-N` is `{skip_head: 0, skip_tail: 0}`
//! with `skip_head = size - N` resolved at stat time — see
//! [`RefRange::resolve_against`]).

use serde::{Deserialize, Serialize};

use crate::ids::ScopeId;

specmark::scope!("spec://fractality/PROP-001#invariants");

/// A claim-check reference to bytes in a filesystem scope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileRef {
    /// The filesystem scope the path is relative to (D19 beacon-proven).
    pub fs: ScopeId,
    /// Scope-relative path, forward slashes on every OS.
    pub path: String,
    /// Byte range; omitted or `"whole"` means the entire file.
    #[serde(default)]
    pub range: RefRange,
    /// Cheap version fingerprint (size+mtime hash, MC-stamped). Readers
    /// send `If-Match` semantics: a mutated file fails loudly instead of
    /// returning silently wrong bytes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub etag: Option<String>,
    /// Optional strong integrity for immutable payloads.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    /// Reserved: a presigned capability (federation era, DEF-6). v0.1
    /// mints nothing here.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grant: Option<String>,
}

/// The `"whole"` keyword — a one-variant enum so the untagged
/// [`RefRange`] can round-trip the literal string form D19 specifies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WholeKeyword {
    #[default]
    Whole,
}

/// Byte range of a [`FileRef`]: `"whole"` | `{offset, len}` |
/// `{skip_head, skip_tail}`.
///
/// The trim form requires **both** fields explicitly — untagged decoding
/// stays unambiguous against the slice form, and a half-written range is
/// a loud error instead of a silent default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RefRange {
    /// The entire file.
    Whole(WholeKeyword),
    /// `bytes=offset..offset+len-1` (RFC 7233 first-byte/last-byte form).
    Slice { offset: u64, len: u64 },
    /// Trim from both ends; resolves against the size at stat time. For a
    /// still-growing file the length pins at resolution (non-atomicity is
    /// the documented caveat, D19).
    Trim { skip_head: u64, skip_tail: u64 },
}

impl Default for RefRange {
    fn default() -> Self {
        RefRange::Whole(WholeKeyword::Whole)
    }
}

impl RefRange {
    pub fn whole() -> Self {
        Self::default()
    }

    pub fn slice(offset: u64, len: u64) -> Self {
        RefRange::Slice { offset, len }
    }

    pub fn trim(skip_head: u64, skip_tail: u64) -> Self {
        RefRange::Trim {
            skip_head,
            skip_tail,
        }
    }

    /// Resolves the range against a known total size into
    /// `(offset, len)`, RFC 7233 semantics:
    ///
    /// - `Whole` is always satisfiable (an empty file resolves to
    ///   `(0, 0)`).
    /// - `Slice` with `offset >= size` is unsatisfiable (`None`, the 416
    ///   case); a `len` overshooting the end clamps to it.
    /// - `Trim` is unsatisfiable when the trims meet or cross.
    ///
    /// ```
    /// use fractality_core::fileref::RefRange;
    ///
    /// assert_eq!(RefRange::whole().resolve_against(10), Some((0, 10)));
    /// assert_eq!(RefRange::slice(4, 100).resolve_against(10), Some((4, 6)));
    /// assert_eq!(RefRange::trim(2, 3).resolve_against(10), Some((2, 5)));
    /// assert_eq!(RefRange::slice(10, 1).resolve_against(10), None);
    /// ```
    pub fn resolve_against(&self, size: u64) -> Option<(u64, u64)> {
        match *self {
            RefRange::Whole(_) => Some((0, size)),
            RefRange::Slice { offset, len } => {
                if offset >= size {
                    return None;
                }
                Some((offset, len.min(size - offset)))
            }
            RefRange::Trim {
                skip_head,
                skip_tail,
            } => {
                let len = size.checked_sub(skip_head)?.checked_sub(skip_tail)?;
                if len == 0 {
                    return None;
                }
                Some((skip_head, len))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_three_wire_forms_round_trip() {
        let whole: RefRange = serde_json::from_str("\"whole\"").expect("whole parses");
        assert_eq!(whole, RefRange::whole());
        assert_eq!(
            serde_json::to_string(&whole).expect("serializes"),
            "\"whole\""
        );

        let slice: RefRange =
            serde_json::from_str(r#"{"offset":4,"len":16}"#).expect("slice parses");
        assert_eq!(slice, RefRange::slice(4, 16));

        let trim: RefRange =
            serde_json::from_str(r#"{"skip_head":1,"skip_tail":2}"#).expect("trim parses");
        assert_eq!(trim, RefRange::trim(1, 2));
    }

    #[test]
    fn half_written_ranges_are_rejected_not_defaulted() {
        // A lone offset must not silently decode as a zero trim.
        assert!(serde_json::from_str::<RefRange>(r#"{"offset":4}"#).is_err());
        assert!(serde_json::from_str::<RefRange>(r#"{"skip_head":4}"#).is_err());
        assert!(serde_json::from_str::<RefRange>(r#"{}"#).is_err());
    }

    #[test]
    fn omitted_range_means_whole() {
        let r: FileRef = serde_json::from_str(r#"{"fs":"scope-1","path":"a/b.txt"}"#)
            .expect("minimal ref parses");
        assert_eq!(r.range, RefRange::whole());
        assert_eq!(r.path, "a/b.txt");
    }

    #[test]
    fn resolve_covers_the_rfc7233_edges() {
        // Whole of empty file: satisfiable, empty.
        assert_eq!(RefRange::whole().resolve_against(0), Some((0, 0)));
        // Slice starting at size: 416.
        assert_eq!(RefRange::slice(5, 1).resolve_against(5), None);
        // Slice overshooting: clamps.
        assert_eq!(RefRange::slice(3, 100).resolve_against(5), Some((3, 2)));
        // Suffix form bytes=-N as trim: last 3 bytes of 10.
        assert_eq!(RefRange::trim(7, 0).resolve_against(10), Some((7, 3)));
        // Trims meeting: empty, unsatisfiable.
        assert_eq!(RefRange::trim(5, 5).resolve_against(10), None);
        // Trims crossing: unsatisfiable.
        assert_eq!(RefRange::trim(8, 8).resolve_against(10), None);
    }

    #[test]
    fn full_ref_round_trips_with_optional_fields() {
        let r = FileRef {
            fs: ScopeId::new("01ARZ3NDEKTSV4RRFFQ69G5FAV"),
            path: "runs/x/result.md".into(),
            range: RefRange::slice(0, 64),
            etag: Some("sz123-mt456".into()),
            sha256: None,
            grant: None,
        };
        let json = serde_json::to_string(&r).expect("serializes");
        assert!(!json.contains("sha256"), "None fields stay off the wire");
        let back: FileRef = serde_json::from_str(&json).expect("parses back");
        assert_eq!(r, back);
    }
}
