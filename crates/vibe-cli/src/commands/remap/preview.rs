//! Read-only request-local data from a verified local owned offer, not supplier provenance.
use super::home::RemapHome;
use crate::output::language::{Cell, List, Role, Row, rule_at, rule_width, viewport};
use crate::{cli::RemapPreviewArgs, output};
use anyhow::{Result, anyhow, ensure};
use vibe_core::manifest::Manifest;
use vibe_wire::generated::remap_preview_result::{RemapPreviewResult, Source, SourceKind};

pub(super) fn run(ctx: &output::Context, args: RemapPreviewArgs, offline: bool) -> Result<()> {
    ensure!(offline, "{}", offline_refusal(&args.package, &args.input));
    let home = RemapHome::open()?;
    let key = args.package.strip_prefix("remap:").unwrap_or(&args.package);
    let reference = vibe_core::PackageRef::parse(key)?;
    ensure!(
        reference.group.is_some() && reference.qualified_name() == key,
        "remap preview requires one explicit unversioned qualified offer key"
    );
    let entries: Vec<_> = home
        .installed
        .packages
        .iter()
        .filter(|e| e.package.key == key)
        .collect();
    ensure!(
        entries.len() == 1,
        "remap preview requires exactly one local offer"
    );
    let entry = entries[0];
    let scope = vibe_safefs::Project::open(home.root())?;
    let index = scope
        .read_file_snapshot_bounded("remap/installed.json", 1024 * 1024)?
        .ok_or_else(|| anyhow!("local offer index missing"))?;
    let captured_index: vibe_core::remap::InstalledRemaps =
        serde_json::from_slice(&index.bytes).map_err(|_| anyhow!("local offer index invalid"))?;
    ensure!(
        captured_index == home.installed,
        "local offer index changed during selection"
    );
    let tree = super::packages::tree_of(&home, key);
    let proof =
        vibe_registry::local_preview::verify_pinned_payload(&tree, &entry.package.content_hash)?;
    let manifest = Manifest::parse_str(std::str::from_utf8(proof.manifest_bytes()?)?)?;
    ensure!(
        manifest.require_package()?.kind == vibe_core::PackageKind::Remap,
        "local offer kind mismatch"
    );
    let observed = vibe_core::remap::offer_from_manifest(&manifest, &entry.package.content_hash)
        .map_err(|_| anyhow!("local offer manifest invalid"))?;
    ensure!(
        observed == entry.package,
        "local offer manifest/rows/policy differ from recorded offer"
    );
    let path = std::path::absolute(&args.input)?;
    let parent = path
        .parent()
        .ok_or_else(|| anyhow!("preview input parent missing"))?;
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow!("preview input filename invalid"))?;
    let input_scope = vibe_safefs::Project::open(parent)?;
    let bytes = input_scope
        .read_file_snapshot_bounded(name, 64 * 1024)?
        .ok_or_else(|| anyhow!("preview input unavailable"))?;
    let input = vibe_wire::behaviour::remap_preview::parse(&bytes.bytes)?;
    let selections = input
        .selections
        .into_iter()
        .map(|(id, answer)| (id, answer.members))
        .collect();
    let values = vibe_core::manifest::evaluate_request_local_data(&manifest, &selections)
        .map_err(|reason| anyhow!(reason))?;
    proof.recheck()?;
    ensure!(
        scope
            .read_file_snapshot_bounded("remap/installed.json", 1024 * 1024)?
            .as_ref()
            == Some(&index),
        "local offer index changed during preview"
    );
    let result = RemapPreviewResult {
        protocol: "vibe-remap-preview/1".into(),
        package: key.into(),
        version: entry.package.version.clone(),
        source: Source {
            kind: SourceKind::LocalOwnedOffer,
            content_hash: entry.package.content_hash.clone(),
        },
        selected_values: values
            .into_iter()
            .map(|(id, value)| Ok((id, Some(serde_json::to_value(value)?))))
            .collect::<Result<_>>()?,
    };
    if args.json || ctx.is_json() {
        ctx.emit_json(&result)
    } else {
        let text = view(&result, viewport(), rule_width());
        if ctx.is_quiet() {
            ctx.summary(&text);
        } else {
            ctx.block(&text);
        }
        Ok(())
    }
}

/// Keep retry argv literal and never embed controls or backtick delimiters in a fix.
pub(super) fn offline_refusal(package: &str, input: &std::path::Path) -> String {
    let input = input.to_string_lossy();
    let unsafe_text = |text: &str| text.chars().any(|c| c.is_control() || c == '`');
    let command = if unsafe_text(package) || unsafe_text(&input) {
        "vibe remap preview PACKAGE --input INPUT --offline".to_owned()
    } else {
        let token = |text: &str| {
            if !text.is_empty()
                && text.bytes().all(|b| b.is_ascii_alphanumeric() || b"_-./:".contains(&b))
            {
                text.to_owned()
            } else {
                format!("'{}'", text.replace('\'', "'\\''"))
            }
        };
        format!("vibe remap preview {} --input {} --offline", token(package), token(&input))
    };
    format!(
        "offline_required: remap preview reads only a verified local offer (violates \
         spec://org.vibevm.core/vibevm/common/PROP-069#AUTHORED-DEFINITIONS; fix: `{command}`)"
    )
}

/// Render verified local offer data as human rows without changing its wire document.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#list")]
pub(super) fn view(result: &RemapPreviewResult, width: usize, rule: usize) -> String {
    let mut rows = vec![Row::new(vec![
        Cell::text("content"),
        Cell::text(&result.source.content_hash),
    ])];
    for (id, value) in &result.selected_values {
        let value = match value {
            Some(serde_json::Value::String(text)) => Cell::text(text),
            Some(value) => Cell::text(value.to_string()),
            None => Cell::new("none", Role::Dim),
        };
        rows.push(Row::new(vec![Cell::text(id), value]));
    }
    format!(
        "{}\n{}",
        rule_at(
            &format!("preview of {}@{}", result.package, result.version),
            Some("local owned offer"),
            rule,
        ),
        List::new(rows).render_at(width)
    )
}
