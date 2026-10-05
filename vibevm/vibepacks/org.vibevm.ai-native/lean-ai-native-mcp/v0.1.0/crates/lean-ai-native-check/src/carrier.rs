use crate::{Diagnostic, FileBinding, SpecBinding, TOOLCHAIN, files};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Carrier {
    pub schema: u32,
    pub namespace: String,
    pub toolchain: String,
    pub sources: Vec<String>,
    pub allowed_axioms: Vec<String>,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
    pub proof: Vec<Proof>,
}
fn default_timeout() -> u64 {
    30
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Proof {
    pub subject: String,
    pub spec_file: String,
    pub spec_revision: Option<u32>,
    pub module: String,
    pub theorem: String,
    pub grade: String,
}

pub(crate) struct Capture {
    pub carrier: Carrier,
    pub carrier_sha256: String,
    pub files: BTreeMap<String, Vec<u8>>,
    pub specs: BTreeMap<String, SpecBinding>,
}

impl Capture {
    pub fn inventory(&self) -> serde_json::Value {
        serde_json::json!({"schema":"lean-ai-native/theorem-inventory/1", "status":"declared", "carrier_sha256":self.carrier_sha256,
            "claims":self.carrier.proof.iter().map(|p| serde_json::json!({"subject":p.subject,"symbol":format!("lean::{}",p.theorem),"module":p.module,"source":module_file(&p.module),"relation":"verifies","grade":p.grade,"spec":self.specs[&p.subject]})).collect::<Vec<_>>()})
    }
    pub fn sources(&self) -> Vec<FileBinding> {
        self.carrier
            .sources
            .iter()
            .map(|path| files::binding(path, &self.files[path]))
            .collect()
    }
    pub fn selected(&self, subject: &str) -> Result<&Proof, Diagnostic> {
        self.carrier
            .proof
            .iter()
            .find(|p| p.subject == subject)
            .ok_or_else(|| {
                Diagnostic::new(
                    "unknown_subject",
                    "requested exact subject is not declared by this carrier",
                )
            })
    }
    pub fn recheck(&self, root: &Path) -> Result<(), Diagnostic> {
        for (path, bytes) in &self.files {
            if files::read_relative(root, Path::new(path))? != *bytes {
                return Err(Diagnostic::new(
                    "input_unstable",
                    format!("{path} changed during checking"),
                ));
            }
        }
        Ok(())
    }
}

pub(crate) fn module_file(module: &str) -> String {
    format!("{}.lean", module.replace('.', "/"))
}
fn name_valid(name: &str) -> bool {
    !name.is_empty()
        && name.split('.').all(|p| {
            !p.is_empty()
                && p.as_bytes()[0].is_ascii_alphabetic()
                && p.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'\'')
        })
}

#[specmark::spec(
    implements = "spec://org.vibevm.ai-native/lean-ai-native-lang/mechanisms/LEAN-THEOREM-CARRIER#CARRIER"
)]
pub(crate) fn capture(root: &Path, path: &Path) -> Result<Capture, Diagnostic> {
    let spelling = files::relative(path)?;
    let raw = files::read_relative(root, path)?;
    let text = std::str::from_utf8(&raw)
        .map_err(|e| Diagnostic::new("malformed_carrier", e.to_string()))?;
    let carrier: Carrier =
        toml::from_str(text).map_err(|e| Diagnostic::new("malformed_carrier", e.to_string()))?;
    if carrier.schema != 1 || carrier.toolchain != TOOLCHAIN {
        return Err(Diagnostic::new(
            "unsupported_contract",
            "requires carrier schema 1 and leanprover/lean4:v4.34.1",
        ));
    }
    let parts: Vec<_> = carrier.namespace.split('/').collect();
    let label = |p: &str| {
        !p.is_empty()
            && !p.starts_with('-')
            && !p.ends_with('-')
            && p.bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    };
    if parts.len() != 2
        || !parts[0].contains('.')
        || !parts[0].split('.').all(label)
        || !label(parts[1])
    {
        return Err(Diagnostic::new(
            "malformed_carrier",
            "namespace must be a fully qualified group/package coordinate",
        ));
    }
    if carrier.sources.is_empty()
        || carrier.sources.len() > 128
        || carrier.proof.is_empty()
        || carrier.proof.len() > 128
        || !(1..=120).contains(&carrier.timeout_seconds)
    {
        return Err(Diagnostic::new(
            "carrier_limit",
            "require 1..128 sources/proofs and timeout_seconds 1..120",
        ));
    }
    let mut seen = BTreeSet::new();
    for axiom in &carrier.allowed_axioms {
        if !["propext", "Quot.sound", "Classical.choice"].contains(&axiom.as_str())
            || !seen.insert(axiom)
        {
            return Err(Diagnostic::new(
                "axiom_policy",
                "allowed_axioms must be a unique subset of propext, Quot.sound, Classical.choice",
            ));
        }
    }
    let mut files = BTreeMap::new();
    files.insert(spelling, raw.clone());
    let mut sources = BTreeSet::new();
    for source in &carrier.sources {
        files::relative(Path::new(source))?;
        let module = source
            .strip_suffix(".lean")
            .ok_or_else(|| Diagnostic::new("malformed_carrier", "source must end in .lean"))?
            .replace('/', ".");
        let first = module.split('.').next().unwrap_or("");
        if !name_valid(&module)
            || module.starts_with("LeanAiNativeInspector")
            || ["Init", "Lean", "Std", "Lake"].contains(&first)
            || !sources.insert(source.clone())
        {
            return Err(Diagnostic::new(
                "malformed_carrier",
                "duplicate, reserved or unsupported module path",
            ));
        }
        files.insert(
            source.clone(),
            files::read_relative(root, Path::new(source))?,
        );
    }
    let mut specs = BTreeMap::new();
    for proof in &carrier.proof {
        if proof.grade != "g1_model" {
            return Err(Diagnostic::new(
                "unsupported_grade",
                "0.1.0 checks G1 model proofs only; no G3 implementation-refinement claim",
            ));
        }
        if !name_valid(&proof.module)
            || !name_valid(&proof.theorem)
            || !sources.contains(&module_file(&proof.module))
        {
            return Err(Diagnostic::new(
                "malformed_carrier",
                "theorem/module must be qualified Lean identifiers and module source must be declared",
            ));
        }
        if specs.contains_key(&proof.subject) {
            return Err(Diagnostic::new(
                "malformed_carrier",
                "each subject must select exactly one theorem per carrier",
            ));
        }
        // All subjects in one file use the same captured bytes. A later read
        // must never replace the bytes against which an earlier unit was bound.
        let spec_raw = if let Some(bytes) = files.get(&proof.spec_file) {
            bytes.clone()
        } else {
            let bytes = files::read_relative(root, Path::new(&proof.spec_file))?;
            files.insert(proof.spec_file.clone(), bytes.clone());
            bytes
        };
        let spec_text = std::str::from_utf8(&spec_raw)
            .map_err(|e| Diagnostic::new("invalid_spec", e.to_string()))?;
        let (units, warnings) = if proof.spec_file.ends_with(".xml") {
            specmap_core::xmlspec::parse_units(&proof.spec_file, spec_text, &carrier.namespace)
        } else if proof.spec_file.ends_with(".md") {
            specmap_core::mdspec::parse_units(&proof.spec_file, spec_text, &carrier.namespace)
        } else {
            return Err(Diagnostic::new(
                "invalid_spec",
                "spec_file must be a Markdown or XML spec source",
            ));
        };
        if !warnings.is_empty() {
            return Err(Diagnostic::new(
                "invalid_spec",
                warnings
                    .iter()
                    .map(|w| w.message.clone())
                    .collect::<Vec<_>>()
                    .join("; "),
            ));
        }
        let matches: Vec<_> = units.iter().filter(|u| u.uri == proof.subject).collect();
        if matches.len() != 1 {
            return Err(Diagnostic::new(
                "unknown_spec_subject",
                "exact subject must resolve once in its declared specification source",
            ));
        }
        let unit = matches[0];
        let revision = unit.revision.as_deref().copied();
        if revision != proof.spec_revision {
            return Err(Diagnostic::new(
                "spec_revision_mismatch",
                "carrier revision differs from authored spec revision (including absence)",
            ));
        }
        specs.insert(
            proof.subject.clone(),
            SpecBinding {
                file: proof.spec_file.clone(),
                file_sha256: files::hash(&spec_raw),
                unit_hash: unit.contentHash.clone(),
                revision,
            },
        );
    }
    Ok(Capture {
        carrier,
        carrier_sha256: files::hash(&raw),
        files,
        specs,
    })
}
