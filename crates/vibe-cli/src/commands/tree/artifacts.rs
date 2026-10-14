//! Decompilers for the committed boot artifacts (PROP-036 §2.8, §3).
//!
//! Two pure readers, no filesystem access — the caller hands in the file
//! text:
//!
//! - [`decompile_static`] parses the static lane's on-disk `vibe:static`
//!   open-marker format into its contributions. This is a **dedicated**
//!   decompiler — it is NOT `vibe_spec::decompile()`, which parses the
//!   distinct `vibe:begin`/`vibe:end` compiler format and returns empty on
//!   `STATIC.md` (PROP-036 §2.8).
//! - [`read_index`] parses the generated `INDEX.md` TOML manifest into its
//!   ordered `[[entry]]` list.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-cli/PROP-036#static-decompile");

use anyhow::{Context, Result};

use super::model::{IndexEntry, IndexKind};

pub use vibe_workspace::boot_debugger::decompile_static;
#[cfg(test)]
use vibe_workspace::boot_debugger::{StaticDecompileError, capture_static};
#[cfg(test)]
const MARKER_SEP: &str = " — ";

/// The parsed `INDEX.md` lane: the `static` pointer and the ordered entries.
pub struct IndexParse {
    pub static_pointer: Option<String>,
    pub entries: Vec<IndexEntry>,
}

/// Read the generated `INDEX.md` TOML into its ordered entry list.
pub fn read_index(text: &str) -> Result<IndexParse> {
    let toml =
        vibe_workspace::boot_debugger::dynamic_index::read_index(text).with_context(|| {
            format!(
                "parsing the generated {} manifest",
                vibe_core::machine_json_path(&vibe_core::layout::current_boot_index())
            )
        })?;
    let entries = toml
        .entries
        .into_iter()
        .enumerate()
        .map(|(order, e)| IndexEntry {
            order: order as u64,
            path: e.path,
            kind: if e.kind == "dynamic" {
                IndexKind::Dynamic
            } else {
                IndexKind::Static
            },
            when: e.when,
        })
        .collect();
    Ok(IndexParse {
        static_pointer: toml.static_pointer,
        entries,
    })
}

/// Map a boot-file path to the `(group, name)` of the package that owns it,
/// or `None` for a host-authored path. A materialised slot path lives under
/// the live layout's deps root (`<deps root>/<group>.<name>/<version>/…`);
/// the second component encodes the package identity.
pub fn slot_package(path: &str) -> Option<(String, String)> {
    // Component-wise strip, so both separator spellings work and the R4
    // flip's two-component root matches in one comparison.
    let rest = std::path::Path::new(path)
        .strip_prefix(vibe_core::layout::current_vibedeps_root())
        .ok()?;
    let rest = rest.to_string_lossy().replace('\\', "/");
    let slot = rest.split('/').next()?;
    // The slot is identity-keyed, `<group>.<name>` (PROP-022 §2.1): the name
    // is a single dot-free LDH label, so the last dot is always the boundary.
    let (group, name) = slot.rsplit_once('.')?;
    Some((group.to_string(), name.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c1(payload: &str) -> String {
        format!(
            "<!-- vibe:c1 {} -->",
            vibe_specdoc::encode_generated_xml_comment(payload)
        )
    }

    /// A materialised slot file on the live layout:
    /// `<deps root>/<slot>/<version>/<tail>`. Built from the layout
    /// module so every fixture here rides the R4 flip unchanged.
    fn slot_file(slot: &str, version: &str, tail: impl AsRef<std::path::Path>) -> String {
        vibe_core::machine_json_path(
            &vibe_core::layout::current_vibedeps_root()
                .join(slot)
                .join(version)
                .join(tail.as_ref()),
        )
    }

    /// A package's boot-lane file inside its slot — the tail a
    /// materialised boot snippet carries.
    fn slot_boot(slot: &str, version: &str, file: &str) -> String {
        slot_file(
            slot,
            version,
            vibe_core::layout::current_specs_root()
                .join(vibe_core::layout::BOOT_DIR)
                .join(file),
        )
    }

    /// A host boot-lane file on the live layout.
    fn host_boot(file: &str) -> String {
        vibe_core::machine_json_path(&vibe_core::layout::current_boot_dir().join(file))
    }

    #[test]
    fn decompiles_two_contributions_with_bodies() {
        let one = slot_boot("org.vibevm.world.addressable-specs", "0.1.0", "15.md");
        let two = slot_boot("org.vibevm.world.redbook", "0.2.0", "03.md");
        let text = format!(
            "\
<!-- header -->

<!-- vibe:static org.vibevm.world/addressable-specs \u{2014} {one} -->

# Addressable Specs

body line
<!-- vibe:static org.vibevm.world/redbook \u{2014} {two} -->

# Redbook
"
        );
        let c = decompile_static(&text).unwrap();
        assert_eq!(c.len(), 2);
        assert_eq!(c[0].origin, "org.vibevm.world/addressable-specs");
        assert_eq!(c[0].source_path, one);
        assert_eq!(c[0].order, 0);
        assert_eq!(c[1].origin, "org.vibevm.world/redbook");
        assert_eq!(c[1].order, 1);
        // The first region's body has non-zero lines (the second marker
        // bounds it).
        assert!(c[0].lines > 0);
        assert!(c[0].embeds.is_empty());
    }

    #[test]
    fn a_host_rel_path_origin_is_kept_verbatim() {
        let core = host_boot("00-core.md");
        let text = format!("<!-- vibe:static {core} \u{2014} {core} -->\nbody\n");
        let c = decompile_static(&text).unwrap();
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].origin, core);
    }

    #[test]
    fn a_non_marker_line_is_not_a_contribution() {
        // A plain heading or an em-dash-free comment is not a marker.
        let text = "# Heading\n<!-- vibe:static no-separator-here -->\ntext\n";
        assert!(decompile_static(text).unwrap().is_empty());
    }

    /// R4 architecture §7.1 — the active-transforms header is SKIPPABLE
    /// non-provenance to this decompiler: a header-bearing tape decompiles
    /// to exactly the contribution set of its header-free twin.
    ///
    /// Classification here is per-kind (only `vibe:static ` payloads become
    /// markers), so the header needs no code — but the codec-encoded token
    /// (`%2D` for the `--` a package name may carry) means the line is also
    /// not mistaken for anything else, and this pin is what keeps a future
    /// marker-widening honest about that.
    #[test]
    fn a_transforms_header_is_skippable_and_never_a_contribution() {
        let spec = slot_file("org.vibevm.world.x", "0.1.0", "b.md");
        let contribution =
            format!("<!-- vibe:static org.vibevm.world/x \u{2014} {spec} -->\n\nbody\n");
        let plain = format!("<!-- header -->\n\n{contribution}");
        let headered = format!(
            "<!-- header -->\n<!-- vibe:transforms org.demo/tools#first org.demo/a-%2Db#second -->\n\n{contribution}"
        );
        let observed = |text: &str| -> Vec<(String, String, u64, usize)> {
            decompile_static(text)
                .unwrap()
                .into_iter()
                .map(|c| (c.origin, c.source_path, c.order, c.embeds.len()))
                .collect()
        };
        assert_eq!(
            observed(&headered),
            observed(&plain),
            "the header adds no contribution and drops none"
        );
    }

    #[test]
    fn attributes_a_nested_embed_span() {
        let spec = slot_file("org.vibevm.world.x", "0.1.0", "b.md");
        let text = format!(
            "\
<!-- vibe:static org.vibevm.world/x \u{2014} {spec} -->

<!-- embed: spec://org.vibevm.core/vibevm/a/b#c -->
inner
<!-- /embed: spec://org.vibevm.core/vibevm/a/b#c -->
"
        );
        let c = decompile_static(&text).unwrap();
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].embeds.len(), 1);
        assert_eq!(
            c[0].embeds[0].address,
            "spec://org.vibevm.core/vibevm/a/b#c"
        );
        assert!(c[0].embeds[0].start_line < c[0].embeds[0].end_line);
    }

    #[test]
    fn reads_index_entries_in_order() {
        let static_lane =
            vibe_core::machine_json_path(&vibe_core::layout::current_boot_static_md());
        let core = host_boot("00-core.md");
        let dyn_entry = slot_boot("org.vibevm.ai-native.rust-ai-native-lang", "0.7.0", "20.md");
        let text = format!(
            "\
schema = 1
static = \"{static_lane}\"

[[entry]]
path = \"{core}\"
kind = \"static\"

[[entry]]
path = \"{dyn_entry}\"
kind = \"dynamic\"
when = \"os:linux\"
"
        );
        let parsed = read_index(&text).unwrap();
        assert_eq!(parsed.static_pointer.as_deref(), Some(static_lane.as_str()));
        assert_eq!(parsed.entries.len(), 2);
        assert_eq!(parsed.entries[0].order, 0);
        assert_eq!(parsed.entries[0].kind, IndexKind::Static);
        assert_eq!(parsed.entries[1].kind, IndexKind::Dynamic);
        assert_eq!(parsed.entries[1].when.as_deref(), Some("os:linux"));
    }

    #[test]
    fn maps_a_slot_path_to_its_package() {
        assert_eq!(
            slot_package(&slot_boot(
                "org.vibevm.ai-native.rust-ai-native-lang",
                "0.7.0",
                "20.md"
            )),
            Some((
                "org.vibevm.ai-native".to_string(),
                "rust-ai-native-lang".to_string()
            ))
        );
        assert_eq!(
            slot_package(&slot_boot("org.vibevm.world.redbook", "0.2.0", "03.md")),
            Some(("org.vibevm.world".to_string(), "redbook".to_string()))
        );
        // A host-authored path maps to no package.
        assert_eq!(slot_package(&host_boot("00-core.md")), None);
    }

    #[test]
    fn c1_provenance_decodes_exact_origin_and_path() {
        let origin = "org.demo/a--b-%雪";
        let path = "dir/a--b-/x%2D&雪.xml";
        let logical = format!("vibe:static {origin}{MARKER_SEP}{path}");
        let text = format!("{}\n\n<spec/>\n", c1(&logical));
        let contributions = decompile_static(&text).unwrap();
        assert_eq!(contributions.len(), 1);
        assert_eq!(contributions[0].origin, origin);
        assert_eq!(contributions[0].source_path, path);
    }

    #[test]
    fn malformed_unknown_and_noncanonical_c1_fail_instead_of_disappearing() {
        for text in [
            "<!-- vibe:c2 vibe:static origin — path -->\n",
            "<!-- vibe:c1 vibe:static origin — path% -->\n",
            "<!-- vibe:c1 vibe:static %41 — path -->\n",
        ] {
            let error = decompile_static(text).expect_err(text);
            assert!(
                matches!(error, StaticDecompileError::Codec { .. }),
                "{error}"
            );
        }
    }

    #[test]
    fn canonical_c1_provenance_requires_separator_and_nonempty_fields() {
        for (payload, reason) in [
            ("vibe:static origin", "separator"),
            ("vibe:static origin - path", "separator"),
            ("vibe:static  — path", "origin is empty"),
            ("vibe:static origin —  ", "path is empty"),
        ] {
            let text = c1(payload);
            let error = decompile_static(&text).expect_err(payload);
            assert!(
                matches!(
                    error,
                    StaticDecompileError::MalformedGeneratedProvenance { .. }
                ),
                "{payload:?}: {error}"
            );
            assert!(error.to_string().contains(reason), "{payload:?}: {error}");
        }
    }

    #[test]
    fn canonical_non_provenance_c1_genres_remain_skippable() {
        for payload in [
            "generated header",
            "RESOLUTION RULES — body",
            "RENAMED ANCHORS (short → heirs)",
            "vibe:hoisted org.demo/pkg — root",
            "vibe:transforms xml-minify",
        ] {
            assert!(
                decompile_static(&c1(payload)).unwrap().is_empty(),
                "{payload}"
            );
        }
    }

    #[test]
    fn every_unterminated_comment_is_a_typed_error_regardless_of_c1_spacing() {
        for text in [
            "<!-- vibe:c1 unterminated",
            "<!--vibe:c1 unterminated",
            "<!--  vibe:c1 unterminated",
            "<!--\nvibe:c1 unterminated",
            "<!-- authored but unterminated",
        ] {
            let error = decompile_static(text).expect_err(text);
            assert!(
                matches!(
                    error,
                    StaticDecompileError::UnterminatedXmlComment { line: 1 }
                ),
                "{text:?}: {error}"
            );
        }
    }

    #[test]
    fn multiline_c1_provenance_keeps_fields_and_body_line_accounting() {
        let origin = "org.demo/\nalpha";
        let path = "dir/\nentry.xml";
        let payload = format!("vibe:static {origin}{MARKER_SEP}{path}");
        let text = format!("{}\nBODY-A\nBODY-B\n", c1(&payload));

        let contributions = decompile_static(&text).unwrap();
        assert_eq!(contributions.len(), 1);
        assert_eq!(contributions[0].origin, origin);
        assert_eq!(contributions[0].source_path, path);
        assert_eq!(contributions[0].lines, 2);
        assert_eq!(contributions[0].bytes, "BODY-A\nBODY-B\n".len() as u64);
    }
}

#[cfg(test)]
#[path = "m_06_b_1_cases.rs"]
mod m_06_b_1_cases;
