//! The `in drop` mark of a version listing: which rows an `[artifacts]
//! packages` drop already holds the payload of, for the platform the
//! listing is running on (PROP-066 `#drops`, PROP-075
//! `#VERSIONS-LIST-OUTPUT`).
//!
//! The question a user in a closed network actually has is not "what
//! versions exist" but "what can I install here and now", and the answer
//! is on their own disk. So the listing asks it — under two hard budgets,
//! because it is asked once per row and a domain is Node's 878 records.
//!
//! **No network and no digest document.** The file names come out of the
//! *skeleton* render of PROP-075 `#TEMPLATE-TWO-STAGE`, which the domain
//! read already produced and which costs no request: a digest variable
//! may appear **only** in the digest-bearing fields, so every URL — and
//! therefore every file name — is exact at the skeleton stage. A version
//! whose names a skeleton cannot settle is left unmarked, never guessed.
//!
//! **No byte of a payload is read.** The drop is searched by **name**
//! alone, through [`package_lookup_by_name`] — the install's own
//! convention minus the one rung keyed by the digest this path does not
//! have (`<drop>/<sha256>/<file>`). A 30–100 MB archive is never opened
//! and never hashed: integrity is the install's job, where the digest is
//! known and a wrong file is the `digest_mismatch` refusal
//! `#drops` already prescribes. The mark therefore says "a file of that
//! name is in place", which is exactly what a person filling a drop by
//! hand wants told, and never "these bytes will install".

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-066#drops");

use std::path::PathBuf;

use vibe_core::Group;
use vibe_core::manifest::{InstallerOptionDecl, Manifest, StepKind, steps_of};
use vibe_core::remap::drop::{package_lookup_by_name, shown_path, url_file_name};

use super::{Found, RegistryDomains};

impl RegistryDomains {
    /// Where every input this version's selected installer option
    /// downloads is already sitting in a configured drop, one path per
    /// input — or `None` for no mark at all.
    ///
    /// The row is marked only when **every** input is found, because a
    /// drop holding half an option's payload installs nothing: a partial
    /// answer would promise exactly what it cannot deliver. An option
    /// that downloads nothing — a system package manager, a `path` input
    /// shipped inside the package — is likewise unmarked: there is no
    /// file for a drop to hold, and `in drop` would be a sentence about a
    /// drop that plays no part.
    pub(super) fn drop_paths(
        &self,
        group: &str,
        name: &str,
        version: &semver::Version,
    ) -> Option<Vec<String>> {
        if self.drops.is_empty() {
            return None;
        }
        let files = self.downloaded_file_names(group, name, version)?;
        if files.is_empty() {
            return None;
        }
        // `<group>/<name>@<version>` is the package spelling the drop
        // convention's first two rungs are built from, the same one an
        // install passes (`PROP-066#drops`).
        let package = format!("{group}/{name}@{version}");
        let mut found = Vec::with_capacity(files.len());
        for file in &files {
            found.push(self.first_rung_holding(&package, file)?);
        }
        Some(found)
    }

    /// The first rung of the first drop that holds a regular file of this
    /// name — existence only, in the layer order `package_drops` gives,
    /// so any configured drop may serve the mark.
    fn first_rung_holding(&self, package: &str, file: &str) -> Option<String> {
        self.drops.iter().find_map(|drop| {
            package_lookup_by_name(drop, Some(package), Some(file))
                .into_iter()
                .find(|rung| rung.is_file())
                .map(|rung| shown_path(&rung))
        })
    }

    /// The names the inputs of the installer option an install would pick
    /// on this host are downloaded as — `None` when there is no such
    /// option, or when one of its inputs has no file name a skeleton can
    /// know.
    fn downloaded_file_names(
        &self,
        group: &str,
        name: &str,
        version: &semver::Version,
    ) -> Option<Vec<String>> {
        let manifest = self.manifest_of(group, name, version)?;
        // The selection an unattended install performs: this host's
        // detected platform and architecture, and no chooser — so a
        // target with several installers to pick between refuses, and the
        // row goes unmarked rather than being marked for an option the
        // user has not chosen yet.
        let option = vibe_app::select_option(
            &manifest.installer_options,
            &vibe_app::InstallChoice::default(),
            None,
        )
        .ok()?;
        downloaded_file_names(&option)
    }

    /// The manifest one version of the listed package is described by, at
    /// the stage a listing is allowed to pay for.
    ///
    /// The two origins split exactly as `#TEMPLATE-CONCRETE-WINS` says
    /// they must: a version the registry publishes concretely is read off
    /// disk and the template is not rendered for it at all, and a version
    /// only the template produces gets the **skeleton** render — call
    /// site 2 of `#TEMPLATE-DOMAIN-SEAM`, reusing the template and the
    /// version record the domain read already captured, so no row costs a
    /// second registry walk.
    fn manifest_of(&self, group: &str, name: &str, version: &semver::Version) -> Option<Manifest> {
        // The group has to be a group for the package to have been
        // listed, so a malformed one is simply no mark.
        Group::parse(group).ok()?;
        let found = self.found.borrow();
        let found: &Found = found.get(&format!("{group}/{name}"))?;
        if let Some(template) = &found.template
            && template.domain.needs_render(version)
        {
            let record = template.domain.record(version)?;
            let layer = vibe_registry::templates::active()?;
            return layer
                .skeleton_manifest(&template.template, version, record)
                .ok();
        }
        let published = found
            .roots
            .roots()
            .iter()
            .map(|root| published_manifest_path(root, group, name, version))
            .find(|path| path.is_file())?;
        Manifest::read(&published).ok()
    }
}

/// `<root>/<group>/<name>/v<version>/vibe.toml` — the layout every
/// directory-shaped registry has, and the one a local registry's own
/// manifest read uses.
fn published_manifest_path(
    root: &std::path::Path,
    group: &str,
    name: &str,
    version: &semver::Version,
) -> PathBuf {
    root.join(group)
        .join(name)
        .join(format!("v{version}"))
        .join(Manifest::FILENAME)
}

/// The file names one installer option's inputs are **downloaded** as, in
/// declaration order — the same three shapes `vibe artifacts where`
/// enumerates: a `[[sources]]` input with a URL, an author-side
/// `drop = "packages"` input that names its `file` directly
/// (`PROP-066#drops-author`), and an `archive` or `native` step whose
/// source is an HTTP(S) URL.
///
/// `None` when one of those declares no file name a drop could be
/// searched by — a URL ending in `/`, a `drop` input missing its `file` —
/// because a row must never be marked on a guess. An input that is not
/// downloaded at all (a package-relative `path`) is skipped: there is
/// nothing for a drop to hold.
fn downloaded_file_names(option: &InstallerOptionDecl) -> Option<Vec<String>> {
    let mut names = Vec::new();
    for step in steps_of(option) {
        match &step.kind {
            StepKind::Sources { sources } => {
                for source in sources {
                    match (&source.url, &source.drop) {
                        (Some(url), _) => names.push(url_file_name(url)?),
                        (None, Some(_)) => names.push(source.file.clone()?),
                        (None, None) => continue,
                    }
                }
            }
            StepKind::Archive { source, .. } | StepKind::Native { source, .. }
                if source.starts_with("https://") || source.starts_with("http://") =>
            {
                names.push(url_file_name(source)?);
            }
            _ => {}
        }
    }
    Some(names)
}

#[cfg(test)]
mod tests {
    use vibe_core::manifest::Manifest;

    use super::downloaded_file_names;

    fn option(body: &str) -> vibe_core::manifest::InstallerOptionDecl {
        let manifest = Manifest::parse_str(&format!(
            "[package]\nname = \"tool\"\ngroup = \"org.example\"\nkind = \"app\"\n\
             version = \"1.0.0\"\n\n[application]\nid = \"tool\"\n\n\
             [[installer_options]]\nid = \"only\"\nname = \"Only\"\nplatform = \"linux\"\n\
             platform_name = \"Linux\"\narch = \"x86_64\"\n{body}"
        ))
        .expect("the fixture manifest parses");
        manifest.installer_options.into_iter().next().unwrap()
    }

    /// An `archive` step's HTTP source is downloaded as the last segment
    /// of its URL — the name a drop is filled with.
    #[test]
    fn an_archive_step_is_named_by_the_last_segment_of_its_url() {
        let option = option(
            "path_dirs = [\"bin\"]\n\n[[installer_options.steps]]\nid = \"unpack\"\n\
             title = \"Unpack\"\nkind = \"archive\"\nformat = \"tar_gz\"\n\
             source = \"https://h.example/d/v1.0.0/tool-1.0.0-linux-x64.tar.gz\"\n\
             sha256 = \"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"\n",
        );
        assert_eq!(
            downloaded_file_names(&option).as_deref(),
            Some(["tool-1.0.0-linux-x64.tar.gz".to_owned()].as_slice())
        );
    }

    /// Every input of a `sources` step counts, a URL one by its last
    /// segment and an author-side `drop` one by the `file` it names —
    /// and a package-relative `path` input is not downloaded at all.
    #[test]
    fn every_downloaded_source_input_counts_and_a_path_input_does_not() {
        let option = option(
            "[installer_options.installation]\ntype = \"source_build\"\n\
             [[installer_options.installation.sources]]\nname = \"a\"\n\
             url = \"https://h.example/a-1.tgz\"\nsha256 = \"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"\n\
             [[installer_options.installation.sources]]\nname = \"b\"\n\
             drop = \"packages\"\nfile = \"b-1.tgz\"\nsha256 = \"cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc\"\n\
             [[installer_options.installation.sources]]\nname = \"c\"\n\
             path = \"vendor/c.tgz\"\nsha256 = \"eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee\"\n\
             [installer_options.installation.recipe]\nscript = \"build.sh\"\n\n\
             [[installer_options.launchers]]\nid = \"m\"\nname = \"M\"\n\
             executable = \"$app_bin/tool\"\nkind = \"command\"\ndefault = true\n",
        );
        assert_eq!(
            downloaded_file_names(&option).as_deref(),
            Some(["a-1.tgz".to_owned(), "b-1.tgz".to_owned()].as_slice())
        );
    }

    /// An option that downloads nothing names no file, and an input whose
    /// URL carries no last segment is no answer at all — never a guess.
    #[test]
    fn nothing_downloaded_is_empty_and_an_unnameable_input_is_unknown() {
        let shell = option(
            "[installer_options.installation]\ntype = \"shell\"\n\
             app_root = \"/usr/lib/tool\"\ninstall = \"apt-get install tool\"\n\
             force_install = \"apt-get install --reinstall tool\"\n\
             uninstall = \"apt-get remove tool\"\nforce_uninstall = \"apt-get purge tool\"\n",
        );
        assert_eq!(downloaded_file_names(&shell), Some(Vec::new()));

        let nameless = option(
            "path_dirs = [\"bin\"]\n\n[[installer_options.steps]]\nid = \"unpack\"\n\
             title = \"Unpack\"\nkind = \"archive\"\nformat = \"tar_gz\"\n\
             source = \"https://h.example/\"\nsha256 = \"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"\n",
        );
        assert_eq!(downloaded_file_names(&nameless), None);
    }
}
