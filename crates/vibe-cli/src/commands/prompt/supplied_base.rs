//! Finding a harness's prompt-base set on this machine, learning the
//! launch's model, and composing the carrier document (PROP-076
//! `#SYS-SUPPLIED-BASE`).
//!
//! **The interpreter lives in `vibe-workspace`; the machine lives here.**
//! `system_prompt::supplied_base` reads a `PROVENANCE.toml` and composes a
//! base from it, and holds no opinion about where that document is or which
//! model a launch will use. Those two questions are host questions — an
//! installed application, a settings tree, an environment, another
//! program's configuration precedence — so they are answered in the command
//! layer, where the installed-applications index and the package store are
//! reachable.
//!
//! **Every answer here is exact or an error.** A base from the wrong
//! version, or for the wrong model family, produces a session whose harness
//! instructions are not the ones that harness wrote, with no error
//! anywhere; that is strictly worse than delivering no lane, which
//! `#SYS-SOFT-LAUNCH` already judges. So there is no nearest-version
//! fallback, no search of the store for "something that looks close", and no
//! default model: each miss is named and the launch stays soft.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-SUPPLIED-BASE");

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::Result;
use vibe_app::{Index, SettingsRoot};
use vibe_workspace::system_prompt::{
    BaseError, CarrierDocument, HarnessKind, Launch, PROMPT_BASES_DIR, PROVENANCE_FILE, Provenance,
};

use super::flags::Undeliverable;

/// The environment variable naming a developer's own prompt-base tree.
///
/// **The second and last place the lookup looks.** `#SYS-SUPPLIED-BASE`
/// allows exactly one addition to the installed package's own source: "an
/// optional developer-supplied directory named by the environment for work
/// on an unreleased version". It names a directory shaped like a harness
/// package's root — the sets are at `<dir>/prompt-bases/<harness-version>/`
/// — so the same tree a packager is editing is the tree a launch reads, with
/// no second layout to keep in step.
pub(crate) const BASES_DIR_ENV: &str = "VIBE_PROMPT_BASES_DIR";

/// Where one harness version's set was looked for, in order.
///
/// Returned rather than logged, because the one thing a reader needs when a
/// set is missing is the list of places that were tried — and the lookup is
/// deliberately short enough to print.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Lookup {
    /// The directories tried, in order. The first that exists wins.
    pub(crate) candidates: Vec<Candidate>,
}

/// One place a set may live, and why the lookup knows about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Candidate {
    /// The set directory itself — `<root>/prompt-bases/<version>/`.
    pub(crate) dir: PathBuf,
    /// Where the root came from, as one clause a refusal prints.
    pub(crate) origin: String,
}

/// Build the lookup for one harness kind and version.
///
/// **Two places, in this order, and nothing else.** First the installed
/// harness application's own package source: the applications index names
/// the package coordinate, and the machine-global store keeps that
/// coordinate's tree at one path, which is the directory an installer step
/// already knows as `$package`. Then the developer directory of
/// [`BASES_DIR_ENV`]. A harness installed by some other means than vibe has
/// no first candidate at all, which is not an error — it is why the second
/// exists.
pub(crate) fn lookup(
    settings: &SettingsRoot,
    kind: HarnessKind,
    harness_version: &str,
    developer: Option<&Path>,
) -> Lookup {
    let mut candidates = Vec::new();
    if let Some((group, name)) = kind.base_package()
        && let Some(root) = installed_package_source(settings, group, name)
    {
        candidates.push(Candidate {
            dir: set_dir(&root, harness_version),
            origin: format!(
                "the installed `{group}/{name}` application's own package source, which is \
                 where a harness's bases ship"
            ),
        });
    }
    if let Some(declared) = developer {
        candidates.push(Candidate {
            dir: set_dir(declared, harness_version),
            origin: format!("`{BASES_DIR_ENV}`, the developer directory"),
        });
    }
    Lookup { candidates }
}

/// The developer tree [`BASES_DIR_ENV`] names, when it names one.
///
/// Read here rather than threaded from the composition root for the reason
/// `resolve`'s `PATH` read and `write_profile`'s `$CODEX_HOME` read are:
/// the fact being asked is where *this* launch should look, at the moment
/// it is about to look, and the lookup itself is a pure function over the
/// value.
#[specmark::spec(
    deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
    reason = "ambient-env: `VIBE_PROMPT_BASES_DIR` is the second and last \
              place PROP-076 `#SYS-SUPPLIED-BASE` allows a prompt-base set \
              to be looked for, and it exists for a packager working on an \
              unreleased harness version — whose whole point is that the \
              tree is named per invocation rather than per installation. \
              Threading it from the composition root would capture it \
              before the launcher and the harness version were known, which \
              is a different fact. `lookup` itself is PURE over the value; \
              this is the only ambient read in the cell, exactly as \
              `program::resolve`'s `PATH` read in the same command family"
)]
pub(crate) fn developer_dir() -> Option<PathBuf> {
    std::env::var_os(BASES_DIR_ENV)
        .map(PathBuf::from)
        .filter(|declared| !declared.as_os_str().is_empty())
}

/// `<root>/prompt-bases/<harness-version>/` — the one layout every reader
/// of a set shares.
fn set_dir(root: &Path, harness_version: &str) -> PathBuf {
    root.join(PROMPT_BASES_DIR).join(harness_version)
}

/// The package tree of an installed application, by coordinate.
///
/// The index records which version of which package an installation came
/// from; the machine store keeps exactly that identity at exactly one path.
/// Neither is searched: an installation that is not recorded, or a store
/// entry that is not there, answers `None` and the lookup moves on.
fn installed_package_source(settings: &SettingsRoot, group: &str, name: &str) -> Option<PathBuf> {
    let index = Index::load(settings).ok()?;
    let coordinate = index
        .installations
        .values()
        .map(|installed| &installed.package)
        .find(|package| package.group == group && package.name == name)?;
    let parsed = vibe_core::Group::parse(&coordinate.group).ok()?;
    let version = semver::Version::parse(&coordinate.version).ok()?;
    vibe_registry::store::lookup(&parsed, &coordinate.name, &version)
}

/// What a launch needs to know about its own model before a base can be
/// chosen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Model {
    /// The model id the recorded selection table is matched against — the
    /// bare model, without the provider and without a variant suffix,
    /// because the provenance states that the match is "against the model id
    /// lower-cased — not the provider id, not the model's family".
    pub(crate) id: String,
    /// The model's display name, for a substitution that asks for it.
    pub(crate) name: String,
    /// Where the answer came from, as one clause a note prints.
    pub(crate) origin: String,
}

/// Learn the model this launch would use.
///
/// **Two sources, and a refusal when neither answers.**
///
/// 1. **The launch's own arguments.** `--model <v>`, `--model=<v>`, `-m <v>`
///    and `-m=<v>` are OpenCode's own spellings; the **last** one wins,
///    which is the shell convention and the harness's own. An argument the
///    user wrote is the most specific thing anyone knows about this launch,
///    so nothing else is consulted when it is there.
/// 2. **The harness's own answer about its own configuration.** Not a
///    reimplementation of its precedence: `opencode --standalone debug
///    config` prints the configuration documents *it* resolved, in order,
///    and the model is read from the last document that carries one. A
///    launcher that reimplemented the search would be one upstream change
///    away from choosing a different model than the program does — and a
///    base chosen for the wrong model is exactly the silent failure this
///    whole mechanism exists to prevent. The probe is given the launch's
///    environment with vibe's own carrier **removed**, so what it reports is
///    the user's configuration rather than ours.
///
/// Neither answering is [`None`], and the caller soft-launches naming it.
/// There is deliberately no default: the default base is correct only for a
/// model that matches no selection row, and "no row matched" is a different
/// statement from "nobody knows which model this is".
pub(crate) fn discover_model(
    program: &Path,
    standing: &[String],
    project_root: &Path,
    carrier_env: &str,
    launch_args: &[String],
) -> Option<Model> {
    if let Some(stated) = model_from_arguments(launch_args) {
        return Some(Model {
            id: bare_model_id(&stated),
            name: bare_model_id(&stated),
            origin: format!("this launch's own `{stated}` argument"),
        });
    }
    // **In the project's own directory, not in this process's.** A harness
    // discovers project-level configuration from the working directory, so a
    // probe run anywhere else answers about a different project — which
    // `vibe prompt flags --path <elsewhere>` would otherwise do silently.
    let output = Command::new(program)
        .args(standing)
        .arg("--standalone")
        .arg("debug")
        .arg("config")
        .current_dir(project_root)
        .env_remove(carrier_env)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let documents = String::from_utf8_lossy(&output.stdout);
    let (selection, names) = configured_model(&documents)?;
    let id = bare_model_id(&selection);
    let name = names.get(&id).cloned().unwrap_or_else(|| id.clone());
    Some(Model {
        id,
        name,
        origin: "the effective OpenCode configuration this launch would use, as `opencode \
                 debug config` reports it"
            .to_owned(),
    })
}

/// The `--model` / `-m` value a launch's own arguments state, if any.
///
/// Pure, so the rule is testable without a harness: the scan is positional
/// and takes the last match, and an `=`-joined form is accepted because the
/// shells and the harness both accept it.
fn model_from_arguments(launch_args: &[String]) -> Option<String> {
    let mut found = None;
    let mut index = 0;
    while index < launch_args.len() {
        let argument = launch_args[index].as_str();
        if let Some(value) = argument
            .strip_prefix("--model=")
            .or_else(|| argument.strip_prefix("-m="))
        {
            if !value.is_empty() {
                found = Some(value.to_owned());
            }
        } else if (argument == "--model" || argument == "-m")
            && let Some(value) = launch_args.get(index + 1)
            && !value.is_empty()
        {
            found = Some(value.clone());
            index += 1;
        }
        index += 1;
    }
    found
}

/// The model the harness's own configuration documents select, plus every
/// model display name those documents carry.
///
/// The document list is the harness's resolution order, so the **last**
/// `model` is the effective one. Both spellings the configuration accepts
/// are read — the `provider/model` string and the `{providerID, model}`
/// object — because which one a user wrote is not vibe's business.
fn configured_model(documents: &str) -> Option<(String, BTreeMap<String, String>)> {
    let parsed: serde_json::Value = serde_json::from_str(documents).ok()?;
    let mut selection = None;
    let mut names = BTreeMap::new();
    for entry in parsed.as_array()? {
        let Some(info) = entry.get("info") else {
            continue;
        };
        match info.get("model") {
            Some(serde_json::Value::String(stated)) if !stated.trim().is_empty() => {
                selection = Some(stated.clone());
            }
            Some(serde_json::Value::Object(stated)) => {
                if let Some(model) = stated.get("model").and_then(serde_json::Value::as_str)
                    && !model.trim().is_empty()
                {
                    selection = Some(model.to_owned());
                }
            }
            _ => {}
        }
        // Either spelling of the provider table, because a configuration
        // written for one version of the schema is still on disk under the
        // next one.
        for key in ["provider", "providers"] {
            let Some(providers) = info.get(key).and_then(serde_json::Value::as_object) else {
                continue;
            };
            for provider in providers.values() {
                let Some(models) = provider
                    .get("models")
                    .and_then(serde_json::Value::as_object)
                else {
                    continue;
                };
                for (id, model) in models {
                    if let Some(name) = model.get("name").and_then(serde_json::Value::as_str) {
                        names.insert(id.to_ascii_lowercase(), name.to_owned());
                    }
                }
            }
        }
    }
    selection.map(|selection| (selection, names))
}

/// The bare model id out of whatever spelling named it: the part after the
/// last `/`, with a `#variant` suffix dropped.
///
/// Both halves are the provenance's statement rather than a guess — it says
/// the match is against the model id and not the provider id — and the
/// variant is a request setting rather than a different model, which is why
/// `gpt-6.1-sol#thinking` must still select the `gpt` base.
fn bare_model_id(stated: &str) -> String {
    stated
        .rsplit('/')
        .next()
        .unwrap_or(stated)
        .split('#')
        .next()
        .unwrap_or(stated)
        .trim()
        .to_owned()
}

/// Compose the carrier document for one launch, or say why it cannot be.
///
/// The order of the questions is the order of the answers a reader can act
/// on: first whether a set exists at all, then whether it is what it claims
/// and has been proved, then which model this launch is, and only then the
/// composition. Each one is a different thing the operator fixes.
pub(crate) fn carrier(request: &Request<'_>) -> Result<Composed, Undeliverable> {
    let Request {
        settings,
        kind,
        harness_version,
        program,
        standing,
        project_root,
        launch_args,
        lane,
    } = *request;
    let lookup = lookup(settings, kind, harness_version, developer_dir().as_deref());
    let Some(found) = lookup
        .candidates
        .iter()
        .find(|candidate| candidate.dir.join(PROVENANCE_FILE).is_file())
    else {
        return Err(Undeliverable::hard(
            format!(
                "this machine's {} is {harness_version} and no prompt-base set for that version \
                 is on it. {} REPLACES the harness's own prompt, so a launch that delivers the \
                 lane must supply {}'s own base prompt for exactly that version, and there is \
                 deliberately no fallback to another version's base",
                kind.as_str(),
                carrier_slot_name(kind),
                kind.as_str()
            ),
            looked_in(&lookup),
        ));
    };
    let provenance = Provenance::read(&found.dir).map_err(|error| {
        Undeliverable::hard(
            format!("the prompt-base set for {harness_version} is unusable: {error}"),
            format!(
                "repair or replace `{}`; a set vibe cannot read is never approximated",
                found.dir.display()
            ),
        )
    })?;
    if provenance.harness != kind.as_str() || provenance.harness_version != harness_version {
        return Err(Undeliverable::hard(
            format!(
                "`{}` describes {} {} and this launch is {} {harness_version} — a mis-filed set \
                 is never used on the chance that it fits",
                found.dir.join(PROVENANCE_FILE).display(),
                provenance.harness,
                provenance.harness_version,
                kind.as_str()
            ),
            format!(
                "put the set for {harness_version} in `{}`, or point `{BASES_DIR_ENV}` at a tree \
                 that has it",
                found.dir.display()
            ),
        ));
    }
    let model = discover_model(
        program,
        standing,
        project_root,
        &provenance_env(&provenance),
        launch_args,
    )
    .ok_or_else(|| {
        Undeliverable::hard(
            format!(
                "this launch's model could not be determined, and which base {} uses depends \
                     on it, so no base may be chosen — taking the generic one would hand a model \
                     family with a template of its own the wrong instructions, silently",
                kind.as_str()
            ),
            format!(
                "pass the model on the command line (`vibe agent {} run -m \
                     <provider>/<model> …`), or set one in OpenCode's own configuration so \
                     `opencode debug config` reports it",
                kind.as_str()
            ),
        )
    })?;
    let tools = provenance.gate_tools();
    let base = provenance
        .compose(
            &found.dir,
            &Launch {
                model_id: &model.id,
                model_name: &model.name,
                tools: &tools,
            },
        )
        .map_err(|error| undeliverable_base(kind, harness_version, &found.dir, &error))?;
    let document = provenance
        .carrier_document(&base, lane)
        .map_err(|error| undeliverable_base(kind, harness_version, &found.dir, &error))?;
    Ok(Composed {
        document,
        base_bytes: base.len(),
        model,
        set_dir: found.dir.clone(),
        set_origin: found.origin.clone(),
    })
}

/// Everything one composition needs: the machine, the harness it would
/// start, and the launch.
///
/// A struct rather than eight parameters because three of the eight are
/// paths and two are string slices — a positional call is one transposition
/// away from probing the wrong program in the wrong directory, and neither
/// mistake would show up as a type error.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Request<'a> {
    /// This machine's settings root, for the installed-package lookup.
    pub(crate) settings: &'a SettingsRoot,
    /// The harness kind whose base is being supplied.
    pub(crate) kind: HarnessKind,
    /// The version of it this launch would start.
    pub(crate) harness_version: &'a str,
    /// The resolved program, for the model probe.
    pub(crate) program: &'a Path,
    /// Its launcher's standing argument prefix, so a wrapper is probed
    /// through rather than around.
    pub(crate) standing: &'a [String],
    /// The project the launch would start in: a harness discovers
    /// project-level configuration from the working directory.
    pub(crate) project_root: &'a Path,
    /// The user's own launch arguments, which may name the model.
    pub(crate) launch_args: &'a [String],
    /// The absolute path of the composed lane.
    pub(crate) lane: &'a Path,
}

/// A composed carrier and the facts a note says it with.
#[derive(Debug, Clone)]
pub(crate) struct Composed {
    /// The document and the flags that make it reachable.
    pub(crate) document: CarrierDocument,
    /// How many bytes of harness base went in front of the lane.
    pub(crate) base_bytes: usize,
    /// The model the base was chosen for, and where that came from.
    pub(crate) model: Model,
    /// The set this came from.
    pub(crate) set_dir: PathBuf,
    /// Why the lookup knew about that directory.
    pub(crate) set_origin: String,
}

/// The one sentence a note says a composed carrier with.
pub(crate) fn note(composed: &Composed) -> String {
    format!(
        "the {}-byte base is OpenCode's own, composed from `{}` ({}) for model `{}` — {}; the \
         set's digests were verified and its `verified` flag is the whole-request proof having \
         been run for this exact version",
        composed.base_bytes,
        composed.set_dir.display(),
        composed.set_origin,
        composed.model.id,
        composed.model.origin
    )
}

/// The slot the base is supplied for, named rather than described, so a
/// refusal says which key of another program's configuration is in play.
fn carrier_slot_name(kind: HarnessKind) -> &'static str {
    match kind {
        HarnessKind::OpenCode => "an OpenCode agent's `system`",
        HarnessKind::Claude | HarnessKind::Codex | HarnessKind::Qwen => {
            "this harness's instruction slot"
        }
    }
}

/// The environment variable the set's own carrier travels in, so the model
/// probe can remove exactly that one from the child's environment.
fn provenance_env(provenance: &Provenance) -> String {
    provenance
        .carrier
        .as_ref()
        .map(|carrier| carrier.env.clone())
        .unwrap_or_default()
}

/// Turn one interpreter refusal into the sentence a soft launch prints.
///
/// The interpreter's own `Display` already names what went wrong; this adds
/// what the operator does about it, which differs per class: an unverified
/// set wants the sync tool, a digest mismatch wants the set replaced, and a
/// rule the interpreter cannot express wants a human to extend one of the
/// two deliberately.
fn undeliverable_base(
    kind: HarnessKind,
    harness_version: &str,
    dir: &Path,
    error: &BaseError,
) -> Undeliverable {
    let fix = match error {
        BaseError::NotVerified { .. } => format!(
            "run the sync tool for this version (`cargo run -p xtask -- opencode-bases \
             --version {harness_version}` in the vibevm repository), which earns `verified` by \
             the whole-request proof and is the only thing that may set it — never hand-edit it"
        ),
        BaseError::DigestMismatch { .. } | BaseError::MissingFile { .. } => format!(
            "replace `{}` with the published set for {harness_version}; data that is not what \
             its provenance claims never becomes a prompt",
            dir.display()
        ),
        BaseError::Unsupported { .. } => format!(
            "extend the provenance in OpenCode's package, or vibe's interpreter with its own \
             spec amendment — `{}` states a rule this build cannot express, and a special case \
             in Rust for one harness version is a literal wearing a different hat",
            dir.display()
        ),
        _ => format!(
            "declare `[system_prompt.harness.{}] delivery = \"read\"` to have the agent read \
             the boot files instead, or repair the set in `{}`",
            kind.as_str(),
            dir.display()
        ),
    };
    Undeliverable::hard(error.to_string(), fix)
}

/// The places the lookup tried, as one imperative clause.
fn looked_in(lookup: &Lookup) -> String {
    if lookup.candidates.is_empty() {
        return format!(
            "install the harness through vibe so its package ships the bases \
             (`vibe install -g org.vibevm.opencode/opencode`), or point `{BASES_DIR_ENV}` at a \
             checkout of that package"
        );
    }
    format!(
        "put the set at one of the two places the lookup knows — {} — and nowhere else: there is \
         no search",
        lookup
            .candidates
            .iter()
            .map(|candidate| format!("`{}` ({})", candidate.dir.display(), candidate.origin))
            .collect::<Vec<_>>()
            .join(", then ")
    )
}

#[cfg(test)]
#[path = "supplied_base/tests.rs"]
mod tests;
