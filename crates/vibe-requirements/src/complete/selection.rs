//! Capture/filter-bound selection and loss-detecting internal metadata pages.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-FACT-PAGING");
use super::*;
#[derive(Debug, Clone)]
pub enum SelectedFactState<'a> {
    Observed(&'a CapturedFact),
    Missing,
    Unavailable,
    Unclassified,
    OutOfScope,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectionGapKind {
    Missing,
    Unavailable,
    Unclassified,
    OutOfScope,
    DuplicateRequested,
    Truncated,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionGap {
    pub address: String,
    pub kind: SelectionGapKind,
}
#[derive(Debug)]
pub struct FactSelection<'a> {
    source: &'a CapturedFactSet,
    filter: String,
    expected: BTreeSet<String>,
    states: BTreeMap<String, SelectedFactState<'a>>,
    omitted: Vec<String>,
    gaps: Vec<SelectionGap>,
    document_gaps: Vec<String>,
}
fn state<'a>(source: &'a CapturedFactSet, address: &str) -> SelectedFactState<'a> {
    if let Some(fact) = source.facts.get(address) {
        return SelectedFactState::Observed(fact);
    }
    let coordinate = format!(
        "{}/{}",
        source.inventory.pin().group.as_ref().unwrap(),
        source.inventory.pin().name
    );
    if !address.starts_with(&format!("spec://{coordinate}/")) {
        return SelectedFactState::OutOfScope;
    }
    let matching: Vec<_> = source
        .documents
        .values()
        .filter(|d| address.starts_with(&d.address_prefix))
        .collect();
    if matching.iter().any(|d| d.state == DocumentState::Native) {
        return SelectedFactState::Missing;
    }
    if matching.iter().any(|d| {
        matches!(
            d.state,
            DocumentState::Unavailable
                | DocumentState::Invalid
                | DocumentState::Unsupported
                | DocumentState::Ambiguous
        )
    }) {
        return SelectedFactState::Unavailable;
    }
    if matching
        .iter()
        .any(|d| d.state == DocumentState::Unclassified)
        || source.gaps.contains(&CaptureGap::NoDescriptors)
    {
        return SelectedFactState::Unclassified;
    }
    if matching.iter().any(|d| d.state == DocumentState::Inert) {
        return SelectedFactState::OutOfScope;
    }
    SelectedFactState::Missing
}
pub(super) fn exact<'a>(source: &'a CapturedFactSet, addresses: &[String]) -> FactSelection<'a> {
    let expected: BTreeSet<_> = addresses.iter().cloned().collect();
    let mut gaps = vec![];
    let mut states = BTreeMap::new();
    let mut seen = BTreeSet::new();
    for address in addresses {
        if !seen.insert(address) {
            gaps.push(SelectionGap {
                address: address.clone(),
                kind: SelectionGapKind::DuplicateRequested,
            })
        }
    }
    for address in &expected {
        let outcome = state(source, address);
        let kind = match outcome {
            SelectedFactState::Observed(_) => None,
            SelectedFactState::Missing => Some(SelectionGapKind::Missing),
            SelectedFactState::Unavailable => Some(SelectionGapKind::Unavailable),
            SelectedFactState::Unclassified => Some(SelectionGapKind::Unclassified),
            SelectedFactState::OutOfScope => Some(SelectionGapKind::OutOfScope),
        };
        if let Some(kind) = kind {
            gaps.push(SelectionGap {
                address: address.clone(),
                kind,
            })
        }
        states.insert(address.clone(), outcome);
    }
    let omitted = source
        .facts
        .keys()
        .filter(|a| !expected.contains(*a))
        .cloned()
        .collect();
    let filter = filter_id(source, "exact-addresses", &expected);
    FactSelection {
        source,
        filter,
        expected,
        states,
        omitted,
        gaps,
        document_gaps: vec![],
    }
}
pub(super) fn all(source: &CapturedFactSet) -> FactSelection<'_> {
    let mut result = exact(source, &source.facts.keys().cloned().collect::<Vec<_>>());
    result.filter = filter_id(source, "whole-source", &result.expected);
    result.document_gaps = source
        .documents
        .iter()
        .filter(|(_, d)| !matches!(d.state, DocumentState::Native | DocumentState::Inert))
        .map(|(p, _)| p.clone())
        .collect();
    result
}
fn filter_id(source: &CapturedFactSet, kind: &str, expected: &BTreeSet<String>) -> String {
    let mut framed = crate::digest::seed(b"vibe-requirements-complete-filter\0epoch=1\0");
    crate::digest::field(&mut framed, b"capture", source.id.as_bytes());
    crate::digest::field(&mut framed, b"scope-kind", kind.as_bytes());
    crate::digest::count(&mut framed, b"addresses", expected.len());
    for address in expected {
        crate::digest::field(&mut framed, b"address", address.as_bytes())
    }
    crate::digest::finish(framed)
}
impl<'a> FactSelection<'a> {
    pub fn capture_id(&self) -> &str {
        &self.source.id
    }
    pub fn filter_id(&self) -> &str {
        &self.filter
    }
    pub fn states(&self) -> &BTreeMap<String, SelectedFactState<'a>> {
        &self.states
    }
    pub fn gaps(&self) -> &[SelectionGap] {
        &self.gaps
    }
    pub fn document_gaps(&self) -> &[String] {
        &self.document_gaps
    }
    pub fn capture_gaps(&self) -> &[CaptureGap] {
        &self.source.gaps
    }
    pub fn omitted_addresses(&self) -> &[String] {
        &self.omitted
    }
    pub fn is_complete(&self) -> bool {
        self.gaps.is_empty() && self.document_gaps.is_empty() && self.source.gaps.is_empty()
    }
    pub fn observed(&self) -> Vec<&'a CapturedFact> {
        self.states
            .values()
            .filter_map(|s| {
                if let SelectedFactState::Observed(f) = s {
                    Some(*f)
                } else {
                    None
                }
            })
            .collect()
    }
    pub fn page(&self, offset: usize, limit: usize) -> Result<FactPage<'a>, PageError> {
        if limit == 0 || limit > 256 {
            return Err(PageError::InvalidLimit);
        }
        let rows = self.observed();
        if offset > rows.len() {
            return Err(PageError::InvalidOffset);
        }
        let end = offset.saturating_add(limit).min(rows.len());
        Ok(FactPage {
            source: self.source,
            filter: self.filter.clone(),
            rows: rows[offset..end].to_vec(),
        })
    }
    pub fn assemble(&self, pages: &[FactPage<'a>]) -> Result<PageAssembly<'a>, PageError> {
        let mut rows = BTreeMap::new();
        for page in pages {
            if !std::ptr::eq(page.source, self.source) {
                return Err(PageError::MixedCapture);
            }
            if page.filter != self.filter {
                return Err(PageError::MixedFilter);
            }
            for row in &page.rows {
                if rows.insert(row.authored.address.clone(), *row).is_some() {
                    return Err(PageError::DuplicateRow);
                }
            }
        }
        let mut gaps = self.gaps.clone();
        for address in &self.expected {
            if matches!(
                self.states.get(address),
                Some(SelectedFactState::Observed(_))
            ) && !rows.contains_key(address)
            {
                gaps.push(SelectionGap {
                    address: address.clone(),
                    kind: SelectionGapKind::Truncated,
                })
            }
        }
        Ok(PageAssembly {
            complete: self.is_complete() && gaps.is_empty(),
            rows: rows.into_values().collect(),
            gaps,
        })
    }
}
#[derive(Debug)]
pub struct FactPage<'a> {
    source: &'a CapturedFactSet,
    filter: String,
    rows: Vec<&'a CapturedFact>,
}
impl<'a> FactPage<'a> {
    pub fn rows(&self) -> &[&'a CapturedFact] {
        &self.rows
    }
    pub fn capture_id(&self) -> &str {
        &self.source.id
    }
    pub fn filter_id(&self) -> &str {
        &self.filter
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PageError {
    InvalidLimit,
    InvalidOffset,
    MixedCapture,
    MixedFilter,
    DuplicateRow,
}
#[derive(Debug)]
pub struct PageAssembly<'a> {
    complete: bool,
    rows: Vec<&'a CapturedFact>,
    gaps: Vec<SelectionGap>,
}
impl<'a> PageAssembly<'a> {
    pub fn is_complete(&self) -> bool {
        self.complete
    }
    pub fn rows(&self) -> &[&'a CapturedFact] {
        &self.rows
    }
    pub fn gaps(&self) -> &[SelectionGap] {
        &self.gaps
    }
}
