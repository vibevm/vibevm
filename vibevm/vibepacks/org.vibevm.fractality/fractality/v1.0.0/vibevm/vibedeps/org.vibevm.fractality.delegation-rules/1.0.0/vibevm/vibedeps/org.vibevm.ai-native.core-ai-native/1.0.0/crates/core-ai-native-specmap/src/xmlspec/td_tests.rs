//! The `<td>` content-expressibility law — split from `tests.rs` along
//! the cell seam when the file crossed the length budget.

use super::*;

const NS: &str = "project";

fn fmt_warnings(w: &[Warning]) -> String {
    w.iter()
        .map(|x| format!("{}:{} [{}] {}", x.file, x.line, x.code, x.message))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn a_td_pipe_inside_a_code_span_is_expressible_and_a_bare_pipe_is_not() {
    // Mirror of the host pivot's K2.6 rule: the Markdown scanner masks
    // inline-code spans before splitting cells, so `` `a | b` `` inside a
    // <td> IS expressible; a bare pipe still is not.
    let ok = concat!(
        "<spec xmlns=\"https://vibevm.org/spec/1\">\n",
        "  <section id=\"s\" title=\"S\">\n",
        "    <table><tr><td>`\"none\" | \"ssh\"`</td></tr></table>\n",
        "  </section>\n",
        "</spec>\n"
    );
    let (units, warnings) = parse_units("spec/test/DOC.xml", ok, NS);
    assert!(warnings.is_empty(), "{}", fmt_warnings(&warnings));
    assert_eq!(units.len(), 1, "the masked pipe must parse");

    let bare = concat!(
        "<spec xmlns=\"https://vibevm.org/spec/1\">\n",
        "  <section id=\"s\" title=\"S\">\n",
        "    <table><tr><td>a | b</td></tr></table>\n",
        "  </section>\n",
        "</spec>\n"
    );
    let (_units, warnings) = parse_units("spec/test/DOC.xml", bare, NS);
    assert!(
        fmt_warnings(&warnings).contains("cannot hold"),
        "a bare pipe must keep the loud limit: {}",
        fmt_warnings(&warnings)
    );
}
