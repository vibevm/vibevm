//! Actual site projections retain selected source semantics and publication checks.
use super::*;

const XML: &str = r#"<spec xmlns="https://vibevm.org/spec/1"><title id="root">Declared Spec</title><p><PROMISE fact="true" status="impl/done">The original promise.</PROMISE></p><p><OLD fact="true" status="spec/void">Retired rule.</OLD></p></spec>"#;

fn source(root: &Path, project: bool) -> std::io::Result<()> {
    package(root);
    let owner = if project { "project" } else { "package" };
    let kind = if project { "" } else { "kind='flow'\n" };
    fs::write(
        root.join("vibe.toml"),
        format!(
            "[{owner}]\ngroup='org.example'\nname='wal'\nversion='1.2.0'\n{kind}\n[[{owner}.sources]]\nfile='native.payload'\nmedia='application/xml'\ninterpretation='vibe-spec'\nrole='spec'\nloading='explicit'\n\n[[{owner}.sources]]\nfile='markdown.md'\nmedia='text/markdown'\ninterpretation='vibe-spec'\nrole='spec'\nloading='scan'\n"
        ),
    )?;
    fs::write(root.join("native.payload"), XML)?;
    fs::write(
        root.join("markdown.md"),
        "# Markdown source {#root}\n\nDeclared Markdown promise.\n",
    )?;
    fs::write(root.join("native.xml"), [0xff, 0x00])?;
    Ok(())
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-LEVEL0-READ",
    r = 1
)]
fn declared_package_and_host_render_all_formats_from_original_sources() {
    for project in [false, true] {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("source");
        source(&root, project).unwrap();
        let mut pair = pair();
        if project {
            pair.origin = Origin::HostProject { root: root.clone() };
        }
        let world = SpecSources::new();
        let work = tmp.path().join("work");
        let result = render(
            &pair,
            &Handy {
                source: root.clone(),
            },
            &options(&work, &world),
        )
        .unwrap();
        assert!(result.ok(), "{:?}", result.failed);
        for (format, tree) in FORMATS.iter().zip(&result.trees) {
            let native = fs::read_to_string(tree.join(build::page_path(
                "org.example/wal/1.2.0/",
                "native.payload",
                *format,
            )))
            .unwrap();
            assert!(native.contains("The original promise."), "{native}");
            assert!(native.contains("PROMISE"), "{native}");
            let markdown = fs::read_to_string(tree.join(build::page_path(
                "org.example/wal/1.2.0/",
                "markdown.md",
                *format,
            )))
            .unwrap();
            assert!(markdown.contains("Declared Markdown promise."));
            let manifest: serde_json::Value =
                serde_json::from_str(&fs::read_to_string(tree.join("manifest.json")).unwrap())
                    .unwrap();
            assert_eq!(manifest["package"]["projection"], true);
            if project {
                assert!(manifest["package"]["kind"].is_null());
            } else {
                assert_eq!(manifest["package"]["kind"], "flow");
            }
            assert!(
                !tree
                    .join(build::page_path(
                        "org.example/wal/1.2.0/",
                        "native.xml",
                        *format
                    ))
                    .exists()
            );
            let full = fs::read_to_string(tree.join("llms-full.txt")).unwrap();
            assert!(full.contains("The original promise."));
        }
        assert_eq!(fs::read(root.join("native.xml")).unwrap(), [0xff, 0x00]);
    }
}

struct Mutating {
    source: PathBuf,
}
impl Prepare for Mutating {
    fn warm(&self, _: &Pair) -> Result<PathBuf> {
        Ok(self.source.clone())
    }
    fn derived(&self, _: &Pair, _: &Path) -> BTreeMap<String, String> {
        assert!(
            fs::write(
                self.source.join("native.payload"),
                XML.replace("original", "changed"),
            )
            .is_ok(),
            "source fixture mutation must succeed"
        );
        BTreeMap::new()
    }
}

#[test]
fn mutation_after_composition_is_refused_before_source_publication() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("source");
    source(&root, false).unwrap();
    let world = SpecSources::new();
    let work = tmp.path().join("work");
    let result = render(&pair(), &Mutating { source: root }, &options(&work, &world)).unwrap();
    assert!(
        result
            .failed
            .as_deref()
            .is_some_and(|reason| reason.contains("changed")),
        "{:?}",
        result.failed
    );
    for (format, tree) in FORMATS.iter().zip(&result.trees) {
        assert!(
            tree.join(build::page_path(
                "org.example/wal/1.2.0/",
                FAILED_PAGE,
                *format
            ))
            .is_file()
        );
        assert!(
            !tree
                .join(build::page_path(
                    "org.example/wal/1.2.0/",
                    "native.payload",
                    *format
                ))
                .exists()
        );
    }
}

#[test]
fn completed_projection_keeps_guard_until_publication() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("source");
    source(&root, false).unwrap();
    let (composed, input) =
        level0::compose_prepared(&root, &tmp.path().join("composition"), &NOTHING).unwrap();
    let world = SpecSources::new();
    let build_options = build::Options {
        format: Format::Html,
        base: "/doc/".into(),
        manifest: manifest::Options::at(crate::manifest::tests::rendered_at()),
        derived: BTreeMap::new(),
    };
    let built = build::build_generated(&composed.dir, &input, &world, &build_options).unwrap();
    fs::write(root.join("native.payload"), "mutated after render").unwrap();
    assert!(input.recheck().is_err());
    assert!(
        built
            .files
            .iter()
            .any(|file| file.path.ends_with("native.payload/index.html"))
    );
}
