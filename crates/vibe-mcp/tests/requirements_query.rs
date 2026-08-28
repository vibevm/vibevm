//! The `requirements_query` surface, driven through the concrete cell, the
//! REAL dispatcher and the REAL production stdio transport. The library owns
//! the answer and the lib-side cell owns the argument law; these cells own
//! what only a whole server can show: that the cell's answer IS the generated
//! root, that one tool call is one JSON-RPC frame, and that a genuine
//! relation scan does not contaminate the stream it shares.

use vibe_test_support as _;

#[path = "requirements_query/dispatch.rs"]
mod dispatch;
#[path = "requirements_query/oracle.rs"]
mod oracle;
#[path = "requirements_query/stdio.rs"]
mod stdio;
#[path = "requirements_query/support.rs"]
mod support;
