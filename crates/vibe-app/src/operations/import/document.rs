//! Exact old/new finite cohort worlds behind a small explicit inspection view.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-CONTINUATION");
use vibe_wire::generated::{
    application::{e1::import_work as old, e2::import_work as full},
    shared as w,
};

#[derive(Debug, Clone)]
pub(crate) enum ImportWork {
    Legacy(old::ImportWork),
    Full(full::ImportWork),
}
impl ImportWork {
    pub(crate) fn target(&self) -> &w::ApplicationImportTarget {
        match self {
            Self::Legacy(d) => &d.target,
            Self::Full(d) => &d.target,
        }
    }
    pub(crate) fn phase(&self) -> &w::ApplicationImportPhase {
        match self {
            Self::Legacy(d) => &d.phase,
            Self::Full(d) => &d.phase,
        }
    }
    pub(crate) fn context(&self) -> &w::ApplicationImportContext {
        match self {
            Self::Legacy(d) => &d.context,
            Self::Full(d) => &d.context,
        }
    }
    pub(crate) fn cursor(&self) -> u32 {
        match self {
            Self::Legacy(d) => d.cohort.cursor,
            Self::Full(d) => d.cohort.cursor,
        }
    }
    pub(crate) fn root_count(&self) -> usize {
        match self {
            Self::Legacy(d) => d.cohort.rows.len(),
            Self::Full(d) => d.cohort.roots.len(),
        }
    }
    pub(crate) fn root_key(&self, index: usize) -> Option<&str> {
        match self {
            Self::Legacy(d) => d.cohort.rows.get(index).map(|r| r.key.as_str()),
            Self::Full(d) => d.cohort.roots.get(index).map(String::as_str),
        }
    }
    pub(crate) fn current(&self) -> Option<&w::ApplicationPendingTarget> {
        match self {
            Self::Legacy(d) => d.cohort.current.as_ref(),
            Self::Full(d) => d.cohort.current.as_ref(),
        }
    }
    pub(crate) fn owns_attempt(&self, row: u32, child: &w::ApplicationPendingTarget) -> bool {
        (self.root_key(row as usize) == Some(child.key.as_str())
            && ((self.cursor() == row && self.current() == Some(child))
                || self.results().iter().any(|result| {
                    result.row_index == Some(row) && result.target.as_ref() == Some(child)
                })))
            || self
                .full()
                .is_some_and(|d| super::receipts::owns(d, row, child))
    }
    /// A terminal Refused attempt may be retired and retried, never reused as success.
    pub(crate) fn released_refusal(
        &self,
        state: &crate::steps::StepsFile,
    ) -> Result<bool, crate::AppError> {
        let Some(d) = self.full() else {
            return Ok(false);
        };
        let Some(op) = &state.operation else {
            return Ok(false);
        };
        if op.parent.is_some()
            || op.phase != w::ApplicationPendingOperationPhase::Complete
            || !d.cohort.results.iter().any(|r| {
                r.action == w::ApplicationPendingImportResultAction::Refused
                    && r.target.as_ref() == Some(&op.target)
            })
        {
            return Ok(false);
        }
        let empty = crate::ImportSecrets::default();
        let scope = crate::ImportScope::new(
            d.target.clone(),
            d.cohort.roots.clone(),
            d.cohort.bindings.clone(),
            Default::default(),
            &empty,
        )?;
        Ok(super::pending::matches_import_scope(
            state,
            scope.fingerprint(),
        ))
    }
    /// Historical refused attempts do not name a current effect after retirement.
    pub(crate) fn inspect_result(
        &self,
        settings: &crate::SettingsRoot,
        result: &w::ApplicationPendingImportResult,
    ) -> Result<bool, crate::AppError> {
        if result.action != w::ApplicationPendingImportResultAction::Refused {
            return Ok(true);
        }
        let Some(target) = &result.target else {
            return Ok(false);
        };
        let path = super::pending::work(settings, &super::pending::coordinate(&target.key)?);
        let Some(state) = crate::steps::StepsFile::load(&path)? else {
            return Ok(false);
        };
        if state
            .operation
            .as_ref()
            .is_some_and(|op| op.target != *target)
        {
            return Ok(false);
        };
        Ok(!self.released_refusal(&state)?)
    }
    pub(crate) fn receipt_targets(&self) -> Vec<(&w::ApplicationPendingTarget, u32)> {
        self.full()
            .into_iter()
            .flat_map(|d| {
                d.cohort
                    .completed_bindings
                    .values()
                    .map(|r| (&r.target, r.origin_row))
            })
            .collect()
    }
    pub(crate) fn results(&self) -> &[w::ApplicationPendingImportResult] {
        match self {
            Self::Legacy(d) => &d.cohort.results,
            Self::Full(d) => &d.cohort.results,
        }
    }
    pub(crate) fn arrangement_done(&self) -> bool {
        match self {
            Self::Legacy(d) => d.cohort.arrangement_done,
            Self::Full(d) => d.cohort.arrangement_done,
        }
    }
    /// Cursor roots and their recorded binding edges define possible effects;
    /// unrelated carried orphan rows and old informational Auto hints do not.
    pub(crate) fn affected_keys(&self) -> Vec<&str> {
        match self {
            Self::Legacy(d) => d.cohort.rows.iter().map(|r| r.key.as_str()).collect(),
            Self::Full(d) => {
                let mut seen = std::collections::BTreeSet::new();
                let mut queue: Vec<&str> = d.cohort.roots.iter().map(String::as_str).collect();
                while let Some(key) = queue.pop() {
                    if !seen.insert(key) {
                        continue;
                    }
                    if let Some(binding) = d.cohort.bindings.get(key) {
                        queue.extend(binding.application.requires.iter().map(|r| r.key.as_str()));
                    }
                }
                seen.into_iter().collect()
            }
        }
    }
    pub(crate) fn full(&self) -> Option<&full::ImportWork> {
        match self {
            Self::Full(d) => Some(d),
            Self::Legacy(_) => None,
        }
    }
    pub(crate) fn set_current(&mut self, target: Option<w::ApplicationPendingTarget>) {
        match self {
            Self::Legacy(d) => d.cohort.current = target,
            Self::Full(d) => d.cohort.current = target,
        }
    }
    pub(crate) fn held(&self, index: usize) -> Option<bool> {
        match self {
            Self::Legacy(d) => d.cohort.rows.get(index).map(|r| r.held),
            Self::Full(d) => d
                .cohort
                .roots
                .get(index)
                .and_then(|k| d.cohort.bindings.get(k))
                .map(|r| r.application.held),
        }
    }
    pub(crate) fn agent_modes(
        &self,
        index: usize,
    ) -> Option<&std::collections::BTreeMap<String, w::ApplicationAgentMode>> {
        match self {
            Self::Legacy(d) => d.cohort.rows.get(index).map(|r| &r.agent_modes),
            Self::Full(d) => d
                .cohort
                .roots
                .get(index)
                .and_then(|k| d.cohort.bindings.get(k))
                .map(|r| &r.agent_modes),
        }
    }
    pub(crate) fn has_arrangement(&self) -> bool {
        match self {
            Self::Legacy(d) => {
                !d.cohort.collections.is_empty()
                    || !d.cohort.pins.is_empty()
                    || !d.cohort.launchers.is_empty()
            }
            Self::Full(d) => {
                !d.cohort.collections.is_empty()
                    || !d.cohort.pins.is_empty()
                    || !d.cohort.launchers.is_empty()
            }
        }
    }
}
