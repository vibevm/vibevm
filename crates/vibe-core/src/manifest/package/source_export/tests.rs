use super::*;
#[cfg(test)]
mod cases {
    use super::*;
    use crate::manifest::PackageMeta;
    fn package(extra: &str) -> PackageMeta {
        toml::from_str(&format!(
            "name='demo'\ngroup='org.demo'\nkind='flow'\nversion='1.0.0'\n{extra}"
        ))
        .unwrap()
    }
    #[test]
    fn source_export_presence_roundtrips_without_enabling_legacy() {
        let absent = package("");
        assert!(!absent.has_explicit_source_policy());
        assert!(!toml::to_string(&absent).unwrap().contains("source_exports"));
        let empty = package("source_exports=[]\n");
        assert!(empty.has_explicit_source_policy());
        assert!(empty.sources.is_none());
        let text = toml::to_string(&empty).unwrap();
        assert!(text.contains("source_exports = []"));
        assert_eq!(toml::from_str::<PackageMeta>(&text).unwrap(), empty);
        let mut json = serde_json::to_value(absent).unwrap();
        json["source_exports"] = serde_json::Value::Null;
        assert!(serde_json::from_value::<PackageMeta>(json).is_err());
    }
    #[test]
    fn source_export_names_paths_and_headings_are_strict() {
        let valid = SourceExport::new(
            "intro.v1".into(),
            "docs/guide.md".into(),
            vec!["Guide".into(), "First chapter".into()],
        )
        .unwrap();
        assert_eq!(valid.headings(), ["Guide", "First chapter"]);
        for name in ["", "1first", "bad/name", "équipe", "a b"] {
            assert!(
                SourceExport::new(name.into(), valid.file().into(), valid.headings().to_vec())
                    .is_err()
            );
        }
        for file in [
            "../escape",
            "/absolute",
            "docs/./guide",
            "docs\\guide",
            "vibevm/vibespecs/boot/STATIC.xml",
            "vibevm/vibespecs/boot/static.XML",
        ] {
            assert!(
                SourceExport::new("valid".into(), file.into(), valid.headings().to_vec()).is_err(),
                "{file}"
            );
        }
        for titles in [
            vec![],
            vec!["".into()],
            vec![" A".into()],
            vec!["A  B".into()],
            vec!["A\nB".into()],
        ] {
            assert!(SourceExport::new("valid".into(), valid.file().into(), titles).is_err());
        }
        let mut json = serde_json::to_value(&valid).unwrap();
        json["trust"] = serde_json::json!(true);
        assert!(serde_json::from_value::<SourceExport>(json).is_err());
    }
    #[test]
    fn source_export_duplicate_names_refuse_on_parse_and_serialization() {
        let row = "[[source_exports]]\nname='intro'\nfile='guide.md'\nheadings=['Guide']\n";
        let mut package = package(row);
        let source = toml::to_string(&package).unwrap();
        assert!(toml::from_str::<PackageMeta>(&format!("{source}\n{row}")).is_err());
        package.source_exports.as_mut().unwrap().push(
            SourceExport::new("intro".into(), "other.md".into(), vec!["Other".into()]).unwrap(),
        );
        assert!(toml::to_string(&package).is_err());
    }
}
