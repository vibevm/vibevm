//! Human rendering of keyring metadata; value bytes never reach it.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-000#TS-USER-KEYRING");

pub(super) fn items(store_revision: u64, rows: &[(String, String, String, String)]) -> String {
    if rows.is_empty() {
        return "Local keyring is empty.".into();
    }
    let mut table = crate::output::language::Table::new(vec![
        "ID".into(),
        "Label".into(),
        "Kind".into(),
        "Revision".into(),
        "Value".into(),
    ]);
    for (id, label, kind, revision) in rows {
        table = table.row(vec![
            id.clone(),
            label.clone(),
            kind.clone(),
            revision.clone(),
            "stored (hidden)".into(),
        ]);
    }
    format!(
        "Local keyring — store revision {store_revision}\n{}",
        table.render()
    )
}

pub(super) fn references(id: &str, consumers: &[String]) -> String {
    if consumers.is_empty() {
        return format!("No registered consumers reference {id}.");
    }
    let mut lines = vec![format!("Consumers of {id}:")];
    lines.extend(consumers.iter().map(|consumer| format!("  {consumer}")));
    lines.join("\n")
}
