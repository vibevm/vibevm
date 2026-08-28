//! The hosted MCP lifecycle surface, driven through its real tool and JSON-RPC
//! dispatcher. Lower runner/state tests own the engine; these cells prove the
//! strict adapter, ports, report funnel and cross-call continuity.

use vibe_test_support as _;

#[path = "lifecycle_run/dependency.rs"]
mod dependency;
#[path = "lifecycle_run/grammar.rs"]
mod grammar;
#[path = "lifecycle_run/hosted.rs"]
mod hosted;
#[path = "lifecycle_run/outcomes.rs"]
mod outcomes;
#[path = "lifecycle_run/support.rs"]
mod support;
#[path = "lifecycle_run/transport.rs"]
mod transport;
