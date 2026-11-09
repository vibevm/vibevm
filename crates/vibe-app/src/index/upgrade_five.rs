//! Typed promotion of protocol5 records into the current protocol6 shape:
//! no path directory, slot off state or applied link PATH entry is fabricated.
use std::collections::BTreeMap;
use vibe_wire::generated::application::{e4::index as old, e5::index as new};
impl Promote<vibe_wire::generated::shared::ConfigurationRecord>
    for vibe_wire::generated::shared::ConfigurationRecord
{
    fn promote(self) -> Self {
        self
    }
}
impl Promote<vibe_wire::generated::shared::ApplicationPendingTarget>
    for vibe_wire::generated::shared::ApplicationPendingTarget
{
    fn promote(self) -> Self {
        self
    }
}
impl Promote<vibe_wire::generated::shared::ConfigurationParameterOrigin>
    for vibe_wire::generated::shared::ConfigurationParameterOrigin
{
    fn promote(self) -> Self {
        self
    }
}
impl Promote<vibe_wire::generated::shared::SourceEvidence>
    for vibe_wire::generated::shared::SourceEvidence
{
    fn promote(self) -> Self {
        self
    }
}
trait Promote<T> {
    fn promote(self) -> T;
}
impl Promote<String> for String {
    fn promote(self) -> String {
        self
    }
}
impl Promote<bool> for bool {
    fn promote(self) -> bool {
        self
    }
}
impl<T, U> Promote<Option<U>> for Option<T>
where
    T: Promote<U>,
{
    fn promote(self) -> Option<U> {
        self.map(Promote::promote)
    }
}
impl<T, U> Promote<Vec<U>> for Vec<T>
where
    T: Promote<U>,
{
    fn promote(self) -> Vec<U> {
        self.into_iter().map(Promote::promote).collect()
    }
}
impl<T, U> Promote<BTreeMap<String, U>> for BTreeMap<String, T>
where
    T: Promote<U>,
{
    fn promote(self) -> BTreeMap<String, U> {
        self.into_iter().map(|(k, v)| (k, v.promote())).collect()
    }
}
macro_rules! row {($name:ident,[$($field:ident),*] $(, $extra:ident = $default:expr)* )=>{
    impl Promote<new::$name> for old::$name {
        fn promote(self)->new::$name {new::$name {$($field:self.$field.promote(),)* $($extra:$default,)*}}
    }
};}
macro_rules! vocabulary {($name:ident,[$($variant:ident),*])=>{
    impl Promote<new::$name> for old::$name {
        fn promote(self)->new::$name {match self {$(old::$name::$variant=>new::$name::$variant,)*}}
    }
};}
pub(super) fn promote(value: old::ApplicationIndex) -> new::ApplicationIndex {
    value.promote()
}
row!(ApplicationIndex, [protocol, installations, slots]);
row!(AcceptedLicense, [name, accepted_at]);
row!(
    AuthoredCommands,
    [uninstall, force_uninstall, update, interpreter]
);
row!(EnvEntry, [step, op, name, value, previous, previous_kind]);
vocabulary!(EnvOp, [PathPrepend, Set, Unset]);
row!(IconSet, [svg, png, ico, icns, symbolic, maskable]);
row!(
    Installation,
    [
        slot,
        package,
        option,
        platform,
        arch,
        method,
        library,
        directory,
        app_root,
        portable,
        launchers,
        public_commands,
        commands,
        features,
        parameters,
        license,
        icon,
        icon_set,
        menu_group,
        categories,
        keywords,
        steps,
        hidden,
        mark,
        requires,
        required_by,
        env,
        held,
        replaced,
        superseded_by,
        configuration,
        pending,
        secret_origins,
        source_evidence
    ],
    path_dirs = Vec::new()
);
vocabulary!(Interpreter, [PosixSh, WindowsCmd, WindowsPowershell]);
row!(
    Launcher,
    [
        id,
        name,
        description,
        executable,
        args,
        working_directory,
        icon,
        icon_set,
        terminal,
        default,
        kind,
        menu,
        categories,
        keywords,
        generic_name,
        window_class,
        mime_types,
        names,
        descriptions,
        actions,
        source
    ]
);
row!(LauncherAction, [id, name, args, icon, icon_set]);
vocabulary!(LauncherKind, [Command, Gui, Tui]);
vocabulary!(LauncherSource, [Authored, DesktopEntry]);
vocabulary!(Mark, [Auto, Manual]);
vocabulary!(Method, [Exe, Msi, Shell, SourceBuild, Steps, TarGz, Zip]);
vocabulary!(Outcome, [Delegated, Done, Failed, Interrupted, Skipped]);
row!(Package, [group, name, version]);
vocabulary!(Phase, [Finish, Prepare]);
row!(PublicCommand, [name, launcher]);
row!(Requirement, [package, key, scope, step]);
vocabulary!(Scope, [Build, Runtime]);
row!(Slot, [default], off = false, path_env = Vec::new());
row!(
    StepRecord,
    [
        id,
        kind,
        phase,
        outcome,
        uninstall,
        force_uninstall,
        interpreter
    ]
);
vocabulary!(ValueKind, [ExpandString, String]);
