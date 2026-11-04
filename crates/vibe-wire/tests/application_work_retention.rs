//! Published output-only work readers: old rows unchanged, new metadata inspectable.
#[path = "fixtures/application_work_e1.rs"]
mod old;
use serde_json::json;
use vibe_wire::generated::application_work as current;

fn row() -> serde_json::Value {
    json!({"protocol":"vibe-app-work/1","home":"/home/cache/app-work","entries":[{"key":"org.example/child@1.0.0","path":"/home/cache/app-work/org.example.child@1.0.0","state":"done","changed":"now","bytes":0,"parts":{"sources":0,"out":0,"inputs":0,"other":0}}],"freed":0})
}
#[test]
fn old_rows_keep_their_shape_and_old_public_reader_accepts_new_metadata() {
    let legacy = row();
    let old: old::ApplicationWork = serde_json::from_value(legacy.clone()).unwrap();
    let new: current::ApplicationWork = serde_json::from_value(legacy).unwrap();
    assert_eq!(
        serde_json::to_value(old).unwrap(),
        serde_json::to_value(new).unwrap()
    );
    let mut rich = row();
    let target = json!({"home":"/home","key":"org.example/parent@1.0.0","operation":"generation-7","future":true});
    rich["entries"][0]["retainedFor"] =
        json!({"owner":{"kind":"application","target":target.clone()},"futureWrapper":true});
    rich["imports"] = json!([{"target":{"home":"/home","operation":"import-9","future":true},"path":"/home/cache/app-work/imports/import-9","phase":"running","cursor":1,"total":3,"changed":"now","bytes":1,"current":target.clone()}]);
    rich["entries"][0]["operation"] = json!({"target":target,"phase":"complete"});
    let old: old::ApplicationWork = serde_json::from_value(rich.clone()).unwrap();
    assert_eq!(old.entries[0].state, old::EntryState::Done);
    let new: current::ApplicationWork = serde_json::from_value(rich).unwrap();
    assert!(
        matches!(&new.entries[0].retained_for.as_ref().unwrap().owner,vibe_wire::generated::shared::ApplicationContinuationTarget::Application(owner) if owner.target.operation=="generation-7")
    );
    assert_eq!(new.imports[0].target.operation, "import-9");
    assert_eq!(
        new.imports[0].current.as_ref().unwrap().operation,
        "generation-7"
    );
    assert_eq!(
        new.entries[0].operation.as_ref().unwrap().phase,
        vibe_wire::generated::shared::ApplicationPendingOperationPhase::Complete
    );
    assert!(!serde_json::to_string(&new).unwrap().contains("future"));
}
