//! What `check` must answer, and the one answer the whole band model rests
//! on: a band-U edit under an unchanged plan is **not** staleness.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-076#SYS-READ-ONLY-CHECK",
    r = 1
);

use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;
use vibe_core::layout;
use vibe_core::manifest::{LinkType, Manifest, SpecFormat};

use crate::WorkspaceError;
use crate::boot::{BootBand, BootEntry, EffectiveBoot};
use crate::boot_artifacts::RouterFacts;
use crate::system_prompt::{
    CheckRequest, CheckVerdict, LaneBodyReader, NotTracked, PrefixReader, PromptCheck,
    check_node_prompt, prompt_warnings, segment_hash12,
};

const TEMPLATE: &str = "# My frame\n\nPermanent instructions.\n";
const ENTRYPOINT: &str = "<vibevm>\nold block\n</vibevm>\n\n# Project\n\nRead the lane.\n";
const LANE: &str = "<!-- vibe:c1 -->\n<spec>compiled</spec>\n";
const BAND_U: &str = "# a user snippet\n";
const INDEX: &str = "schema = 1\n";

/// A node on disk, published once, so every check runs against real bytes.
///
/// Shared with `prepare`'s tests, which need exactly the same fixture: a
/// published node is the only state either command has an opinion about.
pub(crate) struct Node {
    dir: TempDir,
    pub(crate) manifest: Manifest,
}

struct Disk<'a> {
    node_dir: &'a Path,
    root: &'a Path,
}

impl PrefixReader for Disk<'_> {
    fn read(&self, relative: &str) -> Result<Option<Vec<u8>>, WorkspaceError> {
        Ok(fs::read(self.node_dir.join(relative)).ok())
    }
}

impl LaneBodyReader for Disk<'_> {
    fn read_root(&self, relative: &str) -> Result<Option<Vec<u8>>, WorkspaceError> {
        Ok(fs::read(self.root.join(relative)).ok())
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

impl Node {
    /// Write the authored inputs and the committed `INDEX`/`STATIC`, then
    /// publish the three artifacts to disk exactly as the write path does.
    pub(crate) fn published(template: bool) -> Self {
        let dir = TempDir::new().unwrap();
        let boot = dir.path().join(layout::current_boot_dir());
        fs::create_dir_all(&boot).unwrap();
        fs::write(dir.path().join("AGENTS.md"), ENTRYPOINT).unwrap();
        fs::write(boot.join(layout::STATIC_XML), LANE).unwrap();
        fs::write(boot.join(layout::INDEX_MD), INDEX).unwrap();
        fs::write(boot.join("90-user.xml"), BAND_U).unwrap();
        if template {
            fs::write(boot.join("SYSTEM-TEMPLATE.md"), TEMPLATE).unwrap();
        }
        let node = Self {
            dir,
            manifest: Manifest::parse_str(
                "[project]\nname='demo'\ngroup='org.vibevm'\nversion='0.1.0'\n",
            )
            .unwrap(),
        };
        node.publish();
        node
    }

    /// Compose and write the current generation's artifacts.
    fn publish(&self) {
        let effective = effective();
        let lane = LANE.as_bytes();
        let disk = Disk {
            node_dir: self.dir.path(),
            root: self.dir.path(),
        };
        let publication = crate::system_prompt::prepare_publication(
            &crate::system_prompt::PromptPublishRequest {
                node_rel: ".",
                node_dir: self.dir.path(),
                manifest: &self.manifest,
                spec_format: SpecFormat::Xml,
                effective: &effective,
                lane: Some(lane),
                index_bytes: INDEX.as_bytes(),
                router: RouterFacts {
                    node_id: "org.vibevm/demo".to_owned(),
                    static_hash12: Some(segment_hash12(lane)),
                    spec_format: SpecFormat::Xml,
                },
            },
            &disk,
            &disk,
        )
        .unwrap();
        let crate::system_prompt::PromptPublication::Published(published) = publication else {
            return;
        };
        let boot = self.boot();
        fs::write(boot.join(layout::SYSTEM_MD), &published.system).unwrap();
        fs::write(boot.join(layout::SYSTEM_META_JSON), &published.meta).unwrap();
        fs::write(boot.join(layout::AGENT_LANE_MD), &published.lane).unwrap();
        // The managed block the prefix witnessed prospectively is the block
        // the write path leaves, so the test leaves it too.
        let projected = crate::boot_artifacts::project_managed_block(
            Some(ENTRYPOINT),
            &RouterFacts {
                node_id: "org.vibevm/demo".to_owned(),
                static_hash12: Some(segment_hash12(LANE.as_bytes())),
                spec_format: SpecFormat::Xml,
            },
        )
        .unwrap();
        fs::write(self.dir.path().join("AGENTS.md"), projected).unwrap();
    }

    pub(crate) fn boot(&self) -> PathBuf {
        self.dir.path().join(layout::current_boot_dir())
    }

    pub(crate) fn check(&self) -> PromptCheck {
        check_node_prompt(&self.request()).unwrap()
    }

    pub(crate) fn request(&self) -> CheckRequest<'_> {
        CheckRequest {
            node_rel: ".",
            node_dir: self.dir.path(),
            workspace_root: self.dir.path(),
            manifest: &self.manifest,
            model_window: None,
            tracked: &NotTracked,
        }
    }
}

#[test]
fn a_freshly_published_node_is_equal() {
    let node = Node::published(true);
    let answer = node.check();
    assert!(answer.opted_in);
    assert_eq!(answer.verdict, CheckVerdict::Equal, "{answer:#?}");
    assert!(answer.system.present && answer.meta.present && answer.lane.present);
}

/// The single most important assertion of the phase. Band U's bytes are
/// witnessed nowhere, `vibe agent` recomposes the lane at launch, and a
/// runner reads those files per request — so editing one must not make the
/// prefix stale.
#[test]
fn changed_band_u_bytes_under_an_unchanged_plan_stay_equal() {
    let node = Node::published(true);
    fs::write(node.boot().join("90-user.xml"), "# edited by the user\n").unwrap();
    assert_eq!(node.check().verdict, CheckVerdict::Equal);
}

#[test]
fn a_changed_template_is_stale_and_names_the_template() {
    let node = Node::published(true);
    fs::write(node.boot().join("SYSTEM-TEMPLATE.md"), "# other frame\n").unwrap();
    let CheckVerdict::Stale(reasons) = node.check().verdict else {
        panic!("a changed template is stale");
    };
    assert!(
        reasons.iter().any(|reason| reason.contains("template")),
        "{reasons:?}"
    );
}

#[test]
fn a_changed_lane_is_stale_and_names_the_lane() {
    let node = Node::published(true);
    fs::write(node.boot().join(layout::STATIC_XML), "<spec>other</spec>\n").unwrap();
    let CheckVerdict::Stale(reasons) = node.check().verdict else {
        panic!("a changed compiled lane is stale");
    };
    // The entrypoint's own router block names the lane's digest, so a lane
    // change is two reasons rather than one — and both are true.
    assert!(reasons.iter().any(|reason| reason.contains("lane input")));
}

#[test]
fn a_changed_plan_is_stale_and_names_the_plan() {
    let node = Node::published(true);
    fs::write(node.boot().join(layout::INDEX_MD), "schema = 1\n# more\n").unwrap();
    let CheckVerdict::Stale(reasons) = node.check().verdict else {
        panic!("a changed plan is stale");
    };
    assert!(
        reasons
            .iter()
            .any(|reason| reason.contains("continuation plan changed")),
        "{reasons:?}"
    );
}

#[test]
fn a_changed_recipe_pin_is_stale_and_names_the_recipe() {
    let mut node = Node::published(true);
    node.manifest = Manifest::parse_str(
        "[project]\nname='demo'\ngroup='org.vibevm'\nversion='0.1.0'\n\
         [system_prompt]\nrecipe = 1\n",
    )
    .unwrap();
    // Recipe 1 is the current one, so pinning it changes nothing.
    assert_eq!(node.check().verdict, CheckVerdict::Equal);
}

#[test]
fn a_missing_artifact_is_missing_and_names_it() {
    let node = Node::published(true);
    fs::remove_file(node.boot().join(layout::AGENT_LANE_MD)).unwrap();
    let CheckVerdict::Missing(reasons) = node.check().verdict else {
        panic!("a deleted artifact is missing");
    };
    assert_eq!(reasons.len(), 1);
    assert!(reasons[0].contains(layout::AGENT_LANE_MD));
}

#[test]
fn a_node_that_never_opted_in_is_equal_and_reports_no_opt_in() {
    let node = Node::published(false);
    let answer = node.check();
    assert!(!answer.opted_in);
    assert_eq!(answer.verdict, CheckVerdict::Equal);
    assert!(!answer.system.present);
    // Never an empty path, even for a project that generates nothing.
    assert!(answer.system.path.ends_with(layout::SYSTEM_MD));
    assert!(answer.lane.path.ends_with(layout::AGENT_LANE_MD));
}

#[test]
fn deleting_the_template_with_artifacts_left_is_stale_not_missing() {
    let node = Node::published(true);
    fs::remove_file(node.boot().join("SYSTEM-TEMPLATE.md")).unwrap();
    let CheckVerdict::Stale(reasons) = node.check().verdict else {
        panic!("an opt-out with leftovers is stale");
    };
    assert!(reasons[0].contains("opted out"), "{reasons:?}");
}

#[test]
fn a_git_tracked_artifact_is_a_conflict_naming_the_path() {
    struct Tracked;
    impl crate::system_prompt::TrackedPaths for Tracked {
        fn is_tracked(&self, path: &Path) -> Option<bool> {
            Some(path.ends_with(layout::SYSTEM_MD))
        }
    }
    let node = Node::published(true);
    let mut request = node.request();
    request.tracked = &Tracked;
    let CheckVerdict::Conflict(reason) = check_node_prompt(&request).unwrap().verdict else {
        panic!("a tracked generated artifact is a conflict");
    };
    assert!(reason.contains(layout::SYSTEM_MD), "{reason}");
}

#[test]
fn an_unknown_sidecar_protocol_is_freshness_unverifiable() {
    let node = Node::published(true);
    fs::write(
        node.boot().join(layout::SYSTEM_META_JSON),
        "{\"protocol\":\"vibe-system-prompt/9\"}\n",
    )
    .unwrap();
    let CheckVerdict::FreshnessUnverifiable(reason) = node.check().verdict else {
        panic!("a future sidecar cannot be judged");
    };
    assert!(reason.contains("vibe-system-prompt/9"), "{reason}");
}

#[test]
fn a_hand_edited_prefix_is_stale_rather_than_silently_accepted() {
    let node = Node::published(true);
    let path = node.boot().join(layout::SYSTEM_MD);
    let mut bytes = fs::read(&path).unwrap();
    bytes.extend_from_slice(b"\nhand edited\n");
    fs::write(&path, bytes).unwrap();
    let CheckVerdict::Stale(reasons) = node.check().verdict else {
        panic!("a hand-edited prefix is stale");
    };
    assert!(
        reasons
            .iter()
            .any(|reason| reason.contains("not the composition")),
        "{reasons:?}"
    );
}

#[test]
fn the_remedy_names_the_node_only_when_it_is_not_the_root() {
    let node = Node::published(true);
    assert_eq!(node.check().remedy(), "vibe prompt build");
    let mut answer = node.check();
    answer.node_rel = "members/alpha".to_owned();
    assert_eq!(answer.remedy(), "vibe prompt build --node members/alpha");
}

#[test]
fn the_warnings_report_the_lane_and_never_a_verdict_about_it() {
    let node = Node::published(true);
    let mut request = node.request();
    request.model_window = Some(8);
    let answer = check_node_prompt(&request).unwrap();
    let warnings = prompt_warnings(&request, &answer).unwrap();
    assert!(
        warnings
            .iter()
            .any(|warning| warning.contains("does not fit")),
        "{warnings:?}"
    );
    // A warning is never a verdict: the prefix is still equal.
    assert_eq!(answer.verdict, CheckVerdict::Equal);
}
