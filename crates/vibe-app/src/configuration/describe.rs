//! Existing app metadata and executor declarations projected beside common definitions.
use std::collections::BTreeMap;
use vibe_wire::generated::{application_describe as old, configuration::e1::description as new};
impl Promote<u32> for u32 {
    fn promote(self) -> u32 {
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
macro_rules! row {($name:ident,[$($field:ident),*] $(,$extra:ident)* )=>{
    impl Promote<new::$name> for old::$name {
        fn promote(self)->new::$name {new::$name {$($field:self.$field.promote(),)* $($extra:None,)*}}
    }
};}
macro_rules! vocabulary {($name:ident,[$($variant:ident),*])=>{
    impl Promote<new::$name> for old::$name {
        fn promote(self)->new::$name {match self {$(old::$name::$variant=>new::$name::$variant,)*}}
    }
};}
impl Promote<i64> for i64 {
    fn promote(self) -> i64 {
        self
    }
}
impl Promote<u64> for u64 {
    fn promote(self) -> u64 {
        self
    }
}
row!(
    Application,
    [
        id, portable, icon, icon_set, menu_group, categories, keywords
    ]
);
row!(Choice, [value, name]);
row!(
    Condition,
    [
        feature,
        param,
        param_equals,
        command_present,
        file_present,
        env_present,
        agent_configured,
        offline
    ]
);
row!(EnvSet, [name, value]);
row!(
    Feature,
    [
        id,
        name,
        description,
        default,
        parent,
        required,
        requires,
        exclusive_group
    ]
);
vocabulary!(Form, [Installation, Steps]);
row!(IconFile, [slot, size, template, path]);
row!(IconSet, [maskable, files]);
vocabulary!(IconSlot, [Icns, Ico, Png, Svg, Symbolic]);
row!(
    InstallerOption,
    [
        id,
        name,
        platform,
        platform_name,
        arch,
        arch_name,
        form,
        result,
        app_root,
        portable,
        launchers_from,
        launchers,
        public_commands,
        features,
        default_features,
        profiles,
        parameters,
        license,
        requires,
        steps
    ],
    configuration
);
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
        source,
        categories,
        keywords,
        generic_name,
        window_class,
        mime_types,
        names,
        descriptions,
        actions
    ]
);
row!(LauncherAction, [id, name, args, icon_set]);
vocabulary!(LauncherKind, [Command, Gui, Tui]);
vocabulary!(LauncherSource, [Authored, DesktopEntry]);
row!(License, [file, name]);
row!(Package, [group, name, version]);
row!(ParamEquals, [id, value]);
row!(
    Parameter,
    [
        id,
        name,
        description,
        type_,
        default,
        required,
        min,
        max,
        choices,
        must_exist,
        secret
    ]
);
vocabulary!(ParameterType, [Bool, Choice, Integer, Path, String]);
vocabulary!(Phase, [Finish, Prepare]);
row!(Profile, [id, name, description, features, default]);
row!(PublicCommand, [name, launcher]);
row!(Requirement, [package, scope]);
vocabulary!(Result, [External, Owned]);
vocabulary!(Scope, [Build, Runtime]);
row!(SourceInput, [name, url, path, sha256, size, extract]);
row!(
    Step,
    [
        id,
        title,
        kind,
        description,
        phase,
        optional,
        hook,
        when,
        unless,
        needs,
        sources,
        format,
        source,
        sha256,
        size,
        payload_root,
        interpreter,
        script,
        command,
        force_install,
        update,
        uninstall,
        force_uninstall,
        arguments,
        force_arguments,
        from,
        to,
        prompt,
        prompt_file,
        mode,
        required,
        set,
        unset,
        path_prepend,
        requires
    ]
);

/// Whole application description with independent package and installer scopes.
/// Package data is retained for inspection, never applied by installer acquisition.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-069#APP-CONFIGURATION-JOURNEY"
)]
pub fn describe(resolved: &crate::ResolvedPackage) -> new::Description {
    new::Description {
        protocol: "vibe-configuration-description/1".into(),
        package: new::Package {
            group: resolved.package.group.clone(),
            name: resolved.package.name.clone(),
            version: resolved.package.version.clone(),
        },
        application: crate::describe::describe_application(resolved).promote(),
        configuration: resolved
            .configuration
            .as_ref()
            .map(super::definition::definition),
        options: resolved
            .options
            .iter()
            .map(|option| {
                let mut row: new::InstallerOption =
                    crate::describe::describe_option(resolved, option).promote();
                row.configuration = option
                    .configuration
                    .as_ref()
                    .map(super::definition::definition);
                row
            })
            .collect(),
    }
}
