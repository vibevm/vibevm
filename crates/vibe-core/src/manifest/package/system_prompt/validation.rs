//! The value-grain refusals of `[system_prompt]`, each naming its own key
//! and every spelling it would have accepted (PROP-076 `#SYS-MANIFEST-TABLE`).
//!
//! These fire during deserialisation on purpose. A `lane_form` this version
//! cannot honour must refuse rather than default to `verbatim` while the
//! manifest claims otherwise, exactly as a bad `when` refuses instead of
//! defaulting (`super::super::WhenCondition`).

use super::{
    ALL_TARGET_OS, EntrypointChoice, EntrypointFile, EntrypointWire, HostChoice, LaneForm,
    PORTABLE, TargetOs, TargetSelection,
};

/// The three legal entrypoint spellings, as a refusal lists them.
fn legal_entrypoints() -> String {
    EntrypointFile::ALL
        .iter()
        .map(|file| format!("`{file}`"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The three legal OS spellings, as a refusal lists them.
fn legal_systems() -> String {
    ALL_TARGET_OS
        .iter()
        .map(|os| format!("`{os}`"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// One OS name from the closed set, or a refusal naming `field`.
fn parse_os(field: &str, name: &str) -> Result<TargetOs, String> {
    ALL_TARGET_OS
        .iter()
        .find(|os| os.as_str() == name)
        .copied()
        .ok_or_else(|| {
            format!(
                "[system_prompt] {field}: `{name}` is not an operating system vibe recognises — \
                 expected {} (exactly, lowercase)",
                legal_systems()
            )
        })
}

impl TryFrom<String> for HostChoice {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value == PORTABLE {
            return Ok(HostChoice::Portable);
        }
        // A fixed host is an OS name, so the OS refusal is the right one —
        // but it must also offer `portable`, which is not an OS and is the
        // answer a reader usually wants.
        parse_os("host", &value)
            .map(HostChoice::Fixed)
            .map_err(|error| {
                format!(
                    "{error}, or `{PORTABLE}` to leave every `os:` gate in the per-session \
                     continuation band instead of baking it into the prefix"
                )
            })
    }
}

impl From<HostChoice> for String {
    fn from(choice: HostChoice) -> Self {
        choice.as_str().to_string()
    }
}

impl TryFrom<Vec<String>> for TargetSelection {
    type Error = String;

    fn try_from(value: Vec<String>) -> Result<Self, Self::Error> {
        // An empty list is refused rather than read as "every OS": the two
        // mean opposite things, and a list that selects nothing would drop
        // every `target:` row silently. Omitting the key says "every OS".
        if value.is_empty() {
            return Err(format!(
                "[system_prompt] target = [] selects no operating system at all, which would \
                 drop every `target:` boot row — omit the key to mean every OS, or name the \
                 ones this product is for ({})",
                legal_systems()
            ));
        }
        let mut selected = Vec::with_capacity(value.len());
        for name in &value {
            let os = parse_os("target", name)?;
            if selected.contains(&os) {
                return Err(format!(
                    "[system_prompt] target lists `{os}` more than once — each operating system \
                     is named at most once"
                ));
            }
            selected.push(os);
        }
        Ok(TargetSelection::Only(selected))
    }
}

impl From<TargetSelection> for Vec<String> {
    fn from(selection: TargetSelection) -> Self {
        selection
            .platforms()
            .into_iter()
            .map(|os| os.as_str().to_string())
            .collect()
    }
}

impl TryFrom<EntrypointWire> for EntrypointChoice {
    type Error = String;

    fn try_from(wire: EntrypointWire) -> Result<Self, Self::Error> {
        match wire {
            EntrypointWire::Disabled(false) => Ok(EntrypointChoice::None),
            EntrypointWire::Disabled(true) => Err(
                "[system_prompt] entrypoint = true says nothing — write one of \
                 the legal spellings to choose a file, or `false` for no \
                 entrypoint segment"
                    .to_string(),
            ),
            EntrypointWire::Named(name) => EntrypointFile::ALL
                .iter()
                .find(|file| file.as_str() == name)
                .map(|file| EntrypointChoice::File(*file))
                .ok_or_else(|| {
                    format!(
                        "[system_prompt] entrypoint = \"{name}\" is not an agent instruction \
                         file — admitted spellings are {} (exactly, case included), or `false` \
                         for no entrypoint segment. The three are never concatenated, so only \
                         one may be named.",
                        legal_entrypoints()
                    )
                }),
        }
    }
}

impl From<EntrypointChoice> for EntrypointWire {
    fn from(choice: EntrypointChoice) -> Self {
        match choice.file() {
            Some(file) => EntrypointWire::Named(file.as_str().to_string()),
            None => EntrypointWire::Disabled(false),
        }
    }
}

impl TryFrom<String> for LaneForm {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "verbatim" => Ok(LaneForm::Verbatim),
            "projected" => Err(
                "[system_prompt] lane_form = \"projected\" is reserved and not supported in \
                 this version — a canonical-Markdown projection of the lane would change the \
                 token stream and could fail on an unrepresentable placement, so it is refused \
                 rather than silently treated as \"verbatim\" (fix: drop the key, or write \
                 lane_form = \"verbatim\")"
                    .to_string(),
            ),
            other => Err(format!(
                "[system_prompt] lane_form = \"{other}\" is unknown — the only accepted value \
                 in this version is \"verbatim\""
            )),
        }
    }
}

impl From<LaneForm> for String {
    fn from(form: LaneForm) -> Self {
        form.as_str().to_string()
    }
}
