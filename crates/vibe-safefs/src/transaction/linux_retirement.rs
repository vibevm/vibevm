//! Explicit Linux cooperative checked directory move; old native APIs unchanged.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-056#SEC-NO-FOLLOW");
use super::*;

impl Pinned {
    /// Move one observed directory through held parents without replacing any
    /// destination entry. Linux only; no ordinary-rename fallback. Source
    /// identity is checked before/after, not atomically bound to renameat2
    /// against an arbitrary hostile same-user source replacement.
    pub fn retire_directory_noreplace_to(
        &self,
        destination: &Pinned,
        source_name: &str,
        destination_name: &str,
        expected: &EntryState,
    ) -> Result<(), RenameError> {
        #[cfg(not(target_os = "linux"))]
        {
            let _ = (destination, source_name, destination_name, expected);
            Err(RenameError::Unsupported)
        }
        #[cfg(target_os = "linux")]
        {
            use std::{ffi::CString, os::fd::AsRawFd};
            crate::ensure_safe_component(source_name).map_err(RenameError::Failed)?;
            crate::ensure_safe_component(destination_name).map_err(RenameError::Failed)?;
            if expected.kind != EntryStateKind::Directory {
                return Err(RenameError::Unsupported);
            }
            if !self
                .same_filesystem(destination)
                .map_err(RenameError::Failed)?
            {
                return Err(RenameError::CrossFilesystem);
            }
            let check = || -> Result<(), RenameError> {
                let actual = self
                    .inspect_child_state(source_name)
                    .map_err(RenameError::Failed)?;
                if actual.as_ref() != Some(expected) {
                    return Err(RenameError::SourceChanged {
                        path: self.join(source_name),
                        detail: "source directory differs from retained observation".into(),
                    });
                }
                Ok(())
            };
            check()?;
            rename_hook::before(self, destination, source_name, destination_name);
            check()?;
            final_rename_hook::after(self, destination, source_name, destination_name);
            let old =
                CString::new(source_name).map_err(|error| RenameError::Failed(error.into()))?;
            let new = CString::new(destination_name)
                .map_err(|error| RenameError::Failed(error.into()))?;
            #[allow(unsafe_code)]
            // SAFETY: both retained directory descriptors and NUL-terminated
            // single components remain live for this synchronous syscall.
            let status = unsafe {
                libc::renameat2(
                    self.dir.as_raw_fd(),
                    old.as_ptr(),
                    destination.dir.as_raw_fd(),
                    new.as_ptr(),
                    libc::RENAME_NOREPLACE,
                )
            };
            if status != 0 {
                let error = std::io::Error::last_os_error();
                return Err(match error.raw_os_error() {
                    Some(libc::EEXIST) | Some(libc::ENOTEMPTY) => RenameError::Occupied {
                        path: destination.join(destination_name),
                    },
                    Some(libc::EXDEV) => RenameError::CrossFilesystem,
                    Some(libc::ENOSYS) | Some(libc::EINVAL) | Some(libc::EOPNOTSUPP) => {
                        RenameError::Unsupported
                    }
                    _ => RenameError::Failed(error.into()),
                });
            }
            let observed = (|| -> anyhow::Result<bool> {
                Ok(
                    destination.inspect_child_state(destination_name)?.as_ref() == Some(expected)
                        && self.inspect_child_state(source_name)?.is_none(),
                )
            })();
            if !matches!(observed, Ok(true)) {
                return Err(RenameError::PossiblyMoved {source:self.join(source_name),destination:destination.join(destination_name),detail:"post-move identity or source absence could not be established; preserve both locations".into()});
            }
            Ok(())
        }
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    #[test]
    fn exact_directory_move_keeps_identity_and_bytes() {
        let temp = tempfile::tempdir().unwrap();
        let project = Project::open(temp.path()).unwrap();
        let parent = project.root_dir().unwrap();
        std::fs::create_dir(temp.path().join("source")).unwrap();
        std::fs::write(temp.path().join("source/payload"), b"owned").unwrap();
        let expected = parent.inspect_child_state("source").unwrap().unwrap();
        parent
            .retire_directory_noreplace_to(&parent, "source", "stash", &expected)
            .unwrap();
        assert_eq!(
            parent.inspect_child_state("stash").unwrap().as_ref(),
            Some(&expected)
        );
        assert_eq!(
            std::fs::read(temp.path().join("stash/payload")).unwrap(),
            b"owned"
        );
        // The historical API stays unsupported on Linux; this is opt-in only.
        assert!(matches!(
            parent.rename_child_noreplace_to(&parent, "stash", "old-api", &expected),
            Err(RenameError::Unsupported)
        ));
    }
    #[test]
    fn raced_destination_is_never_replaced_and_source_remains() {
        let temp = tempfile::tempdir().unwrap();
        let project = Project::open(temp.path()).unwrap();
        let parent = project.root_dir().unwrap();
        std::fs::create_dir(temp.path().join("source")).unwrap();
        let expected = parent.inspect_child_state("source").unwrap().unwrap();
        let path = temp.path().join("stash");
        super::super::arm_before_rename_noreplace(Some(Box::new(move |_, _, _, _| {
            std::fs::write(&path, b"foreign").unwrap()
        })));
        assert!(matches!(
            parent.retire_directory_noreplace_to(&parent, "source", "stash", &expected),
            Err(RenameError::Occupied { .. })
        ));
        assert_eq!(
            std::fs::read(temp.path().join("stash")).unwrap(),
            b"foreign"
        );
        assert!(temp.path().join("source").is_dir());
    }
    #[test]
    fn last_check_source_race_reports_uncertainty_and_preserves_both_objects() {
        let temp = tempfile::tempdir().unwrap();
        let project = Project::open(temp.path()).unwrap();
        let parent = project.root_dir().unwrap();
        std::fs::create_dir(temp.path().join("source")).unwrap();
        std::fs::write(temp.path().join("source/payload"), b"owned").unwrap();
        let expected = parent.inspect_child_state("source").unwrap().unwrap();
        let root = temp.path().to_owned();
        super::super::arm_after_rename_source_check(Some(Box::new(move |_, _, _, _| {
            std::fs::rename(root.join("source"), root.join("retained-owned")).unwrap();
            std::fs::create_dir(root.join("source")).unwrap();
            std::fs::write(root.join("source/payload"), b"foreign").unwrap();
        })));
        assert!(matches!(
            parent.retire_directory_noreplace_to(&parent, "source", "stash", &expected),
            Err(RenameError::PossiblyMoved { .. })
        ));
        assert_eq!(
            std::fs::read(temp.path().join("retained-owned/payload")).unwrap(),
            b"owned"
        );
        assert_eq!(
            std::fs::read(temp.path().join("stash/payload")).unwrap(),
            b"foreign"
        );
    }
    #[test]
    fn cross_filesystem_refuses_without_fallback() {
        let temp = tempfile::tempdir().unwrap();
        let project = Project::open(temp.path()).unwrap();
        let parent = project.root_dir().unwrap();
        std::fs::create_dir(temp.path().join("source")).unwrap();
        let expected = parent.inspect_child_state("source").unwrap().unwrap();
        super::super::arm_same_filesystem_check(Some(Box::new(|_| false)));
        let result = parent.retire_directory_noreplace_to(&parent, "source", "stash", &expected);
        super::super::arm_same_filesystem_check(None);
        assert!(matches!(result, Err(RenameError::CrossFilesystem)));
        assert!(temp.path().join("source").is_dir());
        assert!(!temp.path().join("stash").exists());
    }
    #[test]
    fn same_bytes_replaced_source_identity_refuses_before_move() {
        let temp = tempfile::tempdir().unwrap();
        let project = Project::open(temp.path()).unwrap();
        let parent = project.root_dir().unwrap();
        std::fs::create_dir(temp.path().join("source")).unwrap();
        let expected = parent.inspect_child_state("source").unwrap().unwrap();
        let root = temp.path().to_owned();
        super::super::arm_before_rename_noreplace(Some(Box::new(move |_, _, _, _| {
            std::fs::rename(root.join("source"), root.join("old")).unwrap();
            std::fs::create_dir(root.join("source")).unwrap();
        })));
        assert!(matches!(
            parent.retire_directory_noreplace_to(&parent, "source", "stash", &expected),
            Err(RenameError::SourceChanged { .. })
        ));
        assert!(!temp.path().join("stash").exists());
        assert!(temp.path().join("old").exists());
    }
}
