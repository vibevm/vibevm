//! Pure metadata rendering: exact publication input from captured decoration.

use super::{Lockfile, Manifest, render_toml};
use crate::error::Result;

impl Manifest {
    /// Render without reading a path, preserving only the supplied decoration.
    ///
    /// ```
    /// use vibe_core::manifest::Manifest;
    /// let old = "# operator note\n[project]\nname='demo'\nversion='1.0.0'\n";
    /// let manifest = Manifest::parse_str(old).unwrap();
    /// let rendered = manifest.render_with_comments(Some(old)).unwrap();
    /// assert!(rendered.contains("# operator note"));
    /// assert_eq!(Manifest::parse_str(&rendered).unwrap(), manifest);
    /// ```
    pub fn render_with_comments(&self, previous: Option<&str>) -> Result<String> {
        render_toml(self, previous)
    }
}

impl Lockfile {
    /// Prepare exact lock text without rereading a potentially changed path.
    /// Timestamp and semantic values come entirely from this value; only
    /// comments and decoration come from `previous`.
    ///
    /// ```
    /// let lock = vibe_core::manifest::Lockfile::empty("example", "fixed-time");
    /// let rendered = lock.render_with_comments(Some("# retained note\n")).unwrap();
    /// assert!(rendered.contains("# retained note"));
    /// assert!(rendered.contains("fixed-time"));
    /// ```
    pub fn render_with_comments(&self, previous: Option<&str>) -> Result<String> {
        render_toml(self, previous)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;

    #[test]
    fn captured_rendering_is_independent_of_a_later_disk_decoration() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("vibe.toml");
        let captured = "# captured\n[project]\nname='demo'\nversion='1.0.0'\n";
        let manifest = Manifest::parse_str(captured).unwrap();
        std::fs::write(&path, captured.replace("# captured", "# later actor")).unwrap();
        let rendered = manifest.render_with_comments(Some(captured)).unwrap();
        assert!(rendered.contains("# captured"));
        assert!(!rendered.contains("later actor"));
        assert_eq!(Manifest::parse_str(&rendered).unwrap(), manifest);
        manifest.write(&path).unwrap();
        assert!(
            std::fs::read_to_string(path)
                .unwrap()
                .contains("# later actor")
        );
    }

    #[test]
    fn ordinary_writer_serializes_before_reading_decoration() {
        struct DecoratesDuringSerialize<'a> {
            path: &'a std::path::Path,
            manifest: Manifest,
        }
        impl Serialize for DecoratesDuringSerialize<'_> {
            fn serialize<S: serde::Serializer>(
                &self,
                serializer: S,
            ) -> std::result::Result<S::Ok, S::Error> {
                std::fs::write(
                    self.path,
                    "# established during serialize\n[project]\nname='demo'\nversion='1.0.0'\n",
                )
                .unwrap();
                self.manifest.serialize(serializer)
            }
        }
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("vibe.toml");
        let value = DecoratesDuringSerialize {
            path: &path,
            manifest: Manifest::parse_str("[project]\nname='demo'\nversion='1.0.0'\n").unwrap(),
        };
        super::super::write_toml(&path, &value).unwrap();
        assert!(
            std::fs::read_to_string(path)
                .unwrap()
                .contains("# established during serialize")
        );
    }
}
