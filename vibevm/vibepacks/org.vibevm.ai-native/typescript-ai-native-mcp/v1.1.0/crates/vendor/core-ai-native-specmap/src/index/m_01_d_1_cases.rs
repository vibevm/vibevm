use super::*;

const MD: &str = "# Document {#root}\n\n| Header | Value |\n| --- | --- |\n| @fact:CC-A-4-1 First. @status:impl/done | @fact:CC-A-4-2 Second. @status:impl/done |\n\n> @fact:QUOTE Quoted. @status:impl/done\n";
const XML: &str = "<spec xmlns=\"https://vibevm.org/spec/1\">\n<title id=\"root\">Document</title>\n<table><tr><td>Header</td><td>Value</td></tr><tr><td><CC-A-4-1 fact=\"true\" status=\"impl/done\">First.</CC-A-4-1></td><td><CC-A-4-2 fact=\"true\" status=\"impl/done\">Second.</CC-A-4-2></td></tr></table>\n<quote><QUOTE fact=\"true\" status=\"impl/done\">Quoted.</QUOTE></quote>\n</spec>\n";

fn indexed(form: &str, source: &str) -> crate::generated::specmap::Specmap {
    let root = tempfile::tempdir().unwrap();
    let specs = root.path().join("vibevm/vibespecs");
    std::fs::create_dir_all(&specs).unwrap();
    std::fs::write(specs.join(format!("DOC.{form}")), source).unwrap();
    build(root.path(), &Config::default())
}

#[test]
fn table_cells_and_quote_index_and_explain_match_md_xml() {
    let md = indexed("md", MD);
    let xml = indexed("xml", XML);
    for id in ["CC-A-4-1", "CC-A-4-2", "QUOTE"] {
        let uri = format!("spec://project/DOC#{id}");
        for map in [&md, &xml] {
            let units: Vec<_> = map
                .specUnits
                .iter()
                .filter(|unit| unit.uri == uri)
                .collect();
            assert_eq!(
                units.len(),
                1,
                "missing exact {uri}; units: {:?}, warnings: {:?}",
                map.specUnits
                    .iter()
                    .map(|unit| unit.uri.as_str())
                    .collect::<Vec<_>>(),
                map.warnings
                    .iter()
                    .map(|warning| warning.code.as_str())
                    .collect::<Vec<_>>()
            );
            assert!(units[0].line > 0);
            let explained = crate::explain::explain_text(map, &uri).unwrap();
            assert!(explained.contains(&uri));
            assert!(crate::explain::explain_json(map, &uri).is_ok());
        }
    }
    assert_eq!(
        md.specUnits
            .iter()
            .map(|unit| &unit.uri)
            .collect::<Vec<_>>(),
        xml.specUnits
            .iter()
            .map(|unit| &unit.uri)
            .collect::<Vec<_>>()
    );
}

#[test]
fn table_cell_native_range_stops_at_its_own_row() {
    let source = "# D {#root}\n\n| Header | Value |\n| --- | --- |\n| @fact:FIRST First. | One |\n| @fact:SECOND Second. | Two |\n";
    assert_eq!(
        crate::mdspec::native_anchor_range(source, "FIRST").unwrap(),
        Some((5, 5))
    );
    assert_eq!(
        crate::mdspec::native_anchor_range(source, "SECOND").unwrap(),
        Some((6, 6))
    );
}

#[test]
fn header_structure_does_not_mint_a_body_fact() {
    let md = indexed(
        "md",
        "# D {#root}\n\n| @fact:HEADER heading | Value |\n| --- | --- |\n| @fact:BODY body | Value |\n",
    );
    let xml = indexed(
        "xml",
        "<spec xmlns=\"https://vibevm.org/spec/1\"><title id=\"root\">D</title><table><tr><td><HEADER fact=\"true\">heading</HEADER></td><td>Value</td></tr><tr><td><BODY fact=\"true\">body</BODY></td><td>Value</td></tr></table></spec>",
    );
    for map in [&md, &xml] {
        assert!(map.specUnits.iter().any(|unit| unit.anchor == "BODY"));
        assert!(!map.specUnits.iter().any(|unit| unit.anchor == "HEADER"));
    }
}

#[test]
fn inline_code_table_markers_do_not_mint_facts() {
    let md = indexed(
        "md",
        "# D {#root}\n\n| Header | Value |\n| --- | --- |\n| `@fact:FAKE` example | `##ALSO-FAKE` example |\n",
    );
    let xml = indexed(
        "xml",
        "<spec xmlns=\"https://vibevm.org/spec/1\"><title id=\"root\">D</title><table><tr><td>Header</td><td>Value</td></tr><tr><td>`@fact:FAKE` example</td><td>`##ALSO-FAKE` example</td></tr></table></spec>",
    );
    for map in [&md, &xml] {
        assert!(
            map.specUnits
                .iter()
                .all(|unit| !unit.anchor.contains("FAKE"))
        );
    }
}
