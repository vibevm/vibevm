use super::*;
use std::os::unix::fs::PermissionsExt;
fn home() -> tempfile::TempDir {
    let t = tempfile::tempdir().unwrap();
    std::fs::set_permissions(t.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    t
}
#[test]
fn all_fixed_paths_observe_without_effects_then_create_private_components() {
    for (kind, path) in [
        (Kind::Pi, ".pi/agent"),
        (Kind::Qwen, ".qwen"),
        (Kind::Codex, ".codex"),
        (Kind::OpenCode, ".config/opencode"),
        (Kind::Hermes, ".hermes"),
        (Kind::Claude, ".claude"),
    ] {
        let t = home();
        let target = observe(t.path(), kind).unwrap();
        assert!(!target.exists);
        assert!(!t.path().join(path).exists());
        let witness = target.witness.clone();
        assert_eq!(observe(t.path(), kind).unwrap().witness, witness);
        let (_, dir) = target.create().unwrap();
        dir.verify_user_private().unwrap();
        assert!(t.path().join(path).is_dir());
        assert_ne!(observe(t.path(), kind).unwrap().witness, witness);
    }
}
#[test]
fn second_component_appearance_and_symlink_refuse_without_adoption() {
    let t = home();
    let pi = t.path().join(".pi");
    std::fs::create_dir(&pi).unwrap();
    std::fs::set_permissions(&pi, std::fs::Permissions::from_mode(0o700)).unwrap();
    let target = observe(t.path(), Kind::Pi).unwrap();
    assert!(!target.exists);
    let old = target.witness.clone();
    let occupant = pi.join("agent");
    std::fs::create_dir(&occupant).unwrap();
    std::fs::set_permissions(&occupant, std::fs::Permissions::from_mode(0o700)).unwrap();
    std::fs::write(occupant.join("foreign"), b"KEEP").unwrap();
    assert_ne!(observe(t.path(), Kind::Pi).unwrap().witness, old);
    assert!(target.create().is_err());
    assert_eq!(std::fs::read(occupant.join("foreign")).unwrap(), b"KEEP");
    let t = home();
    let elsewhere = home();
    std::os::unix::fs::symlink(elsewhere.path(), t.path().join(".qwen")).unwrap();
    assert!(observe(t.path(), Kind::Qwen).is_err());
    assert!(
        std::fs::read_dir(elsewhere.path())
            .unwrap()
            .next()
            .is_none()
    );
}
