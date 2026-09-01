//! Manager-owned publication of a compiled per-unit boot lane (PROP-038
//! §2.1; R4.1 atom B) — the narrow seam through which the per-unit emitter
//! hands an already-rendered INDEX / already-compiled STATIC triple to the
//! crash-recoverable transaction manager the node path already uses. No
//! caller can reach the transaction's internals through this cell; the one
//! `pub(crate)` wrapper is the whole surface.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#UNIT-PUBLICATION-TRANSACTION");

use std::path::Path;

use vibe_core::manifest::SpecFormat;

use crate::boot::EffectiveBoot;
use crate::boot::hybrid::fingerprint::NativePendingFrame;

use super::transaction;
use super::{
    FP_MARKER, INDEX_FILE, STATIC_FILE, STATIC_XML_FILE, WorkspaceError, index_header,
    prepare_index_body, static_file,
};

pub(crate) const NATIVE_PENDING_MARKER: &str = "# vibe:native-pending ";

pub(crate) struct PreparedIndex {
    header: String,
    body: String,
}

pub(crate) struct RecordedUnitFreshness {
    pub(crate) fingerprint: String,
    pub(crate) pending: Option<NativePendingFrame>,
}

pub(crate) fn prepare_index(
    boot: &EffectiveBoot,
    spec_format: SpecFormat,
) -> Result<PreparedIndex, WorkspaceError> {
    Ok(PreparedIndex {
        header: index_header(),
        body: prepare_index_body(boot, spec_format)?,
    })
}

pub(crate) fn finish_index(
    prepared: PreparedIndex,
    fingerprint: Option<&str>,
    pending: Option<NativePendingFrame>,
) -> String {
    let mut output = prepared.header;
    if let Some(fingerprint) = fingerprint {
        output.push_str(FP_MARKER);
        output.push_str(fingerprint);
        output.push('\n');
        if let Some(pending) = pending {
            output.push_str(NATIVE_PENDING_MARKER);
            output.push_str(&hex(pending.as_bytes()));
            output.push('\n');
        }
        output.push('\n');
    }
    output.push_str(&prepared.body);
    output
}

pub(crate) fn read_unit_index_freshness(text: &str) -> Result<RecordedUnitFreshness, ()> {
    let mut fingerprint = None;
    let mut fingerprint_line = None;
    let mut pending = None;
    let mut pending_line = None;
    for (line_number, line) in text.lines().enumerate() {
        if line.trim_start().starts_with("# vibe:fp") {
            let value = line.strip_prefix(FP_MARKER).ok_or(())?;
            if fingerprint.is_some() || !is_lower_hex_64(value) {
                return Err(());
            }
            fingerprint = Some(value.to_owned());
            fingerprint_line = Some(line_number);
        }
        if line.trim_start().starts_with("# vibe:native-pending") {
            let value = line.strip_prefix(NATIVE_PENDING_MARKER).ok_or(())?;
            if pending.is_some() || !is_lower_hex_64(value) {
                return Err(());
            }
            pending = Some(NativePendingFrame::new(decode_hex_32(value)?));
            pending_line = Some(line_number);
        }
    }
    let fingerprint = fingerprint.ok_or(())?;
    if let Some(pending_line) = pending_line
        && Some(pending_line) != fingerprint_line.map(|line| line + 1)
    {
        return Err(());
    }
    Ok(RecordedUnitFreshness {
        fingerprint,
        pending,
    })
}

fn is_lower_hex_64(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn decode_hex_32(value: &str) -> Result<[u8; 32], ()> {
    let mut output = [0_u8; 32];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        output[index] = (nibble(pair[0])? << 4) | nibble(pair[1])?;
    }
    Ok(output)
}

fn nibble(value: u8) -> Result<u8, ()> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        _ => Err(()),
    }
}

fn hex(bytes: &[u8; 32]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut output, byte| {
        let _ = write!(output, "{byte:02x}");
        output
    })
}

/// The generated STATIC spelling a target format does NOT select — the file
/// every publication must leave absent so a lane never carries both.
pub(crate) fn stale_static_file(spec_format: SpecFormat) -> &'static str {
    if matches!(spec_format, SpecFormat::Xml) {
        STATIC_FILE
    } else {
        STATIC_XML_FILE
    }
}

/// Publish one package unit's already-compiled boot lane (PROP-038 §2.1;
/// R4.1 atom B) through the same crash-recoverable transaction manager the
/// node path uses: `INDEX.md`, the selected STATIC's presence/bytes, and the
/// stale spelling's absence land in ONE call — or, on any refusal, not at
/// all (a partially applied set rolls back to the recorded before-state).
///
/// `index_text` is the fully rendered `INDEX.md` (fingerprint header
/// included); `static_bytes` is the compiled static lane, `None` meaning the
/// unit publishes no static artifact (both spellings end absent). Unlike the
/// node path this writes **no** redirect blocks — a dependency package slot
/// is not an agent entry point. Callers hand over compiled bytes only:
/// every fallible render/compile has already happened, so a failure here is
/// a publication failure, never a lost compile, and byte-equal artifacts
/// keep their mtime (the transaction replaces nothing it does not have to).
pub(crate) fn publish_unit_artifacts(
    boot_dir: &Path,
    index_text: &str,
    static_bytes: Option<&[u8]>,
    spec_format: SpecFormat,
) -> Result<(), WorkspaceError> {
    transaction::write_production_with_selectors(
        transaction::ArtifactWrite {
            index_path: &boot_dir.join(INDEX_FILE),
            index_bytes: index_text.as_bytes(),
            static_path: &boot_dir.join(static_file(spec_format)),
            static_bytes,
            stale_path: &boot_dir.join(stale_static_file(spec_format)),
        },
        // No selector work: a package slot writes no redirect blocks.
        |_| Ok(()),
    )
}

pub(crate) fn preflight_artifact_targets(
    index_path: &Path,
    static_path: &Path,
    stale_path: &Path,
) -> Result<(), WorkspaceError> {
    transaction::preflight_artifact_roles(transaction::ArtifactWrite {
        index_path,
        index_bytes: &[],
        static_path,
        static_bytes: None,
        stale_path,
    })
}

#[cfg(test)]
mod freshness_tests {
    use super::*;

    const FP: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const PENDING: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    fn prepared() -> PreparedIndex {
        PreparedIndex {
            header: index_header(),
            body: "schema = 1\n".to_owned(),
        }
    }

    #[test]
    fn legacy_index_bytes_are_a_hard_coded_literal_and_pending_is_one_sibling() {
        let expected = "# vibevm/vibespecs/boot/INDEX.md— generated by vibe, do not edit.\n\
# The computed boot sequence (PROP-009 §2.3). Read every file the\n\
# `[[entry]]` list names, in order.\n\
#\n\
# `installed:<group>/<name>` is resolved during generation: true\n\
# contributions appear normally; false contributions are absent entirely.\n\
# A `kind = \"static\"` entry: read the file directly. A `kind = \"dynamic\"`\n\
# entry: an INCLUDE resolved at boot — when it also carries\n\
# `when = \"os:<name>\"`, read the file only if the session's operating\n\
# system is <name> (windows / macos / linux), and skip it otherwise.\n\n\
# vibe:fp aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n\n\
schema = 1\n";
        assert_eq!(finish_index(prepared(), Some(FP), None), expected);

        let pending = finish_index(
            prepared(),
            Some(FP),
            Some(NativePendingFrame::new([0xbb; 32])),
        );
        assert!(pending.contains(&format!(
            "{FP_MARKER}{FP}\n{NATIVE_PENDING_MARKER}{PENDING}\n\n"
        )));
        let recorded = read_unit_index_freshness(&pending).expect("canonical pending header");
        assert_eq!(recorded.fingerprint, FP);
        assert_eq!(recorded.pending, Some(NativePendingFrame::new([0xbb; 32])));
        assert_eq!(
            super::super::read_fingerprint(&pending).as_deref(),
            Some(FP)
        );
        assert_eq!(
            super::super::read_fingerprint("  # vibe:fp cycle  \n").as_deref(),
            Some("cycle")
        );
    }

    #[test]
    fn strict_pending_codec_rejects_every_noncanonical_or_conflicting_shape() {
        let valid = format!("{FP_MARKER}{FP}\n{NATIVE_PENDING_MARKER}{PENDING}\n");
        for hostile in [
            format!("{FP_MARKER}{FP}\n{FP_MARKER}{FP}\n"),
            format!(
                "{FP_MARKER}{FP}\n{NATIVE_PENDING_MARKER}{PENDING}\n{NATIVE_PENDING_MARKER}{FP}\n"
            ),
            format!(
                "{FP_MARKER}{FP}\n{NATIVE_PENDING_MARKER}{}\n",
                PENDING.to_uppercase()
            ),
            format!("{FP_MARKER}{FP}\n{NATIVE_PENDING_MARKER}bb\n"),
            format!("{NATIVE_PENDING_MARKER}{PENDING}\n"),
            format!("{FP_MARKER}{FP}\n\n{NATIVE_PENDING_MARKER}{PENDING}\n"),
            format!(" {FP_MARKER}{FP}\n"),
            format!("{FP_MARKER}short\n"),
        ] {
            assert!(read_unit_index_freshness(&hostile).is_err(), "{hostile:?}");
        }
        assert!(read_unit_index_freshness(&valid).is_ok());
    }
}
