//! The human views of `vibe ai connections`, `default` and `bindings`: a
//! rule with measured rows, or one block (PROP-067), never JSON.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-067#coverage");

use crate::output::language::{
    Block, Cell, Glyph, LabelValue, Line, List, Meta, Role, Row, paint, rule_at, rule_width,
    viewport,
};
use vibe_connections::{BindingObservation, DefaultSelection, MetadataCapability, Snapshot};

/// The command the empty listing names.
const ADD: &str = "vibe ai connections add --label <label> --endpoint <url> --api-kind <kind> \
                   --default-model <model> --secret-ref <keyring-id>";

/// No width bound for a block whose metadata holds an identifier: the
/// identifier is never dropped (PROP-067 `#width`).
const UNBOUNDED: usize = usize::MAX / 4;

/// The connections at this terminal's width.
pub(super) fn connections(snapshot: &Snapshot) -> String {
    connections_at(snapshot, viewport(), rule_width())
}

/// The connections: a rule naming the store revision, then one row per
/// connection (id, label, API kind, model) with its endpoint, keyring item
/// and revision as children, so no identifier is ever cut.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#list")]
pub(super) fn connections_at(snapshot: &Snapshot, width: usize, rule: usize) -> String {
    let heading = rule_at(
        &format!(
            "ai connections · store revision {}",
            snapshot.store_revision
        ),
        None,
        rule,
    );
    if snapshot.items.is_empty() {
        return format!(
            "{heading}\n\n  No AI connections. Add one with\n  {}",
            paint(Role::Accent, ADD)
        );
    }
    let rows = snapshot
        .items
        .iter()
        .map(|(id, connection)| {
            let metadata = &connection.metadata;
            Row::new(vec![
                Cell::text(id.as_str()),
                Cell::text(metadata.label.clone()),
                Cell::new(format!("({})", metadata.api_kind), Role::Dim),
                Cell::new(format!("model {}", metadata.default_model), Role::Dim),
            ])
            .child(format!("endpoint {}", metadata.endpoint))
            .child(format!("key {}", connection.secret_ref.as_str()))
            .child(format!("revision {}", connection.current_revision.as_str()))
        })
        .collect();
    format!(
        "{heading}\n\n{}",
        List::new(rows).children_under(0).render_at(width)
    )
}

/// The quiet line of a listing: `1 AI connection · store revision 1`.
pub(super) fn connections_count(items: usize, store_revision: u64) -> String {
    let noun = if items == 1 {
        "AI connection"
    } else {
        "AI connections"
    };
    format!("{items} {noun} · store revision {store_revision}")
}

/// The default connection as one block, or the block that there is none;
/// `label` is the default connection's own label when it can be read.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#block")]
pub(super) fn default_block(selection: &DefaultSelection, label: Option<&str>) -> String {
    let revision = Meta::note(format!("store revision {}", selection.store_revision));
    let block = match (&selection.connection, label) {
        (Some(id), Some(label)) => Block::new(Glyph::Done, format!("default connection  {label}"))
            .with_meta(vec![Meta::note(id.as_str()), revision]),
        (Some(id), None) => Block::new(Glyph::Done, format!("default connection  {}", id.as_str()))
            .with_meta(vec![revision]),
        (None, _) => Block::new(Glyph::Skipped, "no default connection").with_meta(vec![revision]),
    };
    block.render_at(UNBOUNDED)
}

/// The quiet line of the default connection.
pub(super) fn default_quiet(selection: &DefaultSelection, label: Option<&str>) -> String {
    let revision = selection.store_revision;
    match (&selection.connection, label) {
        (Some(id), Some(label)) => {
            format!(
                "default connection {label} · {} · store revision {revision}",
                id.as_str()
            )
        }
        (Some(id), None) => format!(
            "default connection {} · store revision {revision}",
            id.as_str()
        ),
        (None, _) => format!("no default connection · store revision {revision}"),
    }
}

/// One agent's binding as a block: its mode, resolution and store revision,
/// the capability as a state word, then the connection and the capability's
/// evidence as label lines.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#block")]
pub(super) fn binding_block(binding: &BindingObservation) -> String {
    let glyph = if binding.resolution.starts_with("unavailable") {
        Glyph::Skipped
    } else {
        Glyph::Done
    };
    let state = match binding.capability {
        MetadataCapability::Compatible { .. } => "compatible",
        MetadataCapability::Unsupported => "unsupported",
        MetadataCapability::Unknown => "unknown",
    };
    let mut body = Vec::new();
    if let Some(connection) = &binding.connection {
        body.push(label("connection", connection.as_str()));
    }
    if let MetadataCapability::Compatible {
        protocol,
        scope,
        evidence,
    } = &binding.capability
    {
        body.push(label("protocol", protocol));
        body.push(label("scope", scope));
        body.push(label("evidence", evidence));
    }
    Block::new(glyph, format!("{}  {}", binding.harness, binding.mode))
        .with_meta(vec![
            Meta::note(binding.resolution.clone()),
            Meta::note(format!("store revision {}", binding.store_revision)),
            Meta::State(state.into()),
        ])
        .with_body(body)
        .render_at(UNBOUNDED)
}

/// The quiet line of a binding: `qwen connection · metadata-only · store revision 4`.
pub(super) fn binding_quiet(binding: &BindingObservation) -> String {
    format!(
        "{} {} · {} · store revision {}",
        binding.harness, binding.mode, binding.resolution, binding.store_revision
    )
}

/// A label line of a block body.
fn label(name: &str, value: &str) -> Line {
    Line::Label(name.into(), vec![LabelValue::Text(value.into())])
}
