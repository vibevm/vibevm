//! Author-side drop inputs of a `sources` step: `drop = "packages"` with a
//! `file` name and a digest, instead of a URL (`PROP-066#drops-author`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-066#drops-author");

use std::path::{Component, Path};

use super::SourceInputDecl;
use super::site::Site;
use crate::manifest::{FieldError, declarant_path};

/// The one artifact drop an input may name.
pub const DROP_PACKAGES: &str = "packages";

/// `file` belongs to a drop input only.
pub(super) fn validate_file_without_drop(
    source: &SourceInputDecl,
    row: &Site<'_>,
) -> Result<(), FieldError> {
    if source.file.is_some() && source.drop.is_none() {
        return Err(row.refuse(
            "file",
            format!(
                "{}.sources `{}` declares `file` without `drop = \"packages\"` \
                 (spec://org.vibevm.core/vibevm/common/PROP-066#drops-author)",
                row.prose, source.name
            ),
        ));
    }
    Ok(())
}

/// A drop input names the `packages` drop and one portable file name.
pub(super) fn validate_drop_input(
    source: &SourceInputDecl,
    row: &Site<'_>,
) -> Result<(), FieldError> {
    let table = row.prose;
    let drop = source.drop.as_deref().unwrap_or_default();
    if drop != DROP_PACKAGES {
        return Err(row.refuse(
            "drop",
            format!(
                "{table}.sources `{}` drop `{drop}` is unknown; the one drop is `packages` \
                 (spec://org.vibevm.core/vibevm/common/PROP-066#drops-author)",
                source.name
            ),
        ));
    }
    let file = Path::new(source.file.as_deref().unwrap_or_default());
    let single = declarant_path(file).is_ok()
        && file.components().count() == 1
        && matches!(file.components().next(), Some(Component::Normal(_)));
    if !single {
        return Err(row.refuse(
            "file",
            format!(
                "{table}.sources `{}` with `drop` needs `file`, one portable file name \
                 (spec://org.vibevm.core/vibevm/common/PROP-066#drops-author)",
                source.name
            ),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::manifest::Manifest;

    fn manifest(input: &str) -> Result<Manifest, String> {
        let body = format!(
            "[package]\nname = \"dropped\"\ngroup = \"org.example\"\nkind = \"app\"\nversion = \"1.0.0\"\n\n\
             [application]\nid = \"dropped\"\n\n\
             [[installer_options]]\nid = \"src\"\nname = \"Src\"\nplatform = \"linux\"\n\
             platform_name = \"Linux\"\narch = \"x86_64\"\n\
             [installer_options.installation]\ntype = \"source_build\"\n\
             [[installer_options.installation.sources]]\nname = \"payload\"\n{input}\n\
             sha256 = \"{}\"\n\
             [installer_options.installation.recipe]\nscript = \"build.sh\"\n",
            "ab".repeat(32)
        );
        Manifest::parse_str(&body).map_err(|error| error.to_string())
    }

    #[test]
    fn a_drop_input_names_the_packages_drop_and_one_file() {
        let parsed = manifest("drop = \"packages\"\nfile = \"tool-1.0.tgz\"").unwrap();
        let round = Manifest::parse_str(&toml::to_string(&parsed).unwrap()).unwrap();
        assert_eq!(round, parsed);
        for (input, expected) in [
            (
                "drop = \"downloads\"\nfile = \"x.tgz\"",
                "drop `downloads` is unknown",
            ),
            ("drop = \"packages\"", "needs `file`"),
            ("drop = \"packages\"\nfile = \"../x.tgz\"", "needs `file`"),
            ("drop = \"packages\"\nfile = \"a/x.tgz\"", "needs `file`"),
            (
                "path = \"x.tgz\"\nfile = \"x.tgz\"",
                "declares `file` without",
            ),
            (
                "url = \"https://example.com/x.tgz\"\ndrop = \"packages\"\nfile = \"x.tgz\"",
                "exactly one of url, path or drop",
            ),
        ] {
            let error = manifest(input).unwrap_err();
            assert!(error.contains(expected), "{input}: {error}");
        }
    }
}
