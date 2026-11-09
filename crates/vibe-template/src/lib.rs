//! The pure template-manifest renderer of PROP-075.
//!
//! A **template package** is one `vibe.toml` written with variables plus a
//! `[template]` table saying where its versions and digests come from. This
//! crate turns that text, one version record and one digest table into the
//! TOML text of an ordinary manifest — and nothing else. It opens no file,
//! makes no request, knows no registry and holds no cache: fetching,
//! caching, offline policy and store materialisation belong to
//! `vibe-registry` (PROP-075 `#TEMPLATE-RENDER-LOCATION`,
//! `#TEMPLATE-DOMAIN-SEAM`), and `tests/purity.rs` holds that line.
//!
//! Rendering is **two-stage** (PROP-075 `#TEMPLATE-TWO-STAGE`): a
//! [`skeleton`] uses the version record alone and is enough for everything
//! resolution needs, while [`render`] substitutes the digests and is
//! performed only for a version that is actually selected, locked,
//! installed, described or mirrored.
//!
//! ```
//! use vibe_template::{DigestTable, VersionRecord, render};
//!
//! let template = concat!(
//!     "[package]\nname = \"tool\"\ngroup = \"org.example\"\nkind = \"app\"\n",
//!     "version = \"1.0.0\"\ntemplate = true\n",
//!     "description = \"tool {version}\"\n\n",
//!     "[application]\nid = \"tool\"\n\n",
//!     "[template.versions]\nkind = \"list\"\n\n",
//!     "[[template.versions.item]]\nversion = \"3.4.5\"\n\n",
//!     "[template.digests]\nkind = \"sha256sums\"\n",
//!     "url = \"https://example.invalid/{version}/SHASUMS256.txt\"\n\n",
//!     "[[installer_options]]\nid = \"linux-x64\"\nname = \"tool {version}\"\n",
//!     "platform = \"linux\"\nplatform_name = \"Linux\"\narch = \"x86_64\"\n",
//!     "path_dirs = [\"bin\"]\n\n",
//!     "[[installer_options.steps]]\nid = \"unpack\"\ntitle = \"Unpack {version}\"\n",
//!     "kind = \"archive\"\nformat = \"tar_gz\"\n",
//!     "source = \"https://example.invalid/tool-{version}-{platform}-{arch}.tar.gz\"\n",
//!     "sha256 = \"{sha256:tool-{version}-{platform}-{arch}.tar.gz}\"\n",
//! );
//! let record = VersionRecord::plain("3.4.5")?;
//! let digests = DigestTable::parse_sha256sums(&format!(
//!     "{}  tool-3.4.5-linux-x86_64.tar.gz\n",
//!     "a".repeat(64)
//! ))?;
//! let rendered = render(template, &record, &digests)?;
//! assert!(rendered.toml.contains("version = \"3.4.5\""));
//! assert!(rendered.toml.contains(&format!("sha256 = \"{}\"", "a".repeat(64))));
//! // Every trace of the template vocabulary is gone from the output.
//! assert!(!rendered.toml.contains("template"));
//! # Ok::<(), vibe_template::RenderError>(())
//! ```

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-RENDER-LOCATION");

mod error;
mod render;
mod source;

pub use error::{RenderError, RenderErrorKind};
pub use render::{DroppedOption, Rendered, SKELETON_DIGEST, Stage, render, skeleton};
pub use source::{DigestEntry, DigestTable, VersionRecord, template_section, version_records};
