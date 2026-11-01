//! Every refusal of the application validators names its field as a TOML
//! path, one known refusal per validator file, plus the table a parse
//! error names.

use super::Manifest;
use crate::manifest::icon_findings;

const SHA: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

const PACKAGE: &str = r#"
[package]
name = "demo"
group = "org.example"
kind = "app"
version = "1.0.0"

[application]
id = "demo"
"#;

fn option(id: &str, arch: &str) -> String {
    format!(
        "\n[[installer_options]]\nid = \"{id}\"\nname = \"{id}\"\nplatform = \"linux\"\n\
         platform_name = \"Linux\"\narch = \"{arch}\"\n"
    )
}

fn archive_step(sha256: &str) -> String {
    format!(
        "\n[[installer_options.steps]]\nid = \"fetch\"\ntitle = \"Fetch\"\nkind = \"archive\"\n\
         format = \"zip\"\nsource = \"dist/demo.zip\"\nsha256 = \"{sha256}\"\n"
    )
}

/// The refusal's address and its sentence.
fn refusal(text: &str) -> (String, String) {
    let error = Manifest::parse_str(text).unwrap_err();
    let field = error
        .manifest_field()
        .unwrap_or_else(|| panic!("no field on: {error}"))
        .to_string();
    (field, error.to_string())
}

#[test]
fn an_option_refusal_names_the_option_field() {
    // validation.rs: the shorthand method's digest.
    let text = format!(
        "{PACKAGE}{}installation = {{ type = \"zip\", source = \"dist/a.zip\", sha256 = \"short\" }}\n",
        option("main", "x86_64")
    );
    let (field, message) = refusal(&text);
    assert_eq!(field, "installer_options[0].installation.sha256");
    assert!(message.contains("[installer_options.installation].sha256 must be a 64-digit"));

    let bad_id = PACKAGE.replace("id = \"demo\"", "id = \"Demo\"");
    assert_eq!(refusal(&bad_id).0, "application.id");
}

#[test]
fn a_step_refusal_names_the_authored_step() {
    // step_validation.rs: a bad sha256 on the second option's first step.
    let text = format!(
        "{PACKAGE}{}{}{}{}",
        option("first", "x86_64"),
        archive_step(SHA),
        option("second", "aarch64"),
        archive_step("nope"),
    );
    let (field, message) = refusal(&text);
    assert_eq!(field, "installer_options[1].steps[0].sha256");
    assert!(message.contains("[[installer_options.steps]] `fetch`.sha256 must be a 64-digit"));
    let (line, _) = Manifest::parse_str(&text)
        .unwrap_err()
        .manifest_field()
        .unwrap()
        .locate(&text)
        .unwrap();
    assert_eq!(text.lines().nth(line - 1), Some("sha256 = \"nope\""));
}

#[test]
fn a_hook_refusal_names_the_hook_not_the_expanded_step() {
    let text = format!(
        "{PACKAGE}{}pre_install = \" \"\n{}",
        option("main", "x86_64"),
        archive_step(SHA)
    );
    assert_eq!(refusal(&text).0, "installer_options[0].pre_install");
}

#[test]
fn a_requirement_refusal_names_its_element() {
    let text = format!(
        "{PACKAGE}{}{}requires = [\"bare-name\"]\n",
        option("main", "x86_64"),
        archive_step(SHA)
    );
    let (field, message) = refusal(&text);
    assert_eq!(field, "installer_options[0].steps[0].requires[0]");
    assert!(message.contains("must be fully qualified"), "{message}");
}

#[test]
fn a_parameter_with_an_empty_prompt_names_its_name() {
    // wizard_validation.rs: a parameter's `name` is the prompt the wizard
    // shows; empty, it is refused at that field.
    let text = format!(
        "{PACKAGE}{}\n[[installer_options.parameters]]\nid = \"port\"\nname = \" \"\ntype = \"integer\"\n{}",
        option("main", "x86_64"),
        archive_step(SHA)
    );
    let (field, message) = refusal(&text);
    assert_eq!(field, "installer_options[0].parameters[0].name");
    assert!(message.contains("[[installer_options.parameters]] `port` name must be nonempty"));
}

fn launcher(id: &str, args: &str) -> String {
    format!(
        "\n[[installer_options.launchers]]\nid = \"{id}\"\nname = \"Demo\"\n\
         executable = \"bin/demo\"\nkind = \"gui\"\nargs = {args}\n"
    )
}

#[test]
fn a_duplicate_launcher_id_names_the_second_row() {
    let text = format!(
        "{PACKAGE}{}{}{}{}",
        option("main", "x86_64"),
        archive_step(SHA),
        launcher("demo", "[]"),
        launcher("demo", "[]"),
    );
    let (field, message) = refusal(&text);
    assert_eq!(field, "installer_options[0].launchers[1].id");
    assert!(message.contains("has duplicate launcher id `demo`"));
}

#[test]
fn a_launcher_refusal_keeps_its_option_prefix_and_names_the_argument() {
    // launcher_validation.rs: the second argument has a placeholder that
    // does not exist.
    let text = format!(
        "{PACKAGE}{}{}{}",
        option("main", "x86_64"),
        archive_step(SHA),
        launcher("demo", "[\"--open\", \"{nope}\"]"),
    );
    let (field, message) = refusal(&text);
    assert_eq!(field, "installer_options[0].launchers[0].args[1]");
    assert!(message.contains("[[installer_options]] `main`: [[launchers]] `demo` argument"));
}

#[test]
fn an_icon_finding_names_the_slot() {
    let text = PACKAGE.replace(
        "id = \"demo\"\n",
        "id = \"demo\"\nicon = { png = { \"64\" = \"share/demo.png\" } }\n",
    );
    let manifest = Manifest::parse_str(&text).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let findings = icon_findings(dir.path(), manifest.application.as_ref(), &[]);
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert_eq!(findings[0].field.to_string(), "application.icon.png.64");
}

#[test]
fn a_parse_error_names_the_table_it_reports() {
    let text = "[project]\nname = \"x\"\n";
    let crate::Error::ParseToml { diagnostic, .. } = Manifest::parse_str(text).unwrap_err() else {
        panic!("a missing field is a parse error");
    };
    assert_eq!(
        diagnostic.field(text).map(|field| field.to_string()),
        Some("project.version".to_owned())
    );
    assert_eq!((diagnostic.line(), diagnostic.column()), (Some(1), Some(1)));

    // Through an array of tables the parser does not say which element,
    // so no address is given.
    let text = format!(
        "{PACKAGE}{}installation = {{ source = \"a.zip\" }}\n",
        option("main", "x86_64")
    );
    let crate::Error::ParseToml { diagnostic, .. } = Manifest::parse_str(&text).unwrap_err() else {
        panic!("a method without its type is a parse error");
    };
    assert_eq!(diagnostic.field(&text), None);
}
