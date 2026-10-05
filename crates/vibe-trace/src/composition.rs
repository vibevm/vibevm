//! One explicit consumer's Rust/Lean declaration map, not a proof verdict.
//! The opt-in table deliberately remains unknown to legacy family parsers.

use anyhow::{Context as _, Result, bail};
use serde::Deserialize;
use specmap_core::{
    config::Config,
    generated::specmap::{CodeItem, Edge, EdgeProvenance, EdgeVerb, Specmap, Warning},
    scanner::{CodeScanner, CompositeScanner, DefaultScanner},
};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Composition {
    pub schema: u32,
    pub kind: String,
    pub subject: String,
    pub rust_symbol: String,
    pub lean_carrier: String,
}

pub struct Loaded {
    pub config: Config,
    pub composition: Option<Composition>,
    original: String,
}

/// Deserialize the existing core config after removing only the explicit,
/// closed host extension. An unrecognized base field remains an error.
pub fn load(root: &Path) -> Result<Option<Loaded>> {
    let path = root.join(Config::REL_PATH);
    let original = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let mut value: toml::Value =
        toml::from_str(&original).context("invalid specmap configuration")?;
    let extra = value
        .as_table_mut()
        .context("specmap configuration must be a table")?
        .remove("composition");
    let composition: Option<Composition> = extra
        .map(toml::Value::try_into)
        .transpose()
        .context("invalid composition table")?;
    let config: Config = value
        .try_into()
        .context("invalid core specmap configuration")?;
    if config.namespace.trim().is_empty() {
        bail!("specmap namespace is missing");
    }
    if let Some(composition) = &composition {
        if composition.schema != 1 || composition.kind != "rust_lean" {
            bail!("unsupported-specmap-composition");
        }
        if !composition
            .subject
            .starts_with(&format!("spec://{}/", config.namespace))
        {
            bail!(
                "cross-package-proof-coverage-unsupported: subject differs from consumer namespace"
            );
        }
        if composition.rust_symbol.is_empty() || composition.lean_carrier.is_empty() {
            bail!("composition requires exact Rust symbol and Lean carrier");
        }
        if !config.missing_external_spec_roots(root).is_empty() {
            bail!("composition-external-source-unavailable");
        }
    }
    Ok(Some(Loaded {
        config,
        composition,
        original,
    }))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Inventory {
    schema: String,
    status: String,
    carrier_sha256: String,
    claims: Vec<Claim>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Claim {
    subject: String,
    symbol: String,
    module: String,
    source: String,
    relation: String,
    grade: String,
    spec: lean_ai_native_check::SpecBinding,
}

struct LeanScanner {
    carrier: String,
    inventory: Inventory,
}

impl CodeScanner for LeanScanner {
    fn id(&self) -> &'static str {
        "lean-theorem-carrier"
    }
    fn scan(&self, _: &Path, _: &Config) -> (Vec<CodeItem>, Vec<Edge>, Vec<Warning>) {
        let items = self
            .inventory
            .claims
            .iter()
            .map(|claim| CodeItem {
                crateName: "lean".into(),
                file: self.carrier.clone(),
                itemKind: "theorem-binding".into(),
                line: 1,
                symbol: claim.symbol.clone(),
                endLine: None,
                fingerprint: Some(Box::new(format!(
                    "carrier1:{}",
                    self.inventory.carrier_sha256
                ))),
            })
            .collect();
        let edges = self
            .inventory
            .claims
            .iter()
            .map(|claim| Edge {
                file: self.carrier.clone(),
                fromSymbol: claim.symbol.clone(),
                line: 1,
                provenance: EdgeProvenance::Authored,
                uri: claim.subject.clone(),
                verb: EdgeVerb::Verifies,
                pinnedR: claim.spec.revision.map(Box::new),
                reason: None,
            })
            .collect();
        (items, edges, Vec::new())
    }
}

/// Every declared scanner must succeed. A caller receives no partial map on
/// failure. Positions identify the authored carrier, not an invented Lean line.
pub fn build(root: &Path, loaded: &Loaded, extra: Option<&dyn CodeScanner>) -> Result<Specmap> {
    let default = DefaultScanner::new();
    let lean = if let Some(composition) = &loaded.composition {
        let reply = lean_ai_native_check::execute(
            &lean_ai_native_check::Context {
                project_root: root.to_path_buf(),
                toolchain_dir: None,
            },
            lean_ai_native_check::Request::Scan {
                carrier: PathBuf::from(&composition.lean_carrier),
            },
        );
        if !reply.ok {
            bail!("lean-carrier-scan-unavailable: {:?}", reply.error);
        }
        let inventory: Inventory =
            serde_json::from_value(reply.result.context("Lean inventory absent")?)
                .context("Lean inventory schema mismatch")?;
        if inventory.schema != "lean-ai-native/theorem-inventory/1"
            || inventory.status != "declared"
        {
            bail!("unsupported Lean inventory schema/status");
        }
        if !inventory
            .claims
            .iter()
            .any(|claim| claim.subject == composition.subject)
        {
            bail!("composition-subject-not-declared");
        }
        for claim in &inventory.claims {
            if !claim
                .subject
                .starts_with(&format!("spec://{}/", loaded.config.namespace))
            {
                bail!("cross-package-proof-coverage-unsupported");
            }
            if claim.relation != "verifies"
                || claim.grade != "g1_model"
                || claim.module.is_empty()
                || claim.source.is_empty()
            {
                bail!("invalid Lean declaration inventory");
            }
        }
        Some(LeanScanner {
            carrier: composition.lean_carrier.clone(),
            inventory,
        })
    } else {
        None
    };
    let mut scanners: Vec<&dyn CodeScanner> = vec![&default];
    if let Some(lean) = &lean {
        scanners.push(lean);
    }
    if let Some(extra) = extra {
        scanners.push(extra);
    }
    let map = specmap_core::index::build_with_scanner(
        root,
        &loaded.config,
        &CompositeScanner::new(scanners),
    );
    if let Some(composition) = &loaded.composition {
        if !map.warnings.is_empty() || !map.suspects.is_empty() {
            bail!("composition-incomplete: source diagnostics or stale edges");
        }
        if !map.edges.iter().any(|edge| {
            edge.uri == composition.subject
                && edge.fromSymbol == composition.rust_symbol
                && matches!(edge.verb, EdgeVerb::Implements)
        }) {
            bail!("composition-rust-implementation-missing");
        }
        for claim in &lean
            .as_ref()
            .context("Lean scanner missing")?
            .inventory
            .claims
        {
            let unit = map
                .specUnits
                .iter()
                .find(|unit| unit.uri == claim.subject)
                .context("composition spec subject missing")?;
            if unit.contentHash != claim.spec.unit_hash
                || unit.revision.as_deref().copied() != claim.spec.revision
            {
                bail!("composition-spec-identity-mismatch");
            }
        }
    }
    if fs::read_to_string(root.join(Config::REL_PATH))? != loaded.original {
        bail!("specmap-configuration-changed-during-scan");
    }
    Ok(map)
}

/// One atomic replacement after the entire map has been validated.
pub fn write(
    root: &Path,
    loaded: &Loaded,
    filename: &str,
    extra: Option<&dyn CodeScanner>,
) -> Result<Specmap> {
    let map = build(root, loaded, extra)?;
    write_map(root, filename, &map)?;
    Ok(map)
}

pub fn write_map(root: &Path, filename: &str, map: &Specmap) -> Result<()> {
    if !matches!(filename, "specmap.json" | "package.specmap.json") {
        bail!("unsupported map output name");
    }
    let bytes = specmap_core::index::to_canonical_bytes(map)?;
    let mut file = tempfile::NamedTempFile::new_in(root)?;
    file.write_all(bytes.as_bytes())?;
    file.as_file().sync_all()?;
    file.persist(root.join(filename))
        .map_err(|error| error.error)?;
    Ok(())
}

#[cfg(test)]
mod tests;
