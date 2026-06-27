//! The PROP-011 §2.6 in-workspace-`file://` mutable-source predicate — split
//! out of the freshness cell so each file stays within the length budget.

use std::path::Path;

specmark::scope!("spec://vibevm/modules/vibe-workspace/PROP-011#skip-resolution");

/// `true` iff `source_url` is a local `file://` path located *inside*
/// `workspace_root` — the in-repo self-hosting registry (`packages/`,
/// `--registry packages`), which the author edits in place. This is the
/// PROP-011 §2.6 mutable case. An *external* local registry or mirror (a
/// `file://` path outside the workspace — a test fixture, a published local
/// mirror) stays immutable and keeps the §2.2/§2.3 fast path; a `git+file://`
/// local git repo is a content-addressed git source and does not match the
/// `file://` prefix; a remote `https://` / `git@` source is not local at all.
///
/// The test is component-wise (separator-agnostic) and case-insensitive on
/// Windows. `workspace_root` is already canonicalised and `\\?\`-free
/// (`Workspace::load`); the source path is decoded from the URL *without*
/// canonicalisation, so a self-hosting `packages/` directly under the root is
/// detected reliably, while an exotic symlinked source that escapes detection
/// merely falls back to the immutable fast path (always safe).
///
/// ```
/// use std::path::Path;
/// use vibe_workspace::freshness::is_in_workspace_file_source as under;
/// let root = Path::new("/home/me/proj");
/// assert!(under("file:///home/me/proj/packages/x", root)); // self-hosting
/// assert!(!under("file:///tmp/fixture/x", root));          // external local
/// assert!(!under("https://example/x", root));              // remote
/// assert!(!under("git+file:///home/me/proj/x", root));     // git source
/// ```
pub fn is_in_workspace_file_source(source_url: &str, workspace_root: &Path) -> bool {
    let Some(rest) = source_url.strip_prefix("file://") else {
        return false;
    };
    // Decode `file://` to a filesystem path. `file:///C:/x` → `/C:/x` on
    // Windows: drop the leading slash before a `DRIVE:` so the path is `C:/x`.
    // `file:///home/x` → `/home/x` is already an absolute Unix path.
    let bytes = rest.as_bytes();
    let path_str = if bytes.len() >= 3
        && bytes[0] == b'/'
        && bytes[1].is_ascii_alphabetic()
        && bytes[2] == b':'
    {
        &rest[1..]
    } else {
        rest
    };
    path_under(Path::new(path_str), workspace_root)
}

/// Component-wise prefix test — `path` is at or below `base`. Separator-
/// agnostic (`Path::components`); case-insensitive on Windows, where the
/// filesystem is case-insensitive and a `file://` URL's drive-letter case
/// need not match the canonicalised root's.
fn path_under(path: &Path, base: &Path) -> bool {
    let fold = |p: &Path| -> Vec<String> {
        p.components()
            .map(|c| {
                let s = c.as_os_str().to_string_lossy();
                if cfg!(windows) {
                    s.to_lowercase()
                } else {
                    s.into_owned()
                }
            })
            .collect()
    };
    let base_c = fold(base);
    let path_c = fold(path);
    !base_c.is_empty() && base_c.len() <= path_c.len() && path_c[..base_c.len()] == base_c[..]
}
