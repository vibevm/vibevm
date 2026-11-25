//! The display text of the configuration refusals, which the CLI reads
//! as `<code>: <headline> (violates <spec>; fix: <fix>): <why>`.

use super::PackageConfigurationError;

#[test]
fn unsupported_configuration_reads_as_a_coded_refusal() {
    let error = PackageConfigurationError::Unsupported {
        package: "org.vibevm.qwen/qwen-code".into(),
        reason: "ambient source layer is present or cannot be proven absent".into(),
    };
    assert_eq!(
        error.to_string(),
        "configuration_unsupported: configuration of package org.vibevm.qwen/qwen-code is not \
         supported by this operation (violates \
         spec://org.vibevm.core/vibevm/common/PROP-069#EFFECT-CONSUMERS; fix: use an operation \
         that supports this configuration input): ambient source layer is present or cannot be \
         proven absent"
    );
}
