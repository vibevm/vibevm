//! Switching a guard off must not also adopt a changed same-version default.
//!
//! A guarded exactly-one group with two valid members and one declared
//! default is installed while its gate is on. Turning only the gate off is an
//! ordinary transition. Turning the gate off while the live package source
//! now declares the other member as the default, at the same version, is a
//! different request: the retained selection no longer matches the published
//! definition, so the install has to refuse and leave the lock untouched.
use super::*;

const AUDIT: &str = "oracle-audit";

fn driver_manifest(f: &Fixture) -> PathBuf {
    f.registry.join(GROUP).join(DRIVER).join("v1.0.0/vibe.toml")
}

/// The consumer answers only the gate and leaves the guarded group to the
/// package's declared default.
fn request_gate(f: &Fixture, on: bool) {
    write(
        &f.project.join("vibe.toml"),
        format!(
            "[project]\nname='guarded-gate-drift'\nversion='0.1.0'\n[requires.packages.'{GROUP}/{DRIVER}']\nversion='=1.0.0'\nlink='static-transitive'\nconfiguration={{policy='use-declared-defaults',default_features=false,selections={{mode={{intent='exact',members=['{}']}}}}}}\n[[registry]]\nname='fixture'\nurl='file://{}'\n",
            if on { "on" } else { "off" },
            f.registry.display()
        ),
    );
}

/// Gate on, guarded group `detail` with members `extra` and `audit`, each
/// bringing its own optional dependency, and `extra` as the declared default.
fn fixture() -> Fixture {
    let f = Fixture::new(true);
    leaf(&f.registry, AUDIT, "ORACLE_AUDIT_LOADED");
    let manifest = package_header(DRIVER)
        + r#"
[requires.packages]
"org.vibevm/oracle-base"="^1"
"org.vibevm/oracle-extra"={version="^1",optional=true}
"org.vibevm/oracle-audit"={version="^1",optional=true}
[[configuration.features]]
id='on'
name='Gate on'
[[configuration.features]]
id='off'
name='Gate off'
[[configuration.features]]
id='extra'
name='Extra'
effects=[{kind='dependency',target='org.vibevm/oracle-extra'}]
[[configuration.features]]
id='audit'
name='Audit'
effects=[{kind='dependency',target='org.vibevm/oracle-audit'}]
[[configuration.groups]]
mode='choice'
id='mode'
name='Mode'
cardinality='exactly-one'
members=['on','off']
[[configuration.groups]]
mode='choice'
id='detail'
name='Guarded detail'
cardinality='exactly-one'
members=['extra','audit']
defaults=['extra']
guards=[{member='on'}]
"#;
    Manifest::parse_str(&manifest).expect("the original definition is valid");
    write(&driver_manifest(&f), manifest);
    request_gate(&f, true);
    f
}

/// Rewrite the live definition so the other member is the declared default.
fn declare_audit_default(f: &Fixture) {
    let path = driver_manifest(f);
    let original = fs::read_to_string(&path).unwrap();
    let changed = original.replace("defaults=['extra']", "defaults=['audit']");
    assert_ne!(original, changed, "the live source bytes must differ");
    Manifest::parse_str(&changed).expect("the changed definition stays valid");
    write(&path, changed);
}

fn locked_packages(f: &Fixture) -> BTreeSet<String> {
    f.lock()
        .packages
        .iter()
        .map(|p| p.name.as_str().to_owned())
        .collect()
}

fn names(list: &[&str]) -> BTreeSet<String> {
    list.iter().map(|name| (*name).to_owned()).collect()
}

#[test]
fn both_declared_defaults_are_valid_fresh_installations() {
    let original = fixture();
    original.install_ok();
    assert_eq!(locked_packages(&original), names(&[DRIVER, BASE, EXTRA]));

    let changed = fixture();
    declare_audit_default(&changed);
    changed.install_ok();
    assert_eq!(locked_packages(&changed), names(&[DRIVER, BASE, AUDIT]));
}

#[test]
fn gate_off_alone_is_accepted_from_the_retained_lock() {
    let f = fixture();
    f.install_ok();
    let active = fs::read(f.project.join("vibe.lock")).unwrap();

    request_gate(&f, false);
    f.install_ok();

    assert_ne!(fs::read(f.project.join("vibe.lock")).unwrap(), active);
    assert_eq!(locked_packages(&f), names(&[DRIVER, BASE]));
}

#[test]
fn gate_off_with_changed_same_version_default_refuses_and_keeps_the_lock() {
    let f = fixture();
    f.install_ok();
    let lock = fs::read(f.project.join("vibe.lock")).unwrap();

    request_gate(&f, false);
    declare_audit_default(&f);
    let before = super::workspace::semantic_snapshot(&f.project);
    let out = f.install();

    assert!(
        !out.status.success(),
        "gate off plus a changed same-version default was accepted: stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        fs::read(f.project.join("vibe.lock")).unwrap(),
        lock,
        "a refused transition must not rewrite the lock"
    );
    assert!(
        super::workspace::semantic_snapshot(&f.project) == before,
        "a refused transition must leave the project untouched"
    );
    assert_eq!(locked_packages(&f), names(&[DRIVER, BASE, EXTRA]));
}
