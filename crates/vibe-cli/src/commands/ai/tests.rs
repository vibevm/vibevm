//! The ai views at a fixed width (PROP-067 `#list`, `#block`).

use super::view;
use vibe_connections::{
    BindingObservation, Connection, DefaultSelection, Metadata, MetadataCapability, Snapshot,
};
use vibe_secrets::SecretId;

const C: &str = "91d2af49-00b3-42d4-a589-56f2e4478fd2";
const K: &str = "2bf3accb-631e-4901-8e8b-d367b0da7861";
const R: &str = "3a2aabf3-6c1d-4e2b-9f3a-0b1c2d3e4f50";

fn id(raw: &str) -> SecretId {
    SecretId::try_from(raw.to_owned()).unwrap()
}

fn snapshot() -> Snapshot {
    Snapshot {
        store_revision: 1,
        items: [(
            id(C),
            Connection {
                current_revision: id(R),
                secret_ref: id(K),
                metadata: Metadata {
                    label: "x".into(),
                    endpoint: "https://api.example.invalid".into(),
                    api_kind: "openai-chat-completions".into(),
                    default_model: "m".into(),
                },
            },
        )]
        .into_iter()
        .collect(),
    }
}

#[test]
fn ai_view_connections_are_rows_under_a_rule() {
    assert_eq!(
        view::connections_at(&snapshot(), 80, 80),
        format!(
            "── ai connections · store revision 1 {}\n\n\
             \x20 {C}  x  (openai-chat-completions)  model m\n\
             \x20 └ endpoint https://api.example.invalid\n\
             \x20 └ key {K}\n\
             \x20 └ revision {R}",
            "─".repeat(43)
        )
    );
    let empty = Snapshot {
        store_revision: 0,
        items: Default::default(),
    };
    assert_eq!(
        view::connections_at(&empty, 80, 80),
        format!(
            "── ai connections · store revision 0 {}\n\n\
             \x20 No AI connections. Add one with\n\
             \x20 vibe ai connections add --label <label> --endpoint <url> --api-kind <kind> \
             --default-model <model> --secret-ref <keyring-id>",
            "─".repeat(43)
        )
    );
    assert_eq!(
        view::connections_count(1, 1),
        "1 AI connection · store revision 1"
    );
}

#[test]
fn ai_view_default_connection_is_one_block() {
    let set = DefaultSelection {
        store_revision: 2,
        connection: Some(id(C)),
    };
    assert_eq!(
        view::default_block(&set, Some("x")),
        format!("✓ default connection  x · {C} · store revision 2")
    );
    let none = DefaultSelection {
        store_revision: 3,
        connection: None,
    };
    assert_eq!(
        view::default_block(&none, None),
        "○ no default connection · store revision 3"
    );
    assert_eq!(
        view::default_quiet(&none, None),
        "no default connection · store revision 3"
    );
}

#[test]
fn ai_view_binding_is_one_block_with_its_evidence() {
    let bound = BindingObservation {
        harness: "qwen".into(),
        store_revision: 4,
        mode: "connection".into(),
        connection: Some(id(C)),
        resolution: "metadata-only".into(),
        capability: MetadataCapability::Compatible {
            protocol: "openai-chat-completions",
            scope: "stored-selection-only",
            evidence: "Qwen 0.27.2 Linux proxy fixture",
        },
    };
    assert_eq!(
        view::binding_block(&bound),
        format!(
            "✓ qwen  connection · metadata-only · store revision 4  [compatible]\n\
             \x20 connection   {C}\n\
             \x20 protocol     openai-chat-completions\n\
             \x20 scope        stored-selection-only\n\
             \x20 evidence     Qwen 0.27.2 Linux proxy fixture"
        )
    );
    let native = BindingObservation {
        harness: "claude".into(),
        store_revision: 4,
        mode: "native".into(),
        connection: None,
        resolution: "unmanaged".into(),
        capability: MetadataCapability::Unsupported,
    };
    assert_eq!(
        view::binding_block(&native),
        "✓ claude  native · unmanaged · store revision 4  [unsupported]"
    );
    assert_eq!(
        view::binding_quiet(&bound),
        "qwen connection · metadata-only · store revision 4"
    );
}
