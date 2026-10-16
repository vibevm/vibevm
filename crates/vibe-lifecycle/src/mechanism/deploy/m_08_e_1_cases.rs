//! Actual-client commissioning. These tests deliberately fail when the physical
//! namespace or release qualification is unavailable; they never silently skip.
pub(super) mod driver;
mod podbay_runner;

#[test]
fn actual_claude_2_1_299_required_matrix() {
    driver::commission("claude_code").unwrap();
}

#[test]
fn actual_codex_0_159_3_required_matrix() {
    driver::commission("codex").unwrap();
}

#[test]
fn current_version_profiles_preserve_legacy_and_wrong_version_refusal() {
    driver::version_contract().unwrap();
}

#[test]
fn receipt_owned_update_recovery_remove_preserves_foreign() {
    driver::receipt_contract().unwrap();
}

#[test]
fn same_core_binary_accepts_external_package_data() {
    driver::extensibility_contract().unwrap();
}

/// Shared file-owner evidence only; never invokes a vendor or Claude simulator.
#[test]
fn codex_scoped_file_recovery_preserves_foreign_without_vendor() {
    driver::codex_file_owner_contract().unwrap();
}

/// The external descriptor is introduced after hashing this running executable.
#[test]
fn external_package_data_uses_fixed_core_without_vendor() {
    driver::external_package_data_contract().unwrap();
}

#[test]
fn codex_plugin_recovery_requires_cache_and_selected_config() {
    driver::plugin_footprint_checks().unwrap();
}
