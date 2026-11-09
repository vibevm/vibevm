//! The Windows persister logic over the in-memory user registry: these run on
//! every host, so the `HKCU\Environment` behaviour is gated on Linux too.

use std::path::Path;

use super::super::profile::{ObservationStatus, PathEntryStatus};
use super::super::{EnvPersister, Persisted};
use super::{MemoryRegistry, RawUserVar, RegistryEnvPersister, RegistryValueKind};

const SHIMS: &str = r"C:\Users\me\.vibe\bin";

fn persister(values: &[(&str, &str, RegistryValueKind)]) -> RegistryEnvPersister<MemoryRegistry> {
    RegistryEnvPersister::new(MemoryRegistry::with(values))
}

fn path_of(persister: &RegistryEnvPersister<MemoryRegistry>) -> Option<RawUserVar> {
    persister.registry().value("Path")
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-019#path", r = 3)]
fn a_missing_user_path_is_created_expandable_with_only_the_shim_dir() {
    let persister = persister(&[]);
    let before = persister.inspect_path(Path::new(SHIMS));
    assert_eq!(before.observation, ObservationStatus::Missing);
    assert_eq!(before.path_entry, PathEntryStatus::Missing);

    assert_eq!(
        persister.ensure_on_path(Path::new(SHIMS)).unwrap(),
        Persisted::Changed
    );
    assert_eq!(
        path_of(&persister),
        Some(RawUserVar {
            value: SHIMS.into(),
            kind: RegistryValueKind::ExpandString,
        })
    );
    assert_eq!(persister.registry().writes(), 1);
    assert_eq!(
        persister.inspect_path(Path::new(SHIMS)).path_entry,
        PathEntryStatus::Correct
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-019#path", r = 3)]
fn the_shim_dir_is_prepended_and_unexpanded_entries_kept_verbatim() {
    let persister = persister(&[(
        "Path",
        r"%USERPROFILE%\AppData\Local\Microsoft\WindowsApps;C:\Tools",
        RegistryValueKind::ExpandString,
    )]);
    assert_eq!(
        persister.inspect_path(Path::new(SHIMS)).path_entry,
        PathEntryStatus::Different
    );
    assert_eq!(
        persister.ensure_on_path(Path::new(SHIMS)).unwrap(),
        Persisted::Changed
    );
    assert_eq!(
        path_of(&persister),
        Some(RawUserVar {
            value: format!(r"{SHIMS};%USERPROFILE%\AppData\Local\Microsoft\WindowsApps;C:\Tools"),
            kind: RegistryValueKind::ExpandString,
        })
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-019#path", r = 3)]
fn a_reg_sz_path_stays_reg_sz() {
    let persister = persister(&[("PATH", r"C:\Tools", RegistryValueKind::String)]);
    persister.ensure_on_path(Path::new(SHIMS)).unwrap();
    assert_eq!(
        path_of(&persister),
        Some(RawUserVar {
            value: format!(r"{SHIMS};C:\Tools"),
            kind: RegistryValueKind::String,
        })
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-019#path", r = 3)]
fn an_equivalent_entry_elsewhere_is_moved_to_the_front_case_insensitively() {
    let persister = persister(&[(
        "Path",
        r"C:\Tools;c:\users\ME\.VIBE\bin\;D:\x",
        RegistryValueKind::ExpandString,
    )]);
    assert_eq!(
        persister.ensure_on_path(Path::new(SHIMS)).unwrap(),
        Persisted::Changed
    );
    assert_eq!(
        path_of(&persister).map(|raw| raw.value),
        Some(format!(r"{SHIMS};C:\Tools;D:\x"))
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-019#path", r = 3)]
fn a_canonical_path_is_never_rewritten() {
    let persister = persister(&[(
        "Path",
        &format!(r"{SHIMS};%USERPROFILE%\bin"),
        RegistryValueKind::ExpandString,
    )]);
    assert_eq!(
        persister.inspect_path(Path::new(SHIMS)).path_entry,
        PathEntryStatus::Correct
    );
    assert_eq!(
        persister.ensure_on_path(Path::new(SHIMS)).unwrap(),
        Persisted::Unchanged
    );
    assert_eq!(persister.registry().writes(), 0);

    let fresh = self::persister(&[("Path", r"C:\Tools", RegistryValueKind::ExpandString)]);
    assert_eq!(
        fresh.ensure_on_path(Path::new(SHIMS)).unwrap(),
        Persisted::Changed
    );
    assert_eq!(
        fresh.ensure_on_path(Path::new(SHIMS)).unwrap(),
        Persisted::Unchanged,
        "the second run finds its own result canonical"
    );
    assert_eq!(fresh.registry().writes(), 1);
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-019#path", r = 3)]
fn vibevm_home_keeps_its_kind_and_skips_identical_writes() {
    let persister = persister(&[("VIBEVM_HOME", r"D:\old", RegistryValueKind::ExpandString)]);
    assert_eq!(
        persister.set_vibevm_home(Path::new(r"D:\new")).unwrap(),
        Persisted::Changed
    );
    assert_eq!(
        persister.registry().value("vibevm_home"),
        Some(RawUserVar {
            value: r"D:\new".into(),
            kind: RegistryValueKind::ExpandString,
        })
    );
    assert_eq!(
        persister.set_vibevm_home(Path::new(r"D:\new")).unwrap(),
        Persisted::Unchanged
    );
    assert_eq!(persister.registry().writes(), 1);

    let fresh = self::persister(&[]);
    fresh.set_vibevm_home(Path::new(r"D:\v")).unwrap();
    assert_eq!(
        fresh.registry().value("VIBEVM_HOME").map(|raw| raw.kind),
        Some(RegistryValueKind::String)
    );
}

/// `$VIBEVM_INSTALL_ROOT` is the one durable value that decides where the
/// whole installation lives (`PROP-019#install-root`), so it follows the
/// same two rules as every other user variable: the kind an existing value
/// carries is preserved — an operator's `REG_EXPAND_SZ` `%SystemDrive%\Work`
/// must keep expanding — and an identical value is never rewritten.
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-019#install-root",
    r = 3
)]
fn the_install_root_keeps_its_kind_and_skips_identical_writes() {
    let persister = persister(&[(
        "VIBEVM_INSTALL_ROOT",
        r"%SystemDrive%\Old",
        RegistryValueKind::ExpandString,
    )]);
    assert_eq!(
        persister.set_install_root(Path::new(r"C:\Work")).unwrap(),
        Persisted::Changed
    );
    assert_eq!(
        persister.registry().value("vibevm_install_root"),
        Some(RawUserVar {
            value: r"C:\Work".into(),
            kind: RegistryValueKind::ExpandString,
        })
    );
    assert_eq!(
        persister.set_install_root(Path::new(r"C:\Work")).unwrap(),
        Persisted::Unchanged
    );
    assert_eq!(persister.registry().writes(), 1);

    // A value born here is plain: nothing in it asked to be expanded.
    let fresh = self::persister(&[]);
    fresh.set_install_root(Path::new(r"C:\Work")).unwrap();
    assert_eq!(
        fresh
            .registry()
            .value("VIBEVM_INSTALL_ROOT")
            .map(|raw| raw.kind),
        Some(RegistryValueKind::String)
    );
    // The install root and the instance pointer are different variables.
    assert!(fresh.registry().value("VIBEVM_HOME").is_none());
}

/// The closed network that runs executables only from `C:\Work`: the PATH
/// step puts `C:\Work\opt\bin` on the durable user PATH, and the user
/// applications' PATH proxies go to that same directory, so an installed
/// command is on the PATH the profile fixed (PROP-019 `#install-root`).
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-019#install-root")]
fn application_commands_land_in_the_directory_the_path_step_made_durable() {
    let persister = persister(&[("Path", r"C:\Tools", RegistryValueKind::ExpandString)]);
    let base = Path::new(r"C:\Work");
    let store = super::super::super::store::VersionStore::new(base.join("opt"));
    persister.set_install_root(base).unwrap();
    persister.ensure_on_path(&store.shim_dir()).unwrap();

    let settings = vibe_app::SettingsRoot::new(Path::new(r"C:\Users\me\.vibe"))
        .with_install_root(&base.join("opt"));
    let commands = settings.proxy_dir();
    assert_eq!(commands, store.shim_dir());
    assert_eq!(
        persister.inspect_path(&commands).path_entry,
        PathEntryStatus::Correct,
        "the applications' command directory is the durable PATH entry"
    );
    let raw = path_of(&persister).unwrap();
    assert!(
        raw.value.starts_with(&commands.display().to_string()),
        "{raw:?}"
    );
    // Without an install root the applications stay in the settings root,
    // which the PATH step did not touch.
    let plain = vibe_app::SettingsRoot::new(Path::new(r"C:\Users\me\.vibe"));
    assert_ne!(
        persister.inspect_path(&plain.proxy_dir()).path_entry,
        PathEntryStatus::Correct
    );
}
