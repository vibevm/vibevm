//! Typed version bridge for unchanged step rows; operation metadata has one steps2 owner.
use std::collections::BTreeMap;
use vibe_wire::generated::application::{e1::steps as old, e2::steps as new};
use vibe_wire::generated::shared::ApplicationPendingOperation;
trait Adapt<T> {
    fn adapt(self) -> T;
}
impl Adapt<String> for String {
    fn adapt(self) -> String {
        self
    }
}
impl Adapt<bool> for bool {
    fn adapt(self) -> bool {
        self
    }
}
impl Adapt<u32> for u32 {
    fn adapt(self) -> u32 {
        self
    }
}
impl<T, U> Adapt<Option<U>> for Option<T>
where
    T: Adapt<U>,
{
    fn adapt(self) -> Option<U> {
        self.map(Adapt::adapt)
    }
}
impl<T, U> Adapt<Vec<U>> for Vec<T>
where
    T: Adapt<U>,
{
    fn adapt(self) -> Vec<U> {
        self.into_iter().map(Adapt::adapt).collect()
    }
}
impl<T, U> Adapt<BTreeMap<String, U>> for BTreeMap<String, T>
where
    T: Adapt<U>,
{
    fn adapt(self) -> BTreeMap<String, U> {
        self.into_iter().map(|(k, v)| (k, v.adapt())).collect()
    }
}
macro_rules! row{($name:ident,[$($f:ident),*])=>{
    impl Adapt<new::$name> for old::$name{fn adapt(self)->new::$name{new::$name{$($f:self.$f.adapt(),)*}}}
    impl Adapt<old::$name> for new::$name{fn adapt(self)->old::$name{old::$name{$($f:self.$f.adapt(),)*}}}
};}
macro_rules! vocabulary{($name:ident,[$($v:ident),*])=>{
    impl Adapt<new::$name> for old::$name{fn adapt(self)->new::$name{match self{$(old::$name::$v=>new::$name::$v,)*}}}
    impl Adapt<old::$name> for new::$name{fn adapt(self)->old::$name{match self{$(new::$name::$v=>old::$name::$v,)*}}}
};}
pub(super) fn promote(
    rows: old::StepsFile,
    operation: ApplicationPendingOperation,
) -> new::StepsFile {
    new::StepsFile {
        protocol: "vibe-app-steps/2".into(),
        operation,
        package: rows.package.adapt(),
        option: rows.option,
        features: rows.features,
        steps: rows.steps.adapt(),
    }
}
pub(super) fn split(document: new::StepsFile) -> (old::StepsFile, ApplicationPendingOperation) {
    (
        old::StepsFile {
            protocol: "vibe-app-steps/1".into(),
            package: document.package.adapt(),
            option: document.option,
            features: document.features,
            steps: document.steps.adapt(),
        },
        document.operation,
    )
}
vocabulary!(AgentMode, [Interactive, Unattended]);
row!(
    AgentRun,
    [agent, mode, envelope, duration_ms, status, message]
);
vocabulary!(AgentStatus, [Fail, Ok, Parked]);
row!(EnvEntry, [step, op, name, value, previous, previous_kind]);
vocabulary!(EnvOp, [PathPrepend, Set, Unset]);
vocabulary!(Interpreter, [PosixSh, WindowsCmd, WindowsPowershell]);
vocabulary!(Outcome, [Delegated, Done, Failed, Interrupted, Skipped]);
row!(Package, [group, name, version]);
vocabulary!(
    RemapRowLayer,
    [Invocation, Local, ProjectPackage, UserPackage]
);
row!(RemapRow, [layer, source, index, reason]);
row!(ServedBy, [target, row]);
row!(
    Step,
    [
        id,
        kind,
        outcome,
        reason,
        started,
        finished,
        uninstall,
        force_uninstall,
        interpreter,
        agent,
        env,
        inputs
    ]
);
row!(StepInput, [name, served_by]);
vocabulary!(ValueKind, [ExpandString, String]);
