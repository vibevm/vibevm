//! The keyring views at a fixed width (PROP-067 `#list`, `#heading`).

use super::human;

const A: &str = "2bf3accb-631e-4901-8e8b-d367b0da7861";
const RA: &str = "810b4901-4e06-49f0-a10e-021c53825291";
const B: &str = "cc0ec34e-5ec4-4875-8fc0-5a7aeb6a54b8";
const RB: &str = "9c4a8089-5b1e-4d7a-9a52-3f1d2e6b7c80";

fn rows() -> Vec<human::ItemRow> {
    vec![
        (A.into(), "third".into(), "api-key".into(), RA.into()),
        (B.into(), "demo".into(), "token".into(), RB.into()),
    ]
}

#[test]
fn secrets_view_list_never_cuts_identifiers() {
    assert_eq!(
        human::items_at(5, &rows(), 80, 80),
        format!(
            "── local keyring · store revision 5 {}\n\n\
             \x20 {A}  third  (api-key)  [value hidden]\n\
             \x20 └ revision {RA}\n\
             \x20 {B}  demo   (token)    [value hidden]\n\
             \x20 └ revision {RB}",
            "─".repeat(44)
        )
    );
    let narrow = human::items_at(5, &rows(), 40, 40);
    for id in [A, RA, B, RB] {
        assert!(narrow.contains(id), "{narrow}");
    }
}

#[test]
fn secrets_view_empty_keyring_names_the_create_command() {
    assert_eq!(
        human::items_at(0, &[], 80, 80),
        format!(
            "── local keyring · store revision 0 {}\n\n\
             \x20 The local keyring is empty. Add an item with\n\
             \x20 vibe secrets create --kind <kind> --label <label> --value-stdin",
            "─".repeat(44)
        )
    );
}

#[test]
fn secrets_view_references_stand_under_a_rule() {
    assert_eq!(
        human::references_at(A, &["ai:91d2af49-00b3-42d4-a589-56f2e4478fd2".into()], 80),
        format!(
            "── consumers of {A} {}\n\n  ai:91d2af49-00b3-42d4-a589-56f2e4478fd2",
            "─".repeat(27)
        )
    );
    assert_eq!(
        human::references_at(A, &[], 80),
        format!(
            "── consumers of {A} {}\n\n  No registered consumer references this item.",
            "─".repeat(27)
        )
    );
}

#[test]
fn secrets_view_quiet_lines_count() {
    assert_eq!(
        human::count_line(2, 5),
        "2 keyring items · store revision 5"
    );
    assert_eq!(human::count_line(1, 1), "1 keyring item · store revision 1");
    assert_eq!(
        human::consumers_line(A, 0),
        format!("no consumer references {A}")
    );
    assert_eq!(
        human::consumers_line(A, 2),
        format!("2 consumers reference {A}")
    );
}
