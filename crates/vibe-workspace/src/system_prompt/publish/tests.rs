//! What a publication decides, and the three answers it must keep apart.
//!
//! `Untouched`, `Removed` and `Published` are different statements, and the
//! bug this cell exists to prevent is any two of them collapsing: a project
//! that never opted in must not have its boot directory written to, and a
//! project that opted *out* must have exactly the four owned paths retired
//! and nothing else.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-076#SYS-TRANSACTION-ROLES",
    r = 1
);

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fs;

use tempfile::TempDir;
use vibe_core::manifest::{LinkType, Manifest, SpecFormat};

use super::*;
use crate::boot::{BootBand, BootEntry, EffectiveBoot};

const TEMPLATE: &str = "# My frame\n\nPermanent instructions.\n";
const ENTRYPOINT: &str = "# Project\n\nRead the lane.\n\n<vibevm>\nold block\n</vibevm>\n";
const LANE: &str = "<!-- vibe:c1 -->\n<spec>compiled</spec>\n";
const BAND_U: &str = "# a user snippet\n";

struct Files {
    node: BTreeMap<String, Vec<u8>>,
    root: BTreeMap<String, Vec<u8>>,
    reads: RefCell<Vec<String>>,
}

impl Files {
    fn new(template: bool) -> Self {
        let mut node: BTreeMap<String, Vec<u8>> = BTreeMap::new();
        node.insert("AGENTS.md".to_owned(), ENTRYPOINT.as_bytes().to_vec());
        if template {
            node.insert(
                crate::path_to_slash(&layout::current_boot_system_template()),
                TEMPLATE.as_bytes().to_vec(),
            );
        }
        let mut root = BTreeMap::new();
        root.insert(band_u_path(), BAND_U.as_bytes().to_vec());
        Self {
            node,
            root,
            reads: RefCell::new(Vec::new()),
        }
    }
}

impl PrefixReader for Files {
    fn read(&self, relative: &str) -> Result<Option<Vec<u8>>, WorkspaceError> {
        self.reads.borrow_mut().push(relative.to_owned());
        Ok(self.node.get(relative).cloned())
    }
}

impl LaneBodyReader for Files {
    fn read_root(&self, relative: &str) -> Result<Option<Vec<u8>>, WorkspaceError> {
        self.reads.borrow_mut().push(format!("//{relative}"));
        Ok(self.root.get(relative).cloned())
    }
}

fn band_u_path() -> String {
    crate::path_to_slash(&layout::current_boot_dir().join("90-user.xml"))
}

fn effective() -> EffectiveBoot {
    let mut boot = EffectiveBoot::default();
    boot.entries.push(BootEntry {
        path: band_u_path(),
        band: BootBand::NodeOwn,
        link: LinkType::Dynamic,
        when: None,
        origin: "org.vibevm/demo".to_owned(),
        provenance: crate::boot::fixture_provenance("org.vibevm/demo"),
        use_ref: false,
        format: Default::default(),
        unit_substituted: false,
        elided: false,
    });
    boot
}

fn manifest() -> Manifest {
    Manifest::parse_str("[project]\nname='demo'\ngroup='org.vibevm'\nversion='0.1.0'\n").unwrap()
}

fn decide(dir: &TempDir, files: &Files, manifest: &Manifest) -> PromptPublication {
    let effective = effective();
    let lane = LANE.as_bytes();
    let request = PromptPublishRequest {
        node_rel: ".",
        node_dir: dir.path(),
        manifest,
        spec_format: SpecFormat::Xml,
        effective: &effective,
        lane: Some(lane),
        index_bytes: b"schema = 1\n",
        router: RouterFacts {
            node_id: "org.vibevm/demo".to_owned(),
            static_hash12: Some(super::super::segment_hash12(lane)),
            spec_format: SpecFormat::Xml,
        },
    };
    prepare_publication(&request, files, files).unwrap()
}

#[test]
fn an_opted_in_node_publishes_the_prefix_its_sidecar_and_the_agent_lane() {
    let dir = TempDir::new().unwrap();
    let files = Files::new(true);
    let manifest = manifest();
    let PromptPublication::Published(published) = decide(&dir, &files, &manifest) else {
        panic!("a node with a template has opted in");
    };
    let prefix = String::from_utf8(published.system.clone()).unwrap();
    // Band order: the template, the entrypoint export, the lane.
    let template_at = prefix.find("My frame").unwrap();
    let entry_at = prefix.find("Read the lane.").unwrap();
    let lane_at = prefix.find("<spec>compiled</spec>").unwrap();
    assert!(template_at < entry_at && entry_at < lane_at);
    // The managed block is gone and no router survived into the prefix.
    assert!(!prefix.contains("old block"));
    assert!(!prefix.contains("\n<vibevm>\n"));

    let meta = String::from_utf8(published.meta.clone()).unwrap();
    assert!(meta.contains("\"protocol\": \"vibe-system-prompt/1\""));
    assert!(meta.ends_with("}\n"));

    let lane = String::from_utf8(published.lane.clone()).unwrap();
    // The lane is the lane part of band P plus the whole of band U, and it
    // carries neither the template nor the entrypoint.
    assert!(lane.contains("<spec>compiled</spec>"));
    assert!(lane.contains(BAND_U));
    assert!(!lane.contains("My frame"));
    assert!(!lane.contains("Read the lane."));
    assert!(super::super::verify_lane_markers(&published.lane).is_ok());
}

#[test]
fn composition_opens_exactly_the_two_authored_inputs_and_the_plan_s_own_rows() {
    let dir = TempDir::new().unwrap();
    let files = Files::new(true);
    let manifest = manifest();
    decide(&dir, &files, &manifest);
    assert_eq!(
        files.reads.into_inner(),
        [
            crate::path_to_slash(&layout::current_boot_system_template()),
            "AGENTS.md".to_owned(),
            format!("//{}", band_u_path()),
        ]
    );
}

#[test]
fn a_node_with_no_template_and_no_leftover_artifact_is_untouched() {
    let dir = TempDir::new().unwrap();
    let files = Files::new(false);
    let manifest = manifest();
    assert_eq!(
        decide(&dir, &files, &manifest),
        PromptPublication::Untouched
    );
}

#[test]
fn a_node_that_opted_out_with_a_leftover_artifact_removes_it() {
    for name in owned_names() {
        let dir = TempDir::new().unwrap();
        let boot = dir.path().join(layout::current_boot_dir());
        fs::create_dir_all(&boot).unwrap();
        fs::write(boot.join(name), b"left over\n").unwrap();
        let files = Files::new(false);
        let manifest = manifest();
        assert_eq!(
            decide(&dir, &files, &manifest),
            PromptPublication::Removed,
            "{name}"
        );
    }
}

#[test]
fn a_plan_row_whose_file_is_absent_refuses_rather_than_delivering_a_short_lane() {
    let dir = TempDir::new().unwrap();
    let mut files = Files::new(true);
    files.root.clear();
    let effective = effective();
    let manifest = manifest();
    let lane = LANE.as_bytes();
    let request = PromptPublishRequest {
        node_rel: ".",
        node_dir: dir.path(),
        manifest: &manifest,
        spec_format: SpecFormat::Xml,
        effective: &effective,
        lane: Some(lane),
        index_bytes: b"schema = 1\n",
        router: RouterFacts {
            node_id: "org.vibevm/demo".to_owned(),
            static_hash12: Some(super::super::segment_hash12(lane)),
            spec_format: SpecFormat::Xml,
        },
    };
    let error = prepare_publication(&request, &files, &files).unwrap_err();
    assert!(error.to_string().contains("the file is absent"), "{error}");
}

#[test]
fn the_ignore_file_covers_the_four_owned_paths_and_never_the_boot_lane() {
    for name in owned_names() {
        assert!(BOOT_IGNORE_BODY.contains(&format!("\n/{name}\n")), "{name}");
    }
    assert!(!BOOT_IGNORE_BODY.contains("/INDEX.md"));
    assert!(!BOOT_IGNORE_BODY.contains("/STATIC"));
    // Nothing volatile, and nothing absolute: it is a constant.
    assert!(!BOOT_IGNORE_BODY.contains('\\'));
    assert!(BOOT_IGNORE_BODY.ends_with('\n'));
}
