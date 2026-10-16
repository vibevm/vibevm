//! Adapter over the incumbent reservation port. All results live in DeployState.
use super::{JournalState, SubmissionJournal};
use std::sync::Arc;
use vibe_lifecycle::{
    LifecycleLease,
    client_deploy::{Approval, Cancellation, Host, Prepared},
};
use vibe_wire::generated::client_deploy::e1::reply::Reply;
pub(crate) struct Journal<'a> {
    pub host: &'a Host,
    pub plan: &'a Prepared,
    pub grant: Option<&'a Approval>,
    pub key: &'a str,
    pub cancel: &'a Cancellation,
    pub clock: &'a str,
}
impl SubmissionJournal for Journal<'_> {
    type Outcome = Reply;
    fn proposed_run_id(&self) -> String {
        self.host.proposed_run(self.key)
    }
    fn inspect(&mut self, _: &LifecycleLease, run: &str) -> Result<JournalState<Reply>, String> {
        Ok(match self.host.inspect(run)? {
            None => JournalState::Absent,
            Some((r, true)) => JournalState::Complete(r),
            Some((_, false)) => JournalState::Prepared,
        })
    }
    fn initialize(&mut self, _: Arc<LifecycleLease>, run: &str) -> Result<(), String> {
        self.host.initialize(self.plan, run)
    }
    fn invoke(&mut self, _: Arc<LifecycleLease>, run: &str) -> Result<Reply, String> {
        self.host
            .execute(self.plan, self.grant, run, false, self.cancel, self.clock)
    }
}
