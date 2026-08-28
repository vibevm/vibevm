//! The cell's own behavior oracle.
//!
//! The dispatcher and stdio cells drive the tool by NAME, through the
//! string-routed server — which proves the wire but leaves the concrete
//! `#[cell]` unclaimed: nothing there says which type owns that behavior.
//! This cell imports [`RequirementsQueryMcpTool`] and drives it directly
//! through the [`McpTool`] seam, so the type has a differential net that
//! moves when it does.
//!
//! It is differential, not descriptive: the expected value is not a
//! transcribed JSON blob but the report `vibe_requirements::query`
//! produces for the same question over the same node, at the same injected
//! clock. A cell that starts filtering, re-sorting, re-keying, truncating
//! or re-rendering the answer fails here — which is exactly the law
//! (architecture §6.3) that both surfaces return the SAME generated root
//! and differ only in argument framing and text projection.

use serde_json::json;
use vibe_mcp::ServerContext;
use vibe_mcp::tools::{McpTool, RequirementsQueryMcpTool};
use vibe_requirements::{QueryContext, RequirementsQuery};

use super::support::{ADDRESS, HOST, PROSE, project_with_map};

#[test]
fn the_cell_answers_the_one_fact_fixture_with_exactly_the_generated_root() {
    let project = project_with_map();
    let output = RequirementsQueryMcpTool
        .run(&json!({}), &ServerContext::new(project.path()))
        .expect("the one-fact fixture answers");
    assert!(!output.is_error());

    // The clock the surface injected. It is the ONE member excluded from
    // `observation_id`, so handing it to the oracle is what makes the two
    // roots comparable byte for byte rather than approximately.
    let observed_at = output.structured()["observation"]["observed_at"]
        .as_str()
        .expect("the observation carries its clock")
        .parse()
        .expect("an RFC3339 timestamp");
    let oracle = vibe_requirements::query(
        &RequirementsQuery::default(),
        &QueryContext {
            selected_root: project.path().to_path_buf(),
            observed_at,
            lifecycle_run_id: None,
        },
        None,
    )
    .expect("the library answers the same question");

    assert_eq!(
        output.structured(),
        &serde_json::to_value(&oracle).unwrap(),
        "the cell returns the generated root, member for member",
    );
    assert_eq!(
        output.text(),
        vibe_requirements::text::render(&oracle),
        "the text channel is the library's own bounded projection",
    );

    // Stated once against the fixture too, so an oracle that silently
    // became "empty equals empty" is red rather than green.
    assert_eq!(oracle.rows.len(), 1);
    assert_eq!(output.structured()["rows"][0]["address"], ADDRESS);
    assert_eq!(output.structured()["sources"][0]["source"]["package"], HOST);
    assert!(output.text().contains(ADDRESS));
    assert!(
        !output.text().contains(PROSE),
        "bounded metadata never carries the authored sentence: {}",
        output.text()
    );
}
