macro_rules! links_tests {
    () => {
#[cfg(unix)]
#[test]
fn strong_owned_directory_create_is_explicitly_unsupported() {
    let (_root, project) = project();
    let parent = project.root_dir().unwrap();
    assert!(matches!(
        parent.create_owned_child_exclusive("candidate", "owner"),
        Err(crate::OwnedDirectoryCreateError::Unsupported)
    ));
    assert!(!parent.path().join("candidate").exists());
}

#[cfg(unix)]
fn link_file(target: &Path, link: &Path) -> bool {
    std::os::unix::fs::symlink(target, link).is_ok()
}

#[cfg(unix)]
fn link_directory(target: &Path, link: &Path) -> bool {
    std::os::unix::fs::symlink(target, link).is_ok()
}

#[cfg(unix)]
fn remove_directory_link(link: &Path) {
    fs::remove_file(link).unwrap();
}

#[cfg(windows)]
fn link_directory(target: &Path, link: &Path) -> bool {
    let link = link.to_string_lossy().replace('/', "\\");
    std::process::Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(link)
        .arg(target)
        .output()
        .is_ok_and(|output| output.status.success())
}

#[cfg(windows)]
fn link_file(target: &Path, link: &Path) -> bool {
    std::os::windows::fs::symlink_file(target, link).is_ok()
}
    };
}
