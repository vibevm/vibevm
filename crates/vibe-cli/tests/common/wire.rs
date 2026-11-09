//! The wire-contract oracle for a registered `--json` report.
//!
//! A report that `formats/REGISTRY.toml` claims has a generated reader in
//! `vibe_wire::generated`, and PROP-044 §8 (G11) wants that reader exercised
//! on the real document. So every document a test reads goes through its
//! generated type and back out again, and the two renderings must agree byte
//! for byte: a key that disappeared, a key nobody declared, a `null` that
//! became an absence, or an object whose members changed places all fail
//! here — at the test that read the document, not at a foreign parser.

use serde::Serialize;
use serde::de::DeserializeOwned;

/// Read `stdout` as the registered report `T` describes it, check it against
/// the generated type, and hand back the document as it was printed.
pub fn report<T: DeserializeOwned + Serialize>(stdout: &str) -> serde_json::Value {
    let _ = typed::<T>(stdout);
    serde_json::from_str(stdout).unwrap_or_else(|error| panic!("{error}: {stdout}"))
}

/// The same check, choosing the type from the document's own `protocol`
/// member — for a test binary whose commands print more than one report. A
/// document of a protocol this oracle does not know yet is handed back
/// unchecked; the per-protocol tests pin the names it does know.
pub fn checked(stdout: &str) -> serde_json::Value {
    let document: serde_json::Value =
        serde_json::from_str(stdout).unwrap_or_else(|error| panic!("{error}: {stdout}"));
    match document
        .get("protocol")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
    {
        "vibe-artifacts-where/1" => {
            report::<vibe_wire::generated::artifacts_where_report::ArtifactsWhereReport>(stdout)
        }
        "vibe-self-apply/1" => {
            report::<vibe_wire::generated::self_apply_report::SelfApplyReport>(stdout)
        }
        "vibe-self-releases/1" => {
            report::<vibe_wire::generated::self_releases_report::SelfReleasesReport>(stdout)
        }
        "vibe-self-source/1" => {
            report::<vibe_wire::generated::self_source_report::SelfSourceReport>(stdout)
        }
        _ => document,
    }
}

/// The same check, handing back the generated value instead of the document —
/// for a test that wants to name the vocabulary values it expects.
pub fn typed<T: DeserializeOwned + Serialize>(stdout: &str) -> T {
    let document: serde_json::Value =
        serde_json::from_str(stdout).unwrap_or_else(|error| panic!("{error}: {stdout}"));
    let object = document
        .as_object()
        .unwrap_or_else(|| panic!("a report is a JSON object: {stdout}"));
    // `--json` stamps `invoked_by` and `unattended` onto every top-level
    // object; neither belongs to any report's schema, and both are stamped
    // last, so dropping them leaves the report's own order untouched.
    let declared: serde_json::Map<String, serde_json::Value> = object
        .iter()
        .filter(|(key, _)| !matches!(key.as_str(), "invoked_by" | "unattended"))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    let declared = serde_json::Value::Object(declared);
    let value: T = serde_json::from_value(declared.clone()).unwrap_or_else(|error| {
        panic!("the document does not read as its registered wire type: {error}\n{stdout}")
    });
    assert_eq!(
        serde_json::to_string_pretty(&value).unwrap(),
        serde_json::to_string_pretty(&declared).unwrap(),
        "the emitted document and its registered wire type disagree"
    );
    value
}
