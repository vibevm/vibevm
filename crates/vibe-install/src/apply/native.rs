use super::*;

type LoweringFactory<'a> = &'a mut dyn FnMut(
    &Workspace,
    &[ResolvedDep],
) -> std::result::Result<
    (
        vibe_workspace::extension_world::ExtensionWorldEpoch,
        vibe_workspace::extension_world::OwnerRuntimeLowering,
    ),
    String,
>;

pub(super) struct NativeApplyPreparation<'a> {
    pub(super) prepare: LoweringFactory<'a>,
    pub(super) run: vibe_workspace::extension_world::OwnerRuntimeRunFacts,
    pub(super) platform: vibe_lifecycle::native::NativePlatform,
}

impl PreparedApplyReport {
    /// Native-aware production sibling without widening the compatibility
    /// free-function surface. The caller supplies only owner-preset lowering;
    /// provider construction and replay identity stay below this boundary.
    #[allow(clippy::too_many_arguments)]
    pub fn apply_native<S, F>(
        source: &S,
        planned: PlannedInstall,
        slot_integrity: SlotIntegrity,
        spec_format: SpecFormat,
        run: RunMetadata,
        streams: StreamMode,
        seams: SlotLifecycleSeams,
        lease: std::sync::Arc<LifecycleLease>,
        trace: Option<&vibe_workspace::compile_trace::TraceRun>,
        prepare: &mut F,
        runtime_run: vibe_workspace::extension_world::OwnerRuntimeRunFacts,
        platform: vibe_lifecycle::native::NativePlatform,
    ) -> Result<Self>
    where
        S: InstallSource + ?Sized,
        F: FnMut(
            &Workspace,
            &[ResolvedDep],
        ) -> std::result::Result<
            (
                vibe_workspace::extension_world::ExtensionWorldEpoch,
                vibe_workspace::extension_world::OwnerRuntimeLowering,
            ),
            String,
        >,
    {
        apply_with_spec_format_and_lifecycle_observed_traced_prepared_inner(
            source,
            planned,
            slot_integrity,
            spec_format,
            run,
            streams,
            seams,
            lease,
            trace,
            Some(NativeApplyPreparation {
                prepare,
                run: runtime_run,
                platform,
            }),
        )
    }
}
