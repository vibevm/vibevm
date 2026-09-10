# Rust-stack-specific notes for integration-alpha

Rust-flavoured trace-id and error-handling conventions on top of the
canonical integration-alpha protocol. Activated via `if_present =
["stack:integration-rust"]`.

Currently delivered eagerly because `vibe-mcp` (M1.7) hasn't shipped
yet — `delivery = "lazy-push"` is honoured in the lockfile but
materialisation falls through to eager with a tracing warning.
