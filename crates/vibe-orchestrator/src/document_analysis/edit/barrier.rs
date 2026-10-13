//! Existing replacement barrier delegates common registered coordination.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#DOCUMENT-EDIT-BARRIER-LAW");
use super::*;
use std::{
    collections::BTreeSet,
    time::{Duration, Instant},
};
pub struct EditBarrier {
    pub(super) plan: EditPlan,
    pub(super) id: u64,
    pub(super) expires: Instant,
    pub(super) answers: Mutex<Answers>,
    pub(super) applying: Mutex<()>,
    expected: fence::ExpectedBuffers,
}
#[derive(Default)]
pub(super) struct Answers {
    pub acknowledged: BTreeSet<u64>,
    pub refused: bool,
    pub consumed: bool,
}
impl std::fmt::Debug for Answers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Answers")
            .field("acknowledged", &self.acknowledged.len())
            .field("refused", &self.refused)
            .field("consumed", &self.consumed)
            .finish()
    }
}
impl std::fmt::Debug for EditBarrier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EditBarrier")
            .field("plan", &self.plan)
            .field("barrier", &self.id)
            .finish()
    }
}
pub(super) fn expected(plan: &EditPlan) -> fence::ExpectedBuffers {
    plan.files
        .iter()
        .map(|(id, file)| (*id, Some(Arc::from(file.before.bytes.as_slice()))))
        .collect()
}
impl EditPlan {
    pub fn barrier(self, ttl: Duration) -> Result<EditBarrier, EditError> {
        let expected = expected(&self);
        let (id, expires) = fence::reserve(&self.authority, &self.participants, &expected, ttl)?;
        Ok(EditBarrier {
            plan: self,
            id,
            expires,
            answers: Mutex::new(Answers::default()),
            applying: Mutex::new(()),
            expected,
        })
    }
}
impl DocumentClient {
    pub fn acknowledge_edit(&self, barrier: &EditBarrier, approve: bool) -> Result<(), EditError> {
        fence::acknowledge(
            self,
            &barrier.plan.authority,
            &barrier.plan.participants,
            &barrier.expected,
            barrier.id,
            barrier.expires,
            &barrier.answers,
            approve,
        )
    }
}
impl EditBarrier {
    pub(super) fn check_time(&self) -> Result<(), EditError> {
        fence::time(self.expires)
    }
    pub(super) fn check_participants(&self) -> Result<(), EditError> {
        self.check_time()?;
        for p in &self.plan.participants {
            fence::validate(p, &self.expected)?
        }
        Ok(())
    }
    pub(super) fn ready(&self) -> Result<(), EditError> {
        fence::ready(
            self.expires,
            &self.answers,
            &self.plan.participants,
            &self.expected,
        )
    }
}
impl Drop for EditBarrier {
    fn drop(&mut self) {
        fence::release(
            &self.plan.authority,
            &self.plan.participants,
            self.id,
            false,
        )
    }
}
