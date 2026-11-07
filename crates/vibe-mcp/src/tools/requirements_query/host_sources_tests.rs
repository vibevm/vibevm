//! Actual MCP query reads original host policy through the shared library.
#[cfg(test)]
mod cases {
    use crate::{
        ServerContext,
        tools::{McpTool, RequirementsQueryMcpTool},
    };
    use std::{
        collections::BTreeMap,
        fs,
        path::{Path, PathBuf},
    };
    fn snapshot(root: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
        fn walk(root: &Path, at: &Path, rows: &mut BTreeMap<PathBuf, Option<Vec<u8>>>) {
            for entry in fs::read_dir(at).unwrap() {
                let path = entry.unwrap().path();
                let relative = path.strip_prefix(root).unwrap().into();
                if path.is_dir() {
                    rows.insert(relative, None);
                    walk(root, &path, rows);
                } else {
                    rows.insert(relative, Some(fs::read(path).unwrap()));
                }
            }
        }
        let mut rows = BTreeMap::new();
        walk(root, root, &mut rows);
        rows
    }
    #[test]
    #[specmark::spec(
        verifies = "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-HOST-INVENTORY",
        r = 1
    )]
    fn mcp_host_native_and_partial_states_are_read_only_and_do_not_expose_raw_body() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("vibe.toml"),"[project]\ngroup='org.example'\nname='host'\nversion='1.0.0'\n[[project.sources]]\nfile='native.payload'\nmedia='application/xml'\ninterpretation='vibe-spec'\nrole='spec'\nloading='explicit'\n[[project.sources]]\nfile='raw.md'\nmedia='text/markdown'\ninterpretation='commonmark'\nrole='resource'\nloading='scan'\n").unwrap();
        let native = root.path().join("native.payload");
        fs::write(&native,"<spec xmlns=\"https://vibevm.org/spec/1\"><p><REAL fact=\"true\" status=\"impl/done\">native text</REAL></p></spec>").unwrap();
        fs::write(
            root.path().join("raw.md"),
            "@fact:FAKE MCP-RAW-CANARY. @status:impl/done\n#embed spec://missing/package/root\n",
        )
        .unwrap();
        let context = ServerContext::new(root.path());
        for state in ["available", "invalid", "unavailable"] {
            match state {
                "invalid" => fs::write(&native, "<broken").unwrap(),
                "unavailable" => fs::remove_file(&native).unwrap(),
                _ => {}
            }
            let before = snapshot(root.path());
            let output = RequirementsQueryMcpTool
                .run(&serde_json::json!({}), &context)
                .unwrap();
            assert!(!output.is_error());
            let report: vibe_wire::generated::requirements_report::RequirementsReport =
                serde_json::from_value(output.structured().clone()).unwrap();
            vibe_wire::behaviour::requirements_report::validate(&report).unwrap();
            let source = output.structured()["sources"]
                .as_array()
                .unwrap()
                .iter()
                .find(|source| source["source"]["package"] == "org.example/host")
                .unwrap();
            assert_eq!(source["state"], state);
            if state == "available" {
                assert_eq!(report.rows.len(), 1);
                assert_eq!(
                    report.rows[0].address,
                    "spec://org.example/host/native.payload#REAL"
                );
            } else {
                assert!(report.rows.is_empty());
            }
            assert_eq!(source["digest"].is_null(), state == "unavailable");
            assert!(!output.structured().to_string().contains("MCP-RAW-CANARY"));
            assert_eq!(snapshot(root.path()), before);
            assert!(!root.path().join(".vibe").exists());
        }
    }
}
