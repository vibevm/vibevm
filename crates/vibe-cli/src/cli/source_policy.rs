//! Invocation-only source choices; persisted declarations remain in their original owner.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#SOURCE-POLICY-LAW");

#[derive(Debug, Clone, Default, clap::Args)]
pub struct SourcePolicyArgs {
    /// Prefer hosting for admitted candidates of the same version. Repeat in priority order.
    #[arg(
        long,
        value_name = "github|gitverse|host:HOST",
        conflicts_with = "no_hosting_preference"
    )]
    pub prefer_hosting: Vec<vibe_core::manifest::HostingSelector>,
    /// Clear inherited hosting preferences for this operation.
    #[arg(long)]
    pub no_hosting_preference: bool,
    /// Exclude actual managed contact hosts; adds to project and user exclusions.
    #[arg(long, value_name = "github|gitverse|host:HOST")]
    pub exclude_hosting: Vec<vibe_core::manifest::HostingSelector>,
}

impl SourcePolicyArgs {
    pub(crate) fn declaration(&self) -> vibe_core::manifest::SourcePolicyDecl {
        vibe_core::manifest::SourcePolicyDecl {
            prefer_hosting: if self.no_hosting_preference || !self.prefer_hosting.is_empty() {
                Some(self.prefer_hosting.clone())
            } else {
                None
            },
            exclude_hosting: self.exclude_hosting.clone(),
        }
    }
}
