//! Structured headless debugger input; shared route owns every snapshot.
use anyhow::{Context, Result};
use clap::{Args, ValueEnum};
use std::{
    io::{BufRead, Read, Write},
    path::PathBuf,
    sync::Arc,
};
#[cfg(test)]
use vibe_orchestrator::document_analysis::{
    AnalysisLimits, DocumentAnalysisHost, ResourceDeclaration, ResourceRole,
};
use vibe_orchestrator::machine_operation::boot_debugger::{self, BootDebuggerRoute};
use vibe_workspace::boot_debugger::protocol::CapturedKind;
mod viewer;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ArtifactKind {
    Static,
    Index,
    Compiler,
}
#[derive(Debug, Args)]
pub struct BootArgs {
    /// Standalone line-oriented boot inspector (help lists its commands).
    #[arg(long)]
    pub view: bool,
    /// Drive the viewer headlessly with semicolon-separated commands; emit JSON frames.
    #[arg(long, requires = "view")]
    pub send: Option<String>,
    /// Explicit host root; request bodies cannot change it.
    #[arg(long)]
    pub root: PathBuf,
    /// Project-relative captured artifact, selected by the host at startup.
    #[arg(long, default_value = "STATIC.md")]
    pub artifact: String,
    #[arg(long, value_enum, default_value = "static")]
    pub kind: ArtifactKind,
    /// Admit only this local registry for explicit scenario capture.
    #[arg(long)]
    pub registry: Option<PathBuf>,
    /// Retain these project-relative derivative result records as observed history.
    #[arg(long = "derivative-record")]
    pub derivative_records: Vec<String>,
}
pub fn run(args: BootArgs) -> Result<()> {
    run_with_io(args, std::io::stdin().lock(), &mut std::io::stdout().lock())
}

/// Standard command composition, shared by terminal and headless fixtures.
fn run_with_io(args: BootArgs, input: impl BufRead, output: &mut impl Write) -> Result<()> {
    let route = open_route(&args)?;
    if args.view {
        viewer::serve(&route, input, output, args.send.as_deref())
    } else {
        serve(&route, input, output)
    }
}

fn open_route(args: &BootArgs) -> Result<Arc<BootDebuggerRoute>> {
    let root = args
        .root
        .canonicalize()
        .context("opening explicit debugger root")?;
    let kind = match args.kind {
        ArtifactKind::Static => CapturedKind::Static,
        ArtifactKind::Index => CapturedKind::Index,
        ArtifactKind::Compiler => CapturedKind::Compiler,
    };
    let route = vibe_package_source::InstallResolver::open_boot_debugger(
        &root,
        args.registry.as_deref(),
        &args.artifact,
        kind,
    )
    .map_err(|e| anyhow::anyhow!(e))?;
    let catalog = vibe_workspace::source_query::derivatives::DerivativeCatalog::load(
        &root,
        &args.derivative_records,
    )
    .map_err(|e| anyhow::anyhow!(e))?;
    route
        .configure_derivatives(catalog)
        .map_err(|e| anyhow::anyhow!(e.message))?;
    Ok(route)
}
/// First output is the host-assigned capture/context. Subsequent lines are
/// strict OperationEnvelope inputs and outputs, never CLI flag reparsing.
pub fn serve(
    route: &BootDebuggerRoute,
    mut input: impl BufRead,
    output: &mut impl Write,
) -> Result<()> {
    use vibe_wire::generated::boot_debugger::e1::request::{Command, CommandCapture, Request};
    let capture = boot_debugger::input(&Request {
        schema: 1,
        context_id: route.context_id().into(),
        command: Command::Capture(Box::new(CommandCapture {})),
    });
    let first = route.exchange(&serde_json::to_vec(&capture)?);
    serde_json::to_writer(&mut *output, &first)?;
    output.write_all(b"\n")?;
    output.flush()?;
    loop {
        let mut bytes = Vec::new();
        let read = input.by_ref().take(65_538).read_until(b'\n', &mut bytes)?;
        if read == 0 {
            break;
        }
        if bytes.len() > 65_537 {
            anyhow::bail!("debugger input line exceeds frame bound");
        }
        if bytes.last() == Some(&b'\n') {
            bytes.pop();
        }
        let reply = route.exchange(&bytes);
        serde_json::to_writer(&mut *output, &reply)?;
        output.write_all(b"\n")?;
        output.flush()?;
    }
    Ok(())
}
#[cfg(test)]
mod m_06_c_1_cases {
    use super::*;
    #[test]
    fn structured_input_emits_canonical_error_and_initial_capture() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("vibe.toml"), "[project]\n").unwrap();
        std::fs::write(root.path().join("INDEX.md"), "").unwrap();
        let host = DocumentAnalysisHost::new(
            root.path(),
            "vibe.toml",
            vec![
                ResourceDeclaration {
                    path: "vibe.toml".into(),
                    role: ResourceRole::Manifest,
                },
                ResourceDeclaration {
                    path: "INDEX.md".into(),
                    role: ResourceRole::Opaque,
                },
            ],
            AnalysisLimits {
                resource_bytes: 1024,
                capture_bytes: 4096,
                open_buffers: 2,
            },
        )
        .unwrap();
        let artifact = host.resource("INDEX.md").unwrap();
        let route = BootDebuggerRoute::new(host, artifact, CapturedKind::Index).unwrap();
        let mut output = Vec::new();
        use vibe_wire::generated::boot_debugger::e1::request::{Command, CommandCapture, Request};
        let request = boot_debugger::input(&Request {
            schema: 1,
            context_id: route.context_id().into(),
            command: Command::Capture(Box::new(CommandCapture {})),
        });
        let mut input = serde_json::to_vec(&request).unwrap();
        input.extend_from_slice(b"\n{\"foreign\":true}\n");
        serve(&route, std::io::Cursor::new(input), &mut output).unwrap();
        let values: Vec<vibe_wire::generated::machine::e1::operation_envelope::OperationEnvelope> =
            output
                .split(|b| *b == b'\n')
                .filter(|b| !b.is_empty())
                .map(|b| serde_json::from_slice(b).unwrap())
                .collect();
        assert_eq!(values.len(), 3);
        use vibe_wire::generated::machine::e1::operation_envelope::Body;
        assert!(matches!(values[0].body, Body::Result(_)));
        assert!(matches!(values[1].body, Body::Result(_)));
        assert!(matches!(values[2].body, Body::Error(_)));
        assert!(!root.path().join(".vibe").exists());
    }
}

#[cfg(test)]
#[path = "m_06_c_1_cases.rs"]
mod cross_surface;

#[cfg(test)]
mod m_06_e_2_cases;

#[cfg(test)]
mod m_10_d_1_cases;

#[cfg(test)]
mod m_06_f_1_cases;
