//! Owned host adapters for the real full-pass/hosted-runner boundary.
use crate::ports::*;
use crate::*;
use std::{
    path::Path,
    sync::{Arc, Mutex},
};
use vibe_lifecycle::{
    RunMetadata,
    agent::{AgentBackend, AgentCompletion, AgentRequest, PromptRequest, ResolvedPrompt},
};
use vibe_wire::generated::lifecycle::e1::context::RunAgentMode;
use vibe_wire::generated::lifecycle_report::LifecycleContributionReport;

#[derive(Default)]
pub(super) struct Host {
    pub resolved: Mutex<Vec<PromptRequest>>,
    pub paid: Mutex<Vec<AgentRequest>>,
}
impl AgentBackend for Host {
    fn resolve_prompt(&self, request: &PromptRequest) -> Result<ResolvedPrompt, String> {
        self.resolved.lock().unwrap().push(request.clone());
        Ok(ResolvedPrompt {
            text: "Produce exactly the selected work-a fixture, then stop.".into(),
            unsupported: vec![],
        })
    }
    fn complete(&self, request: &AgentRequest) -> Result<AgentCompletion, String> {
        self.paid.lock().unwrap().push(request.clone());
        Err("paid provider must never be entered by this hosted fixture".into())
    }
}
struct Quiet;
impl RunObserver for Quiet {
    fn stream_mode(&self) -> vibe_lifecycle::process::StreamMode {
        vibe_lifecycle::process::StreamMode::Capture
    }
    fn binary_quiet(&self) -> bool {
        true
    }
    fn emit_machine_failure(&self) -> bool {
        false
    }
    fn observe_plan(&self, _: &RitualPlan, _: &RunMetadata, _: bool) -> anyhow::Result<()> {
        Ok(())
    }
    fn observe_contribution(&self, _: &LifecycleContributionReport) {}
    fn observe_untracked_failure(
        &self,
        _: &RunMetadata,
        _: &str,
        _: &[LifecycleContributionReport],
    ) -> anyhow::Result<()> {
        Ok(())
    }
}
impl InstallObserver for Quiet {
    fn stream_mode(&self) -> vibe_lifecycle::process::StreamMode {
        vibe_lifecycle::process::StreamMode::Capture
    }
    fn emit_machine_failure(&self) -> bool {
        false
    }
    fn narrate(&self, _: InstallNarration<'_>) {}
    fn lane_sizes(&self, _: &Path) -> Vec<(String, Option<u64>)> {
        vec![]
    }
    fn plan_events(&self) -> &dyn vibe_install::PlanObserver {
        self
    }
    fn slot_observer(&self, _: &RunMetadata) -> Arc<dyn vibe_install::SlotLifecycleObserver> {
        Arc::new(Quiet)
    }
}
impl vibe_install::PlanObserver for Quiet {
    fn on(&self, _: vibe_install::PlanEvent) {}
}
impl vibe_install::SlotLifecycleObserver for Quiet {
    fn observe(&self, _: &vibe_install::SlotLifecyclePlan) -> Result<(), String> {
        Ok(())
    }
    fn outcome(&self, _: &vibe_install::SlotLifecycleReport) -> Result<(), String> {
        Ok(())
    }
}
impl ConfirmGate for Quiet {
    fn confirm_install(&self, _: usize) -> anyhow::Result<()> {
        Ok(())
    }
}
impl PackageSourceFactory for Quiet {
    fn build(&self, _: PackageSourceBuild<'_>) -> anyhow::Result<Box<dyn PackageSource>> {
        anyhow::bail!("source preparation forbidden in package-free fixture")
    }
}
impl RegistryEnvironment for Quiet {
    fn prepare(&self) -> anyhow::Result<RegistryEnvironmentSnapshot> {
        Ok(RegistryEnvironmentSnapshot {
            embedded_root: None,
            global: Default::default(),
        })
    }
}
pub(super) fn run(root: &Path, host: Arc<Host>) -> (PhaseOutcome, RunMetadata) {
    let prepared = prepare_full_pass_lifecycle(
        lease_default_lifecycle(root).unwrap(),
        DefaultLifecycleRequest {
            requested: vibe_lifecycle::Phase::Verify,
            force: false,
            agent_mode: RunAgentMode::Agent,
            assume_yes: true,
            trace_flag: false,
            install_inputs: Default::default(),
            policy: Default::default(),
        },
    )
    .unwrap();
    let metadata = prepared.metadata().clone();
    let outcome = prepared.run(
        DefaultLifecyclePorts {
            observer: &Quiet,
            install_observer: &Quiet,
            confirm_gate: &Quiet,
            sources: &Quiet,
            environment: &Quiet,
            manifest_mutation: &NoManifestMutation,
            agent: host,
        },
        None,
        vibe_core::manifest::TargetOs::Linux,
        "2026-10-13T13:40:00Z".parse().unwrap(),
        None,
    );
    (outcome, metadata)
}
pub(super) const SOURCE: &[u8] =
    b"pub fn answer()->u32{42}\n#[test] fn value(){assert_eq!(answer(),42);}\n";
pub(super) fn declarations(root: &Path, id: &str) {
    let manifest = root.join("vibe.toml");
    let mut text = std::fs::read_to_string(&manifest).unwrap();
    text.push_str(&format!("\n[[extension]]\nid='produce'\npoint='phase:create'\neffects=['product-source']\ninputs=['docs/custom/changes/{id}/intent.md']\nhandler={{kind='agent',prompt='spec://org.demo/consumer/PROMPT#root'}}\n[extension.config]\noutputs=[{{path='src/product.rs',kind='file',accept='non-empty file'}}]\n"));
    for (id, phase, effect) in [
        ("compile", "build", "build-artifact"),
        ("observe", "verify", "evidence-observer"),
    ] {
        text.push_str(&format!("\n[[extension]]\nid='{id}'\npoint='phase:{phase}'\neffects=['{effect}']\ninputs=['src/product.rs']\nhandler={{kind='script',base='scripts/operation'}}\n"));
    }
    std::fs::write(manifest, text).unwrap();
    std::fs::create_dir_all(root.join("scripts")).unwrap();
    std::fs::write(root.join("scripts/operation.sh"),r#"#!/bin/sh
python3 - <<'PY'
import os,json,pathlib,subprocess
c=json.load(open(os.environ['VIBE_CONTEXT']));root=pathlib.Path(c['project']['root']);op=c['execution']['id']
with (root/'calls').open('a') as f:f.write(op+'\n')
if op=='compile':
 (root/'target').mkdir(exist_ok=True);out=root/'target/check';subprocess.run(['rustc','--edition=2024','--test',str(root/'src/product.rs'),'-o',str(out)],check=True,env=dict(os.environ,RUSTUP_AUTO_INSTALL='0',CARGO_NET_OFFLINE='true'));ident='compiled'
elif op=='observe':
 result=subprocess.run([str(root/'target/check')],check=True,capture_output=True);(root/'evidence').mkdir(exist_ok=True);out=root/'evidence/current-source';out.write_bytes((root/'src/product.rs').read_bytes());(root/'evidence/stdout').write_bytes(result.stdout);ident='evidence'
else:raise RuntimeError('unexpected operation')
fail=(root/'FAIL-VERIFY').exists() and op=='observe'
json.dump({'envelope':1,'status':'fail' if fail else 'ok','message':'selected verification failure' if fail else None,'tasks':[],'artifacts':[] if fail else [{'id':ident,'kind':'file','path':str(out)}]},open(os.environ['VIBE_REPLY'],'w'))
PY
"#).unwrap();
}
