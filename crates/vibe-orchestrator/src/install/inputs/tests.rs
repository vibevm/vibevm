//! Input-normalisation reds — split from `inputs.rs` to keep that cell
//! inside its line budget (guide#surface-form). The selected-world bundle's
//! own reds live beside it in `selection/tests.rs`.

#[cfg(test)]
mod spec_format_tests {
    use super::super::*;

    fn manifest(project_setting: Option<SpecFormat>) -> Manifest {
        let mut manifest: Manifest =
            toml::from_str("[project]\nname = \"demo\"\nversion = \"0.1.0\"\n")
                .expect("valid manifest");
        manifest.project.as_mut().expect("project").spec_format = project_setting;
        manifest
    }

    #[test]
    fn package_rooted_spec_format_is_equipotent() {
        // PROP-024 ##MANIFEST-ROLES-ARE-EQUIPOTENT: a package-rooted
        // checkout pins its materialisation exactly as a project does.
        let manifest: Manifest = toml::from_str(
            "[package]
name = \"b\"
group = \"org.x\"
kind = \"flow\"
version = \"1.0.0\"
spec_format = \"xml\"
",
        )
        .expect("valid manifest");
        assert_eq!(resolve_spec_format(&manifest, None), SpecFormat::Xml);
    }

    #[test]
    fn project_spec_format_wins_over_user_default() {
        assert_eq!(
            resolve_spec_format(&manifest(Some(SpecFormat::Xml)), Some(SpecFormat::Markdown)),
            SpecFormat::Xml
        );
    }

    #[test]
    fn user_default_and_builtin_mixed_fill_absent_project_setting() {
        assert_eq!(
            resolve_spec_format(&manifest(None), Some(SpecFormat::Markdown)),
            SpecFormat::Markdown
        );
        assert_eq!(
            resolve_spec_format(&manifest(None), None),
            SpecFormat::Mixed
        );
    }
}
