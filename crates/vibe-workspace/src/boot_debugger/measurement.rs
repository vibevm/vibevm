//! Read-only measurements of captured graphs. No compiler or provider calls.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-072#CAPTURED-MEASUREMENT");
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    io::{self, Read},
    path::{Component, Path},
};
use vibe_spec::BootGraph;

pub const RECIPE: &str = "captured-utf8-sha256/v1; bytes verbatim; scalars Unicode; no tokenizer";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Count {
    Exact { value: u64 },
    Estimated { value: u64, recipe: String },
    Unavailable { reason: String },
}
impl Count {
    pub fn exact_value(&self) -> Option<u64> {
        match self {
            Self::Exact { value } => Some(*value),
            _ => None,
        }
    }
    pub fn exact(value: u64) -> Self {
        Self::Exact { value }
    }
    pub fn unavailable(reason: &str) -> Self {
        Self::Unavailable {
            reason: reason.into(),
        }
    }
    fn sum<'a>(values: impl Iterator<Item = &'a Count>) -> Self {
        let mut sum = 0u64;
        for value in values {
            match value {
                Self::Exact { value } => match sum.checked_add(*value) {
                    Some(n) => sum = n,
                    None => return Self::unavailable("u64 arithmetic overflow"),
                },
                _ => return Self::unavailable("one or more captured mappings unavailable"),
            }
        }
        Self::exact(sum)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Payload {
    pub sha256: String,
    pub bytes: u64,
    pub scalars: Count,
}

/// Bounded working memory, including UTF-8 split across arbitrary read chunks.
/// Invalid UTF-8 keeps exact bytes/hash and explicitly unavailable scalars.
pub fn measure_reader(mut reader: impl Read) -> io::Result<Payload> {
    let mut hash = Sha256::new();
    let mut bytes = 0u64;
    let mut scalars = 0u64;
    let mut valid = true;
    let mut pending = Vec::with_capacity(8196);
    let mut buffer = [0u8; 8192];
    loop {
        let n = match reader.read(&mut buffer) {
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            result => result?,
        };
        if n == 0 {
            break;
        }
        bytes = bytes
            .checked_add(n as u64)
            .ok_or_else(|| io::Error::other("byte counter overflow"))?;
        hash.update(&buffer[..n]);
        if valid {
            pending.extend_from_slice(&buffer[..n]);
            match std::str::from_utf8(&pending) {
                Ok(text) => {
                    scalars += text.chars().count() as u64;
                    pending.clear();
                }
                Err(e) => {
                    scalars += std::str::from_utf8(&pending[..e.valid_up_to()])
                        .expect("validated prefix")
                        .chars()
                        .count() as u64;
                    if e.error_len().is_some() {
                        valid = false;
                        pending.clear();
                    } else {
                        pending.drain(..e.valid_up_to());
                    }
                }
            }
        }
    }
    Ok(Payload {
        sha256: format!("{:x}", hash.finalize()),
        bytes,
        scalars: if valid && pending.is_empty() {
            Count::exact(scalars)
        } else {
            Count::unavailable("invalid UTF-8")
        },
    })
}
fn text_payload(text: &str) -> Payload {
    measure_reader(text.as_bytes()).expect("in-memory read")
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TokenMeasurement {
    pub tokenizer: Option<String>,
    pub version: Option<String>,
    pub recipe: String,
    pub count: Count,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Snippet {
    pub contribution: usize,
    pub source: Option<usize>,
    pub origin: String,
    pub path: String,
    pub kind: String,
    pub condition: Option<String>,
    pub package: Option<String>,
    pub raw: Option<Payload>,
    pub emitted: Option<Payload>,
    pub direct_bytes: Count,
    pub inclusive_bytes: Count,
    pub occurrences: Vec<Occurrence>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Occurrence {
    pub source: usize,
    pub requested: Option<String>,
    pub parent: Option<usize>,
    pub root: bool,
    pub emitted: Option<Payload>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Totals {
    pub direct_bytes: Count,
    pub inclusive_bytes: Count,
    pub unique_raw_bytes: Count,
    pub repeated_raw_bytes: Count,
    pub unique_emitted_bytes: Count,
    pub repeated_emitted_bytes: Count,
    pub recorded_occurrences: u64,
    pub runtime_reads: Count,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SourceMeasurement {
    pub index: usize,
    pub identity: String,
    pub raw: Option<Payload>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Lane {
    pub name: String,
    pub artifact: Option<Payload>,
    pub snippets: Vec<Snippet>,
    pub sources: Vec<SourceMeasurement>,
    pub totals: Totals,
    pub gaps: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Report {
    pub capture_id: String,
    pub recipe: String,
    pub basis: String,
    pub lanes: Vec<Lane>,
    pub packages: BTreeMap<String, Totals>,
    pub tokens: TokenMeasurement,
    pub ambient_tail: Count,
    pub provider_billing: Count,
    pub scope_data_observability: Count,
    pub inspected_source_reads: BTreeMap<String, Count>,
}

fn totals<'a>(rows: impl Iterator<Item = &'a Snippet>) -> Totals {
    let rows: Vec<_> = rows.collect();
    let mut unique = BTreeMap::new();
    let mut repeated = Vec::new();
    for row in &rows {
        if let Some(p) = &row.raw {
            unique.insert((&p.sha256, p.bytes), p.bytes);
            repeated.push(Count::exact(p.bytes));
        } else {
            repeated.push(Count::unavailable("raw source not captured"));
        }
    }
    let mut unique_emitted = BTreeMap::new();
    for row in &rows {
        if let Some(p) = &row.emitted {
            unique_emitted.insert((&p.sha256, p.bytes), p.bytes);
        }
    }
    Totals {
        unique_emitted_bytes: if rows.iter().all(|r| r.emitted.is_some()) {
            Count::sum(
                unique_emitted
                    .values()
                    .map(|n| Count::exact(*n))
                    .collect::<Vec<_>>()
                    .iter(),
            )
        } else {
            Count::unavailable("emitted ranges unavailable")
        },
        repeated_emitted_bytes: Count::sum(rows.iter().map(|r| &r.inclusive_bytes)),
        direct_bytes: Count::sum(rows.iter().map(|r| &r.direct_bytes)),
        inclusive_bytes: Count::sum(rows.iter().map(|r| &r.inclusive_bytes)),
        unique_raw_bytes: if rows.iter().all(|r| r.raw.is_some()) {
            Count::sum(
                unique
                    .values()
                    .map(|n| Count::exact(*n))
                    .collect::<Vec<_>>()
                    .iter(),
            )
        } else {
            Count::unavailable("raw source not captured")
        },
        repeated_raw_bytes: Count::sum(repeated.iter()),
        recorded_occurrences: rows.iter().map(|r| r.occurrences.len() as u64).sum(),
        runtime_reads: Count::unavailable(
            "declarations/emitted occurrences are not a runtime read trace",
        ),
    }
}

/// Measure only payload mappings supplied by this capture. Empty mappings do
/// not erase graph children. Positions/ancestry are never inferred from edges.
pub fn measure_graph(
    name: &str,
    graph: &BootGraph,
    raw: &BTreeMap<usize, Payload>,
    emitted: &BTreeMap<usize, Payload>,
) -> Lane {
    let snippets: Vec<_> = graph
        .contributions
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let payload = emitted.get(&i).cloned();
            let inclusive = payload
                .as_ref()
                .map(|p| Count::exact(p.bytes))
                .unwrap_or_else(|| Count::unavailable("no captured emitted range"));
            Snippet {
                contribution: i,
                source: row.seed,
                origin: row.origin.clone(),
                path: row.path.clone(),
                kind: row.kind.clone(),
                condition: None,
                package: None,
                raw: row.seed.and_then(|s| raw.get(&s).cloned()),
                emitted: payload,
                direct_bytes: Count::unavailable("no captured direct-byte mapping"),
                inclusive_bytes: inclusive,
                occurrences: row
                    .occurrences
                    .iter()
                    .map(|o| Occurrence {
                        source: o.source,
                        requested: o.requested.clone(),
                        parent: match o.parent {
                            vibe_spec::InclusionParent::Occurrence(p) => Some(p),
                            _ => None,
                        },
                        root: o.parent == vibe_spec::InclusionParent::Root,
                        emitted: None,
                    })
                    .collect(),
            }
        })
        .collect();
    Lane {
        name: name.into(),
        sources: graph
            .sources
            .iter()
            .enumerate()
            .map(|(index, source)| SourceMeasurement {
                index,
                identity: format!("{:?}", source.identity),
                raw: raw.get(&index).cloned(),
            })
            .collect(),
        artifact: None,
        totals: totals(snippets.iter()),
        snippets,
        gaps: graph.gaps.clone(),
    }
}

fn raw_file(root: &Path, path: &str) -> Result<Payload, String> {
    if Path::new(path)
        .components()
        .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err("source path is not project-relative".into());
    }
    let canonical_root = root.canonicalize().map_err(|e| e.to_string())?;
    let canonical = root.join(path).canonicalize().map_err(|e| e.to_string())?;
    if !canonical.starts_with(&canonical_root) {
        return Err("source resolves outside project".into());
    }
    let file = std::fs::File::open(canonical).map_err(|e| e.to_string())?;
    if !file.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err("source is not a regular file".into());
    }
    measure_reader(file).map_err(|e| e.to_string())
}

/// A single captured pair of committed artifacts; tree supplies its already
/// read texts here. Source files are read once per distinct path in this call.
/// No condition is evaluated and no runtime inclusion/billing is asserted.
pub fn capture_texts(root: &Path, static_text: Option<&str>, index_text: Option<&str>) -> Report {
    let mut lanes = Vec::new();
    let mut cache: BTreeMap<String, Result<Payload, String>> = BTreeMap::new();
    for (name, text) in [("static", static_text), ("index", index_text)] {
        let mut lane = Lane {
            name: name.into(),
            artifact: text.map(text_payload),
            snippets: Vec::new(),
            sources: Vec::new(),
            totals: totals(std::iter::empty()),
            gaps: Vec::new(),
        };
        if let Some(text) = text {
            let captured = if name == "static" {
                super::capture_static(name.into(), text)
                    .map(|c| (c.graph, Vec::new()))
                    .map_err(|e| e.to_string())
            } else {
                super::dynamic_index::capture_index(name.into(), text)
                    .map(|c| {
                        (
                            c.graph,
                            c.index.entries.into_iter().map(|e| e.when).collect(),
                        )
                    })
                    .map_err(|e| e.to_string())
            };
            match captured {
                Err(e) => lane.gaps.push(e),
                Ok((graph, conditions)) => {
                    let mut raw = BTreeMap::new();
                    let mut errors = Vec::new();
                    for row in &graph.contributions {
                        if let Some(source) = row.seed {
                            match cache
                                .entry(row.path.clone())
                                .or_insert_with(|| raw_file(root, &row.path))
                            {
                                Ok(p) => {
                                    raw.insert(source, p.clone());
                                }
                                Err(e) => errors.push(format!("{}: {e}", row.path)),
                            }
                        }
                    }
                    let emitted = if name == "static" {
                        super::static_summary::body_ranges(text)
                            .unwrap_or_default()
                            .into_iter()
                            .enumerate()
                            .map(|(i, r)| (i, text_payload(&text[r])))
                            .collect()
                    } else {
                        BTreeMap::new()
                    };
                    let artifact = lane.artifact;
                    lane = measure_graph(name, &graph, &raw, &emitted);
                    lane.artifact = artifact;
                    lane.gaps.extend(errors);
                    for (i, row) in lane.snippets.iter_mut().enumerate() {
                        row.condition = conditions.get(i).cloned().flatten();
                        if name == "static" {
                            if let Some(root) = row.occurrences.first_mut() {
                                root.emitted = row.emitted.clone();
                            }
                            for (occurrence, captured) in row
                                .occurrences
                                .iter_mut()
                                .zip(&graph.contributions[i].occurrences)
                                .skip(1)
                            {
                                if let Some((start, end)) = captured.artifact_lines {
                                    let mut offset = 0usize;
                                    let mut begin = None;
                                    let mut finish = None;
                                    for (line, segment) in text.split_inclusive('\n').enumerate() {
                                        if line as u64 + 1 == start {
                                            begin = Some(offset);
                                        }
                                        offset += segment.len();
                                        if line as u64 + 1 == end {
                                            finish = Some(offset);
                                            break;
                                        }
                                    }
                                    if let Some((a, b)) = begin.zip(finish) {
                                        occurrence.emitted = Some(text_payload(&text[a..b]));
                                    }
                                }
                            }
                        }
                    }
                    // Direct means emitted bytes outside recorded nested spans,
                    // not authored-source ancestry or transitive dependency cost.
                    if name == "static" {
                        for (row, captured) in lane.snippets.iter_mut().zip(&graph.contributions) {
                            let complete = captured
                                .occurrences
                                .iter()
                                .enumerate()
                                .all(|(i, _)| super::why_included(captured, i).is_some())
                                && row.occurrences.iter().all(|o| o.emitted.is_some());
                            if complete {
                                let children: Vec<_> = row
                                    .occurrences
                                    .iter()
                                    .filter(|o| o.parent == Some(0))
                                    .filter_map(|o| o.emitted.as_ref())
                                    .collect();
                                let child_bytes: u64 = children.iter().map(|p| p.bytes).sum();
                                if let Some(value) = row
                                    .emitted
                                    .as_ref()
                                    .and_then(|p| p.bytes.checked_sub(child_bytes))
                                {
                                    row.direct_bytes = Count::exact(value);
                                }
                            }
                        }
                    }
                    lane.totals = totals(lane.snippets.iter());
                }
            }
        } else {
            lane.gaps.push("artifact unavailable".into());
        }
        if text.is_none() || (!lane.gaps.is_empty() && lane.snippets.is_empty()) {
            lane.totals.direct_bytes = Count::unavailable("artifact unavailable or invalid");
            lane.totals.inclusive_bytes = Count::unavailable("artifact unavailable or invalid");
            lane.totals.unique_raw_bytes = Count::unavailable("artifact unavailable or invalid");
            lane.totals.repeated_raw_bytes = Count::unavailable("artifact unavailable or invalid");
            lane.totals.unique_emitted_bytes =
                Count::unavailable("artifact unavailable or invalid");
            lane.totals.repeated_emitted_bytes =
                Count::unavailable("artifact unavailable or invalid");
        }
        lanes.push(lane);
    }
    let mut report = Report { capture_id: String::new(), recipe: RECIPE.into(), basis: "committed marker regions: inclusive includes nested spans, direct excludes recorded children; raw repeated counts contribution appearances, not runtime reads; current raw sources are not asserted to have produced artifacts".into(), lanes, packages: BTreeMap::new(),
        tokens: TokenMeasurement { tokenizer: None, version: None, recipe: "unavailable: no tokenizer selected or executed".into(), count: Count::unavailable("no tokenizer selected or executed") },
        ambient_tail: Count::unavailable("unknown: ambient session tail not observed"), provider_billing: Count::unavailable("provider billing not observed"), scope_data_observability: Count::unavailable("RootTag/ScopeDataObservability not captured"), inspected_source_reads: cache.iter().map(|(path, result)| (path.clone(), if result.is_ok() { Count::exact(1) } else { Count::unavailable("read failed; see lane gaps") })).collect() };
    report.refresh_identity();
    report
}
impl Report {
    fn refresh_identity(&mut self) {
        self.capture_id.clear();
        self.capture_id = format!(
            "sha256:{:x}",
            Sha256::digest(serde_json::to_vec(&self).expect("serializable report"))
        );
    }
    /// Lock records prove package coordinates and slot paths; never derive
    /// inclusive dependency attribution from lock graph proportions.
    pub fn bind_packages(&mut self, lock: &vibe_core::manifest::Lockfile) {
        for lane in &mut self.lanes {
            for row in &mut lane.snippets {
                for p in &lock.packages {
                    let id = format!("{}/{}", p.group, p.name);
                    let slot = vibe_core::layout::current_vibedeps_root()
                        .join(format!("{}.{}", p.group, p.name))
                        .join(p.version.to_string());
                    if (lane.name == "static" && row.origin == id)
                        || (lane.name == "index" && Path::new(&row.path).starts_with(&slot))
                    {
                        row.package = Some(id);
                        break;
                    }
                }
            }
        }
        self.packages.clear();
        for p in &lock.packages {
            let id = format!("{}/{}", p.group, p.name);
            let rows: Vec<_> = self
                .lanes
                .iter()
                .flat_map(|l| &l.snippets)
                .filter(|r| r.package.as_ref() == Some(&id))
                .collect();
            if !rows.is_empty() {
                self.packages.insert(id, totals(rows.into_iter()));
            }
        }
        self.refresh_identity();
    }
}

pub fn capture_workspace(root: &Path) -> Report {
    let static_path = crate::boot_artifacts::resolve_static_path(root);
    let static_read = match &static_path {
        Ok(Some(path)) => std::fs::read_to_string(path)
            .map(Some)
            .map_err(|e| e.to_string()),
        Ok(None) => Ok(None),
        Err(e) => Err(e.to_string()),
    };
    let index_read = std::fs::read_to_string(root.join(vibe_core::layout::current_boot_index()));
    let mut report = capture_texts(
        root,
        static_read.as_ref().ok().and_then(|s| s.as_deref()),
        index_read.as_deref().ok(),
    );
    if let Err(e) = static_read {
        report.lanes[0]
            .gaps
            .push(format!("static read refused: {e}"));
    }
    if let Err(e) = index_read {
        if e.kind() != io::ErrorKind::NotFound {
            report.lanes[1]
                .gaps
                .push(format!("index read {:?}: {e}", e.kind()));
        }
    }
    let lock_path = root.join("vibe.lock");
    let lock = match std::fs::metadata(&lock_path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(vibe_core::manifest::Lockfile::empty(
            "measurement",
            "unrecorded",
        )),
        _ => vibe_core::manifest::Lockfile::read(&lock_path),
    };
    match lock {
        Ok(lock) => report.bind_packages(&lock),
        Err(e) => report
            .lanes
            .iter_mut()
            .for_each(|lane| lane.gaps.push(format!("package mapping unavailable: {e}"))),
    }
    report.refresh_identity();
    report
}

#[cfg(test)]
mod m_06_b_2_cases;

mod carrier;
pub use carrier::{CarrierReport, LinkedOccurrence, measure_retained};
