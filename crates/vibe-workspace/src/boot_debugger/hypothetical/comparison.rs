//! Complete-world comparisons, never addition of independently measured plans.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-072#MARGINAL-COMPARISON");
use super::*;
use measurement::{Count, Payload};
use std::collections::BTreeSet;

pub const RECIPE: &str = "frozen-world-marginal/1; candidate-minus-baseline; raw declaration membership; emitted artifact includes framing";

#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum SignedCount {
    Exact { value: i64 },
    Unavailable { reason: String },
}
impl SignedCount {
    fn subtract(before: &Count, after: &Count) -> Self {
        match (before, after) {
            (Count::Exact { value: b }, Count::Exact { value: a }) => {
                match i64::try_from(i128::from(*a) - i128::from(*b)) {
                    Ok(value) => Self::Exact { value },
                    Err(_) => Self::Unavailable { reason: "signed delta exceeds i64".into() },
                }
            }
            _ => Self::Unavailable { reason: "one or both complete-world measures unavailable; see world missing inputs and measure reasons".into() },
        }
    }
}
#[derive(Debug, Serialize)]
pub struct PackageEvidence {
    pub coordinate: String,
    pub manifest: Manifest,
    pub requires: Vec<(Group, String)>,
    pub source_identity: String,
    pub files: BTreeMap<String, Payload>,
}
#[derive(Debug, Serialize)]
pub struct Declaration {
    /// Ordinal is scoped to this exact world's placement inventory.
    pub contribution: usize,
    pub origin: String,
    pub path: String,
    pub loading: String,
    pub condition: Option<String>,
    pub raw: Option<Payload>,
}
#[derive(Debug, Serialize)]
pub struct PackageCost {
    pub coordinate: String,
    pub static_raw_direct: Count,
    pub static_raw_inclusive: Count,
    pub dynamic_potential_raw_direct: Count,
    pub dynamic_potential_raw_inclusive: Count,
    /// Reachable package membership is a set, even for diamond graphs.
    pub inclusive_members: Vec<String>,
    pub unavailable_members: Vec<String>,
}
#[derive(Debug, Serialize)]
pub struct WorldCost {
    pub world: String,
    pub catalog: String,
    pub capture: String,
    /// Retains the original owner evidence; this is not a second solver/wire.
    pub resolution_evidence: String,
    pub host_manifest: Manifest,
    pub host_files: BTreeMap<String, Payload>,
    pub packages: Vec<PackageEvidence>,
    pub declarations: Vec<Declaration>,
    pub package_costs: Vec<PackageCost>,
    /// Original compiler source/contribution/ordered occurrence carrier.
    pub compiler: Option<vibe_wire::generated::compiler_ir::e1::ir::Ir>,
    pub static_measurement: Option<measurement::Lane>,
    pub static_raw: Count,
    pub static_emitted: Count,
    pub dynamic_potential_raw: Count,
    pub runtime_reads: Count,
    pub missing: Vec<String>,
}
#[derive(Debug, Serialize)]
pub struct MeasureLink {
    pub world: String,
    pub catalog: String,
    pub capture: String,
    pub declarations: Vec<usize>,
    pub compiler_contributions: Vec<usize>,
}
#[derive(Debug, Serialize)]
pub struct Delta {
    pub lane: String,
    pub quantity: String,
    pub recipe: String,
    pub baseline: MeasureLink,
    pub candidate: MeasureLink,
    pub value: SignedCount,
}
#[derive(Debug, Serialize)]
pub struct Comparison {
    pub scenario: String,
    pub recipe: String,
    pub baseline: WorldCost,
    pub candidate: WorldCost,
    pub marginal: Vec<Delta>,
    pub aggregation: String,
}
impl Comparison {
    /// Experiments sharing a baseline are still non-additive. Callers must
    /// capture and evaluate the actual joint candidate through PreparedScenario.
    pub fn sum_independent(_experiments: &[Self]) -> Result<(), String> {
        Err("independent marginal experiments are non-additive; capture a complete joint candidate world".into())
    }
}
fn payloads(files: &BTreeMap<String, Vec<u8>>) -> Result<BTreeMap<String, Payload>, String> {
    files
        .iter()
        .map(|(path, bytes)| {
            measurement::measure_reader(bytes.as_slice())
                .map(|p| (path.clone(), p))
                .map_err(|e| e.to_string())
        })
        .collect()
}
fn sum_declarations(
    rows: &[Declaration],
    members: Option<&BTreeSet<String>>,
    dynamic: bool,
    complete: bool,
) -> Count {
    if !complete {
        return Count::unavailable("complete captured closure unavailable");
    }
    let mut value = 0u64;
    for row in rows.iter().filter(|r| {
        (r.loading == "dynamic") == dynamic && members.is_none_or(|m| m.contains(&r.origin))
    }) {
        let Some(payload) = &row.raw else {
            return Count::unavailable("declaration source capture unavailable");
        };
        let Some(next) = value.checked_add(payload.bytes) else {
            return Count::unavailable("u64 raw byte overflow");
        };
        value = next;
    }
    Count::exact(value)
}
fn world_cost(world: &FrozenWorld, preview: &Preview) -> Result<WorldCost, String> {
    let mut files = world.host.files.clone();
    let mut packages = Vec::new();
    for row in &world.packages {
        let p = row.manifest.require_package().map_err(|e| e.to_string())?;
        let slot = super::package_slot(p);
        for (path, bytes) in &row.files {
            files.insert(format!("{slot}/{path}"), bytes.clone());
        }
        packages.push(PackageEvidence {
            coordinate: format!("{}/{}", p.group, p.name),
            manifest: row.manifest.clone(),
            requires: row.requires.clone(),
            source_identity: row.source_identity.clone(),
            files: payloads(&row.files)?,
        });
    }
    let declarations = preview
        .placements
        .iter()
        .enumerate()
        .map(|(contribution, p)| {
            Ok(Declaration {
                contribution,
                origin: p.origin.clone(),
                path: p.path.clone(),
                loading: p.loading.clone(),
                condition: p.when.clone(),
                raw: files
                    .get(&p.path)
                    .map(|b| measurement::measure_reader(b.as_slice()).map_err(|e| e.to_string()))
                    .transpose()?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let complete = preview.status == "complete";
    let mut package_costs = Vec::new();
    for package in &packages {
        let direct = BTreeSet::from([package.coordinate.clone()]);
        let mut members = BTreeSet::new();
        let mut absent = BTreeSet::new();
        let mut pending = vec![package.coordinate.clone()];
        while let Some(id) = pending.pop() {
            if !members.insert(id.clone()) {
                continue;
            }
            match packages.iter().find(|p| p.coordinate == id) {
                None => {
                    absent.insert(id);
                }
                Some(p) => pending.extend(p.requires.iter().map(|(g, n)| format!("{g}/{n}"))),
            }
        }
        let available = complete && absent.is_empty();
        package_costs.push(PackageCost {
            coordinate: package.coordinate.clone(),
            static_raw_direct: sum_declarations(&declarations, Some(&direct), false, available),
            static_raw_inclusive: sum_declarations(&declarations, Some(&members), false, available),
            dynamic_potential_raw_direct: sum_declarations(
                &declarations,
                Some(&direct),
                true,
                available,
            ),
            dynamic_potential_raw_inclusive: sum_declarations(
                &declarations,
                Some(&members),
                true,
                available,
            ),
            inclusive_members: members.into_iter().collect(),
            unavailable_members: absent.into_iter().collect(),
        });
    }
    Ok(WorldCost {
        world: world.identity().into(),
        catalog: world.catalog().into(),
        capture: world.capture().into(),
        resolution_evidence: world.resolution_evidence.clone(),
        host_manifest: world.host.manifest.clone(),
        host_files: payloads(&world.host.files)?,
        packages,
        package_costs,
        static_raw: sum_declarations(&declarations, None, false, complete),
        dynamic_potential_raw: sum_declarations(&declarations, None, true, complete),
        static_emitted: if complete {
            preview
                .measurement
                .as_ref()
                .and_then(|m| m.artifact.as_ref())
                .map(|a| Count::exact(a.bytes))
                .unwrap_or_else(|| Count::unavailable("no captured compiled static artifact"))
        } else {
            Count::unavailable("complete compiled static closure unavailable")
        },
        declarations,
        compiler: preview.carrier.clone(),
        static_measurement: preview.measurement.clone(),
        runtime_reads: Count::unavailable(
            "unknown: dynamic declarations do not observe runtime reads",
        ),
        missing: preview.missing.clone(),
    })
}
fn link(world: &WorldCost, dynamic: bool) -> MeasureLink {
    MeasureLink {
        world: world.world.clone(),
        catalog: world.catalog.clone(),
        capture: world.capture.clone(),
        declarations: world
            .declarations
            .iter()
            .filter(|d| (d.loading == "dynamic") == dynamic)
            .map(|d| d.contribution)
            .collect(),
        compiler_contributions: if dynamic {
            vec![]
        } else {
            world
                .static_measurement
                .as_ref()
                .map(|m| m.snippets.iter().map(|s| s.contribution).collect())
                .unwrap_or_default()
        },
    }
}
pub(super) fn compare(
    scenario: &PreparedScenario,
    before: &Preview,
    after: &Preview,
) -> Result<Comparison, String> {
    let baseline = world_cost(&scenario.baseline, before)?;
    let candidate = world_cost(&scenario.candidate, after)?;
    let mut marginal = Vec::new();
    for (lane, quantity, b, a) in [
        (
            "static",
            "raw_declaration_bytes",
            &baseline.static_raw,
            &candidate.static_raw,
        ),
        (
            "static",
            "emitted_artifact_bytes",
            &baseline.static_emitted,
            &candidate.static_emitted,
        ),
        (
            "dynamic_potential",
            "raw_declaration_bytes",
            &baseline.dynamic_potential_raw,
            &candidate.dynamic_potential_raw,
        ),
    ] {
        marginal.push(Delta {
            lane: lane.into(),
            quantity: quantity.into(),
            recipe: RECIPE.into(),
            baseline: link(&baseline, lane == "dynamic_potential"),
            candidate: link(&candidate, lane == "dynamic_potential"),
            value: SignedCount::subtract(b, a),
        });
    }
    Ok(Comparison { scenario: scenario.identity().into(), recipe: format!("{RECIPE}; {}", measurement::RECIPE), baseline, candidate, marginal,
        aggregation: "non-additive: independent deltas must not be summed; evaluate a complete joint candidate".into() })
}
