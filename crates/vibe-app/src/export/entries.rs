//! The text of each system entry the projection writes
//! (`PROP-059#launcher-runtime`): a freedesktop `.desktop` file, the
//! `Info.plist` and shell shim of a macOS `.app` wrapper, and the Windows
//! PowerShell script that saves a `.lnk` through `WScript.Shell`. Pure
//! functions of the launcher row, so every platform's text is tested on
//! any host; only the writing in `export.rs` touches a disk.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#launcher-runtime");

use std::collections::BTreeMap;

/// What one entry says, gathered from a catalog row.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct EntryText {
    pub(crate) key: String,
    pub(crate) home: String,
    pub(crate) name: String,
    pub(crate) names: BTreeMap<String, String>,
    pub(crate) generic_name: Option<String>,
    pub(crate) comment: Option<String>,
    pub(crate) comments: BTreeMap<String, String>,
    /// The whole command line, program first.
    pub(crate) command: Vec<String>,
    /// A theme icon name or an absolute file.
    pub(crate) icon: Option<String>,
    pub(crate) terminal: bool,
    pub(crate) categories: Vec<String>,
    pub(crate) keywords: Vec<String>,
    pub(crate) mime_types: Vec<String>,
    pub(crate) window_class: Option<String>,
}

/// A desktop-entry string value: the escapes of the specification's
/// `string` type.
fn escaped(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('\t', "\\t")
        .replace('\r', "\\r")
}

/// A `;`-list value; a `;` inside an item is escaped.
fn list(items: &[String]) -> String {
    items
        .iter()
        .map(|item| format!("{};", escaped(item).replace(';', r"\;")))
        .collect()
}

/// One `Exec` argument: quoted when it holds a reserved character, with
/// `"`, `` ` ``, `$` and `\` escaped inside the quotes, and `%` doubled so
/// no field code is read into it.
fn exec_argument(argument: &str) -> String {
    let reserved = |ch: char| {
        ch.is_whitespace()
            || matches!(
                ch,
                '"' | '\''
                    | '\\'
                    | '>'
                    | '<'
                    | '~'
                    | '|'
                    | '&'
                    | ';'
                    | '$'
                    | '*'
                    | '?'
                    | '#'
                    | '('
                    | ')'
                    | '`'
            )
    };
    let argument = argument.replace('%', "%%");
    if argument.is_empty() || argument.contains(reserved) {
        let mut quoted = String::from('"');
        for ch in argument.chars() {
            if matches!(ch, '"' | '`' | '$' | '\\') {
                quoted.push('\\');
            }
            quoted.push(ch);
        }
        quoted.push('"');
        quoted
    } else {
        argument
    }
}

/// The `.desktop` file of one launcher. `X-Vibe-Key` is the marker that
/// makes the file vibe's, so a removal never deletes a file someone else
/// wrote under the same name.
pub(crate) fn desktop_entry(entry: &EntryText) -> String {
    let mut text = String::from("[Desktop Entry]\nType=Application\nVersion=1.5\n");
    let mut line = |key: &str, value: &str| text.push_str(&format!("{key}={value}\n"));
    line("Name", &escaped(&entry.name));
    for (language, name) in &entry.names {
        line(&format!("Name[{}]", locale(language)), &escaped(name));
    }
    if let Some(generic) = &entry.generic_name {
        line("GenericName", &escaped(generic));
    }
    if let Some(comment) = &entry.comment {
        line("Comment", &escaped(comment));
    }
    for (language, comment) in &entry.comments {
        line(&format!("Comment[{}]", locale(language)), &escaped(comment));
    }
    let exec: Vec<String> = entry.command.iter().map(|arg| exec_argument(arg)).collect();
    // The string escapes apply on top of the quoting (desktop entry
    // specification, "The Exec key").
    line("Exec", &escaped(&exec.join(" ")));
    if let Some(icon) = &entry.icon {
        line("Icon", &escaped(icon));
    }
    line("Terminal", if entry.terminal { "true" } else { "false" });
    for (key, items) in [
        ("Categories", &entry.categories),
        ("Keywords", &entry.keywords),
        ("MimeType", &entry.mime_types),
    ] {
        if !items.is_empty() {
            line(key, &list(items));
        }
    }
    if let Some(class) = &entry.window_class {
        line("StartupWMClass", &escaped(class));
    }
    line("X-Vibe-Key", &escaped(&entry.key));
    line("X-Vibe-Home", &escaped(&entry.home));
    text
}

/// A language tag as a desktop entry spells its locale (`de-DE` → `de_DE`).
fn locale(tag: &str) -> String {
    tag.replace('-', "_")
}

/// XML text with the five predefined entities.
fn xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// The `Info.plist` of a macOS wrapper. `VibeKey` is its marker, the
/// counterpart of `X-Vibe-Key`.
pub(crate) fn info_plist(
    entry: &EntryText,
    identifier: &str,
    executable: &str,
    icon: Option<&str>,
) -> String {
    let mut keys = vec![
        ("CFBundleDisplayName", entry.name.clone()),
        ("CFBundleExecutable", executable.to_owned()),
        ("CFBundleIdentifier", identifier.to_owned()),
        ("CFBundleName", entry.name.clone()),
        ("CFBundlePackageType", "APPL".to_owned()),
        ("CFBundleShortVersionString", "1.0".to_owned()),
        ("VibeHome", entry.home.clone()),
        ("VibeKey", entry.key.clone()),
    ];
    if let Some(icon) = icon {
        keys.push(("CFBundleIconFile", icon.to_owned()));
    }
    keys.sort();
    let body: String = keys
        .iter()
        .map(|(key, value)| format!("  <key>{key}</key>\n  <string>{}</string>\n", xml(value)))
        .collect();
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
         <plist version=\"1.0\">\n<dict>\n{body}</dict>\n</plist>\n"
    )
}

/// A POSIX shell word in single quotes.
fn sh_word(word: &str) -> String {
    format!("'{}'", word.replace('\'', "'\\''"))
}

/// The executable of a macOS wrapper: it replaces itself with the start
/// command, so the process the Dock watches is the one vibe started.
pub(crate) fn shell_shim(command: &[String]) -> String {
    let words: Vec<String> = command.iter().map(|word| sh_word(word)).collect();
    format!("#!/bin/sh\nexec {}\n", words.join(" "))
}

/// The executable of a wrapper whose launcher draws in a terminal: a
/// double-clicked bundle has none, so it asks Terminal to run the start
/// script beside it.
pub(crate) fn terminal_shim(script: &str) -> String {
    format!("#!/bin/sh\nexec /usr/bin/open -a Terminal \"$(dirname \"$0\")/{script}\"\n")
}

/// A PowerShell single-quoted string.
fn ps_string(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

/// One Windows command-line argument, quoted the way the C runtime splits
/// it back when it holds a space or a quote.
fn windows_argument(argument: &str) -> String {
    if !argument.is_empty() && !argument.contains([' ', '\t', '"']) {
        return argument.to_owned();
    }
    let mut quoted = String::from('"');
    let mut backslashes = 0;
    for ch in argument.chars() {
        match ch {
            '\\' => backslashes += 1,
            '"' => {
                quoted.push_str(&"\\".repeat(backslashes * 2 + 1));
                quoted.push('"');
                backslashes = 0;
            }
            other => {
                quoted.push_str(&"\\".repeat(backslashes));
                quoted.push(other);
                backslashes = 0;
            }
        }
    }
    quoted.push_str(&"\\".repeat(backslashes * 2));
    quoted.push('"');
    quoted
}

/// The PowerShell script that saves one `.lnk` through `WScript.Shell`, the
/// same COM object the shortcut dialogs of Windows use; the tree carries no
/// `.lnk` writer of its own.
pub(crate) fn shortcut_script(
    entry: &EntryText,
    lnk: &str,
    working_directory: Option<&str>,
    icon: Option<&str>,
) -> String {
    let (program, arguments) = entry
        .command
        .split_first()
        .map(|(program, rest)| (program.clone(), rest))
        .unwrap_or_default();
    let arguments: Vec<String> = arguments.iter().map(|arg| windows_argument(arg)).collect();
    let mut script = format!(
        "$ErrorActionPreference = 'Stop'\n\
         $link = (New-Object -ComObject WScript.Shell).CreateShortcut({})\n\
         $link.TargetPath = {}\n\
         $link.Arguments = {}\n\
         $link.Description = {}\n",
        ps_string(lnk),
        ps_string(&program),
        ps_string(&arguments.join(" ")),
        ps_string(&format!("{} (vibe {})", entry.name, entry.key)),
    );
    if let Some(directory) = working_directory {
        script.push_str(&format!(
            "$link.WorkingDirectory = {}\n",
            ps_string(directory)
        ));
    }
    if let Some(icon) = icon {
        script.push_str(&format!("$link.IconLocation = {}\n", ps_string(icon)));
    }
    script.push_str("$link.Save()\n");
    script
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::desktop::{DesktopFile, exec_words};

    fn entry() -> EntryText {
        EntryText {
            key: "org.example/editor@1.0.0#edit".into(),
            home: "/home/u/.vibe".into(),
            name: "My \"Editor\"".into(),
            names: BTreeMap::from([("de-DE".into(), "Bearbeiter".into())]),
            comment: Some("Edits;things".into()),
            command: vec![
                "env".into(),
                "VIBE_SETTINGS=/home/u/my vibe".into(),
                "/opt/vibe".into(),
                "app".into(),
                "start".into(),
                "org.example/editor#edit".into(),
                "100%".into(),
            ],
            icon: Some("vibe-org.example-editor-edit".into()),
            terminal: true,
            categories: vec!["Development".into(), "Utility".into()],
            ..EntryText::default()
        }
    }

    #[test]
    fn the_desktop_entry_reads_back_through_the_reader_with_the_same_command() {
        let text = desktop_entry(&entry());
        let file = DesktopFile::parse(&text).unwrap();
        let main = DesktopFile::MAIN;
        assert_eq!(file.string(main, "Name").as_deref(), Some("My \"Editor\""));
        assert_eq!(file.localised(main, "Name")["de-DE"], "Bearbeiter");
        assert_eq!(file.boolean(main, "Terminal"), Some(true));
        assert_eq!(file.list(main, "Categories"), ["Development", "Utility"]);
        assert_eq!(
            file.string(main, "X-Vibe-Key").as_deref(),
            Some("org.example/editor@1.0.0#edit")
        );
        let exec = file.string(main, "Exec").unwrap();
        assert_eq!(exec_words(&exec).unwrap(), entry().command);
    }

    #[test]
    fn the_wrapper_and_the_shortcut_quote_what_they_carry() {
        let plist = info_plist(&entry(), "org.vibevm.launcher.x", "vibe-launcher", None);
        assert!(plist.contains("<key>VibeKey</key>\n  <string>org.example/editor@1.0.0#edit"));
        assert!(plist.contains("My &quot;Editor&quot;"));
        let shim = shell_shim(&["/opt/vi'be".into(), "app".into()]);
        assert_eq!(shim, "#!/bin/sh\nexec '/opt/vi'\\''be' 'app'\n");
        assert!(terminal_shim("start.command").contains("open -a Terminal"));
        let script = shortcut_script(&entry(), r"C:\Users\u\Start\It's.lnk", None, Some("x.ico"));
        assert!(script.contains(r"CreateShortcut('C:\Users\u\Start\It''s.lnk')"));
        assert!(script.contains("$link.Arguments = '\"VIBE_SETTINGS=/home/u/my vibe\""));
        assert_eq!(windows_argument(r#"a "b"\"#), r#""a \"b\"\\""#);
    }
}
