//! Locked workspace body resolution and explicit unavailable reasons.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-WORKER-TYPES");

use std::fs;

use super::{AgentTypeBody, AgentTypeSources, LockedSlot, builtin_agent_types};
use crate::vibedeps;
use vibe_core::Group;

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-076#SYS-WORKER-TYPES")]
fn executor_rules_come_from_the_locked_working_rules_slot() {
    let workspace = tempfile::tempdir().unwrap();
    let root = workspace.path();
    let group = Group::parse("org.vibevm.world").unwrap();
    let version = "1.0.12".parse().unwrap();
    let slot = vibedeps::slot_abs_path(root, &group, "working-rules", &version);
    fs::create_dir_all(slot.join("rules")).unwrap();
    fs::write(slot.join("rules/worker.md"), b"RULES\n").unwrap();
    let sources = AgentTypeSources {
        lane: Ok(b"PROJECT LANE\n".to_vec()),
        workspace_root: root.to_path_buf(),
        locked: Ok(vec![LockedSlot {
            group: group.clone(),
            name: "working-rules".into(),
            version,
        }]),
    };
    let executor = &builtin_agent_types()[1].body;
    assert_eq!(sources.resolve(executor), Ok(b"RULES\n".to_vec()));
    assert_eq!(
        sources.resolve(&AgentTypeBody::ProjectLane),
        Ok(b"PROJECT LANE\n".to_vec())
    );

    // An in-place package uses its unversioned working tree, not the copy slot.
    let in_place = vibedeps::in_place_slot_abs_path(root, &group, "working-rules");
    fs::write(in_place.join(".git"), b"gitdir: elsewhere\n").unwrap();
    fs::create_dir_all(in_place.join("rules")).unwrap();
    fs::write(in_place.join("rules/worker.md"), b"IN-PLACE\r\n\xff").unwrap();
    assert_eq!(sources.resolve(executor), Ok(b"IN-PLACE\r\n\xff".to_vec()));
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-076#SYS-WORKER-TYPES")]
fn missing_package_and_missing_lane_are_unavailable() {
    let workspace = tempfile::tempdir().unwrap();
    let mut sources = AgentTypeSources {
        lane: Err("x".into()),
        workspace_root: workspace.path().to_path_buf(),
        locked: Ok(vec![]),
    };
    let executor = &builtin_agent_types()[1].body;
    assert_eq!(
        sources.resolve(executor),
        Err("`org.vibevm.world/working-rules` is not installed in this project, so this type has no body; installing it provides one".into())
    );
    sources.locked = Err("no lock".into());
    assert_eq!(sources.resolve(executor), Err("no lock".into()));

    let group = Group::parse("org.vibevm.world").unwrap();
    let version = "1.0.12".parse().unwrap();
    let slot = vibedeps::slot_abs_path(workspace.path(), &group, "working-rules", &version);
    fs::create_dir_all(slot.join("rules")).unwrap();
    sources.locked = Ok(vec![LockedSlot {
        group,
        name: "working-rules".into(),
        version,
    }]);
    let file = slot.join("rules/worker.md");
    assert_eq!(
        sources.resolve(executor),
        Err(format!(
            "`{}` is missing; run `vibe install`",
            crate::path_to_slash(file.strip_prefix(workspace.path()).unwrap())
        ))
    );
    assert_eq!(
        sources.resolve(&AgentTypeBody::ProjectLane),
        Err("x".into())
    );
}
