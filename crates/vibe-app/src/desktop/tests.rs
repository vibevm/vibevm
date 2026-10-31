//! Desktop-entry reader proofs: groups, localised keys, escapes, lists,
//! booleans and `Exec` quoting with field codes.

use super::*;

const ENTRY: &str = r#"# A comment
[Desktop Entry]
Type=Application
Name=Text Editor
Name[de_DE.UTF-8]=Texteditor
Name[sr@latin]=Uređivač
Comment=Edit\stext\nfiles
Exec=edit --title "A \"B\" \\ C \$HOME" %U
Terminal=false
NoDisplay=true
Hidden=maybe
Keywords=text;notes\;draft;;
Actions=new-window;

[Desktop Action new-window]
Name=New Window
Exec=edit --new-window %u
"#;

#[test]
fn groups_localised_keys_escapes_lists_and_booleans() {
    let file = DesktopFile::parse(ENTRY).unwrap();
    let main = DesktopFile::MAIN;
    assert_eq!(file.groups.len(), 2);
    assert_eq!(
        file.string(main, "Comment").as_deref(),
        Some("Edit text\nfiles")
    );
    let names = file.localised(main, "Name");
    assert_eq!(names["de-DE"], "Texteditor");
    assert_eq!(names["sr@latin"], "Uređivač");
    assert_eq!(file.boolean(main, "NoDisplay"), Some(true));
    assert_eq!(file.boolean(main, "Terminal"), Some(false));
    assert_eq!(file.boolean(main, "Hidden"), None);
    assert_eq!(file.list(main, "Keywords"), ["text", "notes;draft"]);
    assert_eq!(
        file.string("Desktop Action new-window", "Name").as_deref(),
        Some("New Window")
    );
}

#[test]
fn exec_quoting_and_field_codes() {
    let file = DesktopFile::parse(ENTRY).unwrap();
    let exec = file.string(DesktopFile::MAIN, "Exec").unwrap();
    assert_eq!(
        exec_words(&exec).unwrap(),
        ["edit", "--title", "A \"B\" \\ C $HOME", "{urls}"]
    );
    assert_eq!(
        exec_words("a %f %F %u").unwrap(),
        ["a", "{file}", "{files}", "{url}"]
    );
    assert_eq!(exec_words("a %i %c %k %d").unwrap(), ["a"]);
    assert_eq!(exec_words("a --x=%F").unwrap(), ["a", "--x={file}"]);
    assert_eq!(exec_words("a \"\" b").unwrap(), ["a", "", "b"]);
    assert!(exec_words("   ").is_err());
}

#[test]
fn malformed_files_are_refused() {
    assert!(
        DesktopFile::parse("Name=x\n")
            .unwrap_err()
            .contains("before any group")
    );
    assert!(
        DesktopFile::parse("[A]\nnot a key\n")
            .unwrap_err()
            .contains("Key=Value")
    );
    assert!(
        DesktopFile::parse("[A]\nK=1\nK=2\n")
            .unwrap_err()
            .contains("repeated")
    );
    assert!(
        DesktopFile::parse("[A]\n[A]\n")
            .unwrap_err()
            .contains("repeated")
    );
}
