//! Exact retained continuation; no public input file or original argv is replayed.
use super::{
    agents::{AgentSetup, report_parked},
    display::StepDisplay,
    global::{self, Observation},
    report,
};
use crate::output;
use anyhow::Result;
use vibe_app::{OperationContext, ResumeRequest, SettingsRoot};

pub(super) fn run(
    ctx: &output::Context,
    home: &SettingsRoot,
    key: &str,
    operation: &str,
    parameters: Vec<(String, String)>,
    offline: bool,
    source_operation: &vibe_package_source::SourceOperation,
) -> Result<Option<i32>> {
    let info = vibe_app::resume_info(home, key, operation)?;
    let agents = AgentSetup::for_resume(ctx, &info)?;
    let packages = global::Packages {
        operation: source_operation.clone(),
        settings: global::settings_root()?,
        registry: match &info.lookup {
            vibe_wire::generated::shared::ApplicationPendingLookup::Registry(r) => {
                Some(r.root.clone().into())
            }
            _ => None,
        },
        embedded: match &info.lookup {
            vibe_wire::generated::shared::ApplicationPendingLookup::Chain(r) => {
                r.embedded.clone().map(Into::into)
            }
            _ => None,
        },
        offline: offline || info.offline,
    };
    let resolver = |name: &str| match &info.lookup {
        vibe_wire::generated::shared::ApplicationPendingLookup::Project(p) => {
            super::project::resolve_saved_requirement(std::path::Path::new(&p.root), name)
        }
        vibe_wire::generated::shared::ApplicationPendingLookup::Caller(_) => {
            Err(vibe_app::ResolveError::Registry {
                spelling: name.into(),
                reason: "this saved caller-owned resolver cannot be reconstructed by the CLI"
                    .into(),
            }
            .into())
        }
        _ => packages.resolve(name),
    };
    let proxy = global::proxy_image()?;
    let flat = StepDisplay::flat();
    let observe = Observation::new(ctx, true, &flat)?.with_source_operation(&packages.operation);
    let show = global::plan_printer(ctx);
    let request = ResumeRequest {
        key: key.into(),
        operation: operation.into(),
        parameters,
    };
    let outcome = flat.around(ctx, || {
        let operation = OperationContext {
            agent: agents.hooks(),
            resolver: Some(&resolver),
            resolver_scope: match &info.lookup {
                vibe_wire::generated::shared::ApplicationPendingLookup::Project(p) => {
                    vibe_app::ResolverScope::Project(std::path::Path::new(&p.root))
                }
                vibe_wire::generated::shared::ApplicationPendingLookup::Caller(_) => {
                    vibe_app::ResolverScope::Caller
                }
                _ => packages.scope(),
            },
            ..global::operation(
                ctx,
                home,
                offline || info.offline,
                &proxy,
                Some(&show),
                &observe,
            )
        };
        report_parked(ctx, vibe_app::resume(&operation, &request))
    });
    let outcome = flat.report(ctx, outcome)?;
    report::render(ctx, &outcome)?;
    Ok(None)
}
