//! Reusable presentation state; all observations come from a typed service.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-072#DEBUGGER-VIEWER-IDENTITY");
use serde::Serialize;
use vibe_wire::generated::boot_debugger::e1::{request as q, result as r};

/// The host adapter retains all context and resource authority.
pub trait Service {
    fn query(&self, command: q::Command) -> Result<r::Result, String>;
}

#[derive(Debug, Clone)]
pub enum Action {
    Next,
    Previous,
    Parent,
    Child,
    Select(usize),
    Snapshot(usize),
    Refresh,
    Detail,
    Source(u64),
    Compare(usize, usize),
    Show,
    Scenario { target: String, draft_json: String },
    Preview(q::CommandPreview),
}

/// Fields containing domain evidence are copied verbatim from service results.
#[derive(Debug, Serialize)]
pub struct Frame {
    pub snapshots: Vec<String>,
    pub snapshot_id: String,
    pub selected: Option<usize>,
    pub start: usize,
    pub total: usize,
    pub nodes: Vec<r::Node>,
    pub sources: Vec<r::Source>,
    pub gaps: Vec<String>,
    pub comparison_gaps: Vec<(String, Vec<String>)>,
    pub observation: Option<r::Result>,
    pub diagnostic: Option<String>,
}

pub struct Viewer<S> {
    service: S,
    snapshots: Vec<String>,
    current: r::BodyInspect,
    selected: Option<usize>,
    start: usize,
    comparison_gaps: Vec<(String, Vec<String>)>,
}

impl<S: Service> Viewer<S> {
    pub fn new(service: S) -> Result<Self, String> {
        let id = capture(&service)?;
        let current = inspect(&service, &id)?;
        Ok(Self {
            service,
            snapshots: vec![id],
            selected: (!current.nodes.is_empty()).then_some(0),
            current,
            start: 0,
            comparison_gaps: vec![],
        })
    }

    fn snapshot(&self, index: usize) -> Result<String, String> {
        self.snapshots
            .get(index)
            .cloned()
            .ok_or_else(|| "unknown snapshot ordinal".into())
    }

    fn node(&self) -> Result<&r::Node, String> {
        self.selected
            .and_then(|i| self.current.nodes.get(i))
            .ok_or_else(|| "no selected captured occurrence".into())
    }

    fn select(&mut self, index: usize) -> Result<(), String> {
        if index >= self.current.nodes.len() {
            return Err("row ordinal outside captured graph".into());
        }
        self.selected = Some(index);
        self.start = index / 20 * 20;
        Ok(())
    }

    fn load(&mut self, id: &str) -> Result<(), String> {
        let current = inspect(&self.service, id)?;
        self.selected = (!current.nodes.is_empty()).then_some(0);
        self.start = 0;
        self.current = current;
        Ok(())
    }

    pub fn apply(&mut self, action: Action) -> Result<Option<r::Result>, String> {
        let command = match action {
            Action::Scenario { target, draft_json } => {
                q::Command::Scenario(Box::new(q::CommandScenario {
                    snapshot_id: self.current.capture_id.clone(),
                    target,
                    draft_json,
                }))
            }
            Action::Preview(request) => q::Command::Preview(Box::new(request)),
            Action::Show => return Ok(None),
            Action::Select(i) => {
                self.select(i)?;
                return Ok(None);
            }
            Action::Next | Action::Previous => {
                let i = self.selected.ok_or("no selected captured occurrence")?;
                let next = if matches!(action, Action::Next) {
                    i.checked_add(1)
                } else {
                    i.checked_sub(1)
                };
                self.select(next.ok_or("edge of captured graph")?)?;
                return Ok(None);
            }
            Action::Parent | Action::Child => {
                let n = self.node()?;
                let next = self
                    .current
                    .nodes
                    .iter()
                    .position(|other| {
                        other.contribution == n.contribution
                            && if matches!(action, Action::Parent) {
                                n.parent == Some(other.occurrence)
                            } else {
                                other.parent == Some(n.occurrence)
                            }
                    })
                    .ok_or("captured relation unavailable or no related occurrence")?;
                self.select(next)?;
                return Ok(None);
            }
            Action::Snapshot(i) => {
                self.load(&self.snapshot(i)?)?;
                return Ok(None);
            }
            Action::Refresh => {
                let id = capture(&self.service)?;
                self.load(&id)?;
                if !self.snapshots.contains(&id) {
                    self.snapshots.push(id);
                }
                return Ok(None);
            }
            Action::Detail => {
                let n = self.node()?;
                q::Command::Explain(Box::new(q::CommandExplain {
                    snapshot_id: self.current.capture_id.clone(),
                    contribution: n.contribution,
                    occurrence: n.occurrence,
                }))
            }
            Action::Source(offset) => {
                let n = self.node()?;
                let source = self
                    .current
                    .sources
                    .iter()
                    .find(|s| s.source == n.source)
                    .ok_or("captured source mapping unavailable")?;
                q::Command::Chunk(Box::new(q::CommandChunk {
                    snapshot_id: self.current.capture_id.clone(),
                    resource_id: source.artifact_resource_id.clone(),
                    filter: q::CommandChunkFilter::Raw,
                    offset: offset.to_string(),
                    limit: 2048,
                }))
            }
            Action::Compare(a, b) => {
                let a = self.snapshot(a)?;
                let b = self.snapshot(b)?;
                let result =
                    self.service
                        .query(q::Command::Compare(Box::new(q::CommandCompare {
                            snapshot_id: a.clone(),
                            other_snapshot_id: b.clone(),
                        })))?;
                self.comparison_gaps = vec![
                    (a.clone(), inspect(&self.service, &a)?.gaps),
                    (b.clone(), inspect(&self.service, &b)?.gaps),
                ];
                return Ok(Some(result));
            }
        };
        self.service.query(command).map(Some)
    }

    pub fn frame(&self, observation: Option<r::Result>, diagnostic: Option<String>) -> Frame {
        Frame {
            snapshots: self.snapshots.clone(),
            snapshot_id: self.current.capture_id.clone(),
            selected: self.selected,
            start: self.start,
            total: self.current.nodes.len(),
            nodes: self
                .current
                .nodes
                .iter()
                .skip(self.start)
                .take(20)
                .cloned()
                .collect(),
            sources: self.current.sources.clone(),
            gaps: self.current.gaps.clone(),
            observation,
            diagnostic,
            comparison_gaps: self.comparison_gaps.clone(),
        }
    }
}

#[cfg(test)]
#[path = "viewer_tests.rs"]
mod m_06_c_2_cases;

fn capture(service: &impl Service) -> Result<String, String> {
    let result = service.query(q::Command::Capture(Box::new(q::CommandCapture {})))?;
    match result.body {
        r::Body::Capture(c) => Ok(c.capture_id),
        _ => Err("expected capture result".into()),
    }
}
fn inspect(service: &impl Service, id: &str) -> Result<r::BodyInspect, String> {
    let result = service.query(q::Command::Inspect(Box::new(q::CommandInspect {
        snapshot_id: id.into(),
    })))?;
    match result.body {
        r::Body::Inspect(c) if c.capture_id == id => Ok(*c),
        _ => Err("expected matching immutable inspection".into()),
    }
}
