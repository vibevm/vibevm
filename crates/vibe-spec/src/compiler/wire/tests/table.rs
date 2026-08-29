//! Table-driven phase coverage: one planted fault in one field from every
//! arm/family of the generated wire, each asserting the registry label of
//! the phase that owns it. A family missing from the table is a family the
//! preflight may silently skip — keep the table exhaustive by arm.

use std::path::PathBuf;

use super::super::decode;

fn corpus() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("formats/corpora/compiler_ir/e1")
}

fn base(name: &str) -> serde_json::Value {
    serde_json::from_slice(&std::fs::read(corpus().join("valid").join(name)).unwrap()).unwrap()
}

type Mutation = fn(&mut serde_json::Value);

fn check(name: &str, base_doc: &str, mutation: Mutation, label: &str) {
    let mut document = base(base_doc);
    mutation(&mut document);
    let error = decode(&serde_json::to_vec(&document).unwrap())
        .unwrap_err()
        .to_string();
    assert!(
        error.contains(&format!("gate `{label}`")),
        "{name}: expected `{label}`, got {error}"
    );
}

const BLANK: &str = "  ";
const UPPER: &str = "ED73521B98C3CAB322C923AFD66C6A5ECBE81A2A24983071F3CA35DE314EA4F8";

#[test]
fn scalar_identities_of_every_family() {
    let cases: &[(&str, &str, Mutation)] = &[
        ("source format", "source_document.json", |d| {
            d["doc"]["format"] = serde_json::json!(BLANK);
        }),
        ("source static-entry path", "source_document.json", |d| {
            d["doc"]["address"] =
                serde_json::json!({"kind": "static-entry", "origin": "o", "path": BLANK});
        }),
        ("subject declared path", "source_document.json", |d| {
            d["doc"]["subject"]["declared_path"] = serde_json::json!(BLANK);
        }),
        ("subject provider group", "source_document.json", |d| {
            d["doc"]["subject"]["provider"] =
                serde_json::json!({"kind": "dependency", "group": BLANK, "name": "lib"});
        }),
        ("subject provider name", "source_document.json", |d| {
            d["doc"]["subject"]["provider"] =
                serde_json::json!({"kind": "host-coordinate", "group": "org.demo", "name": "\n"});
        }),
        ("subject provider host name", "source_document.json", |d| {
            d["doc"]["subject"]["provider"] =
                serde_json::json!({"kind": "host-ungrouped", "name": BLANK});
        }),
        (
            "nested subject declared path",
            "documents_artifact.json",
            |d| {
                d["documents"][1]["source"]["subject"]["declared_path"] = serde_json::json!("a\nb");
            },
        ),
        ("node anchor id", "document_document.json", |d| {
            d["doc"]["tree"]["nodes"][1]["id"] = serde_json::json!(BLANK);
        }),
        ("anchor name", "document_document.json", |d| {
            d["doc"]["tree"]["anchors"][BLANK] = serde_json::json!(1);
        }),
        ("duplicate-anchor name", "document_document.json", |d| {
            d["doc"]["tree"]["duplicate_anchors"] = serde_json::json!(["a\nb"]);
        }),
        ("directive alias key", "document_document.json", |d| {
            let value = d["doc"]["tree"]["directives"]["aliases"]["Part"].clone();
            d["doc"]["tree"]["directives"]["aliases"][BLANK] = value;
        }),
        ("closure alias key", "closure_artifact.json", |d| {
            d["closure"]["nodes"][2]["aliases"][BLANK] =
                d["closure"]["nodes"][2]["tree"]["directives"]["aliases"]["Part"].clone();
        }),
        ("closure rename original", "closure_artifact.json", |d| {
            d["closure"]["renames"][0]["rename"]["original"] = serde_json::json!(BLANK);
        }),
        ("lane rename qualified", "lane_artifact.json", |d| {
            d["lane"]["frame"]["renames"][0]["rename"]["qualified"] = serde_json::json!("\n");
        }),
        // (emission renames ride the same law; the emitted corpus carries an
        // empty rename list, so the closure/lane sites above carry coverage)
        ("lane frame generated path", "lane_artifact.json", |d| {
            d["lane"]["frame"]["generated_path"] = serde_json::json!(BLANK);
        }),
        ("lane chunk marker key", "lane_artifact.json", |d| {
            d["lane"]["contributions"][0]["chunks"][0]["marker"] = serde_json::json!(BLANK);
        }),
        ("link occurrence marker key", "closure_artifact.json", |d| {
            d["closure"]["link"]["result"]["occurrences"][0]["marker"] = serde_json::json!(BLANK);
        }),
        (
            "snapshot document key",
            "closure_artifact_compat.json",
            |d| {
                let value =
                    d["closure"]["pending_sources"]["documents"]["spec://demo/manual/missing.md"]
                        .clone();
                d["closure"]["pending_sources"]["documents"]["bad\nkey"] = value;
            },
        ),
        (
            "discovery order entry",
            "closure_artifact_compat.json",
            |d| {
                d["closure"]["pending_sources"]["discovery_order"][0] = serde_json::json!(BLANK);
            },
        ),
        ("explicit use key", "closure_artifact_compat.json", |d| {
            d["closure"]["pending_embeds"]["explicit_use_keys"][0] = serde_json::json!("");
        }),
        (
            "pending tree node id",
            "closure_artifact_compat.json",
            |d| {
                d["closure"]["pending_sources"]["documents"]["spec://demo/manual/base.md#base"]["document"]
                    ["tree"]["nodes"][1]["id"] = serde_json::json!(BLANK);
            },
        ),
        (
            "absorption simple address origin",
            "closure_artifact.json",
            |d| {
                d["closure"]["absorption"]["plan"]["contributions"][1]["address"]["origin"] =
                    serde_json::json!(BLANK);
            },
        ),
        ("link simple address path", "closure_artifact.json", |d| {
            d["closure"]["link"]["result"]["contributions"][1]["address"]["path"] =
                serde_json::json!(BLANK);
        }),
        (
            "emission simple address origin",
            "emitted_artifact.json",
            |d| {
                d["emitted"]["provenance"]["contributions"][1]["address"]["origin"] =
                    serde_json::json!(BLANK);
            },
        ),
        ("emit backend charset", "emitted_artifact.json", |d| {
            d["emitted"]["provenance"]["backend"] = serde_json::json!("Bad!");
            d["emitted"]["provenance"]["context"]["artifact"] = serde_json::json!("Bad!");
            d["emitted"]["provenance"]["context"]["target"] = serde_json::json!("Bad!");
            d["emitted"]["provenance"]["producer"] = serde_json::json!("emit:Bad!");
        }),
    ];
    for (name, base_doc, mutation) in cases {
        check(name, base_doc, *mutation, "scalar-ids");
    }
}

#[test]
fn context_tuples_of_every_site() {
    let cases: &[(&str, &str, Mutation)] = &[
        ("closure context", "closure_artifact.json", |d| {
            d["closure"]["context"]["artifact"] = serde_json::json!("wrong-id");
        }),
        ("lane context", "lane_artifact.json", |d| {
            d["lane"]["context"]["artifact"] = serde_json::json!("wrong-id");
        }),
        ("emitted context", "emitted_artifact.json", |d| {
            d["emitted"]["provenance"]["context"]["artifact"] = serde_json::json!("wrong-id");
        }),
    ];
    for (name, base_doc, mutation) in cases {
        check(name, base_doc, *mutation, "context-tuple");
    }
}

#[test]
fn origin_relations_of_every_site() {
    let cases: &[(&str, &str, Mutation)] = &[
        ("closure normal", "closure_artifact.json", |d| {
            d["closure"]["contributions"][0]["meta"]["origin"] = serde_json::json!("org.demo/x");
        }),
        ("closure hoisted", "closure_artifact.json", |d| {
            d["closure"]["contributions"][3]["meta"]["origin"] = serde_json::json!("org.demo/x");
        }),
        ("plan normal", "closure_artifact.json", |d| {
            d["closure"]["absorption"]["plan"]["contributions"][0]["meta"]["origin"] =
                serde_json::json!("org.demo/x");
        }),
        ("link witness normal", "closure_artifact.json", |d| {
            d["closure"]["link"]["result"]["contributions"][0]["meta"]["origin"] =
                serde_json::json!("org.demo/x");
        }),
        ("lane normal", "lane_artifact.json", |d| {
            d["lane"]["contributions"][0]["meta"]["origin"] = serde_json::json!("org.demo/x");
        }),
        ("emission witness normal", "emitted_artifact.json", |d| {
            d["emitted"]["provenance"]["contributions"][0]["meta"]["origin"] =
                serde_json::json!("org.demo/x");
        }),
    ];
    for (name, base_doc, mutation) in cases {
        check(name, base_doc, *mutation, "origin-package-relation");
    }
}

#[test]
fn digest_spellings_of_every_site() {
    let cases: &[(&str, &str, Mutation)] = &[
        ("link input digest", "closure_artifact.json", |d| {
            d["closure"]["link"]["result"]["input_digest"] = serde_json::json!(UPPER);
        }),
        ("lane source link digest", "lane_artifact.json", |d| {
            d["lane"]["source_link_digest"] = serde_json::json!(UPPER);
        }),
        ("source lane digest", "emitted_artifact.json", |d| {
            d["emitted"]["provenance"]["source_lane_digest"] = serde_json::json!(UPPER);
        }),
        ("chunk digest", "emitted_artifact.json", |d| {
            d["emitted"]["provenance"]["contributions"][0]["chunk_digest"] =
                serde_json::json!(UPPER);
        }),
        ("bytes digest", "emitted_artifact.json", |d| {
            d["emitted"]["provenance"]["bytes_digest"] = serde_json::json!(UPPER);
        }),
        ("bytes base64", "emitted_artifact.json", |d| {
            d["emitted"]["bytes_b64"] = serde_json::json!("AP9=");
        }),
    ];
    for (name, base_doc, mutation) in cases {
        check(name, base_doc, *mutation, "digest-base64-canonical");
    }
}

#[test]
fn address_and_fence_families_of_every_site() {
    fn drift(d: &mut serde_json::Value, pointer: &[&str], raw: &str) {
        let mut cursor = d;
        for key in pointer {
            cursor = if let Ok(index) = key.parse::<usize>() {
                &mut cursor[index]
            } else {
                &mut cursor[*key]
            };
        }
        cursor["raw"] = serde_json::json!(raw);
    }
    let cases: &[(&str, &str, Mutation)] = &[
        ("source address", "source_document.json", |d| {
            drift(d, &["doc", "address", "address"], "spec://x/y/z#q");
        }),
        ("directive address", "document_document.json", |d| {
            drift(
                d,
                &["doc", "tree", "directives", "directives", "0", "address"],
                "spec://x/y/z#q",
            );
        }),
        ("in-place use address", "document_document.json", |d| {
            drift(
                d,
                &["doc", "tree", "directives", "in_place_uses", "0", "address"],
                "spec://x/y/z#q",
            );
        }),
        ("tree alias value", "document_document.json", |d| {
            drift(
                d,
                &["doc", "tree", "directives", "aliases", "Part"],
                "spec://x/y/z#q",
            );
        }),
        ("closure node address", "closure_artifact.json", |d| {
            drift(
                d,
                &["closure", "nodes", "0", "address", "address"],
                "spec://x/y/z#q",
            );
        }),
        ("closure alias value", "closure_artifact.json", |d| {
            drift(
                d,
                &[
                    "closure",
                    "nodes",
                    "2",
                    "tree",
                    "directives",
                    "aliases",
                    "Part",
                ],
                "spec://x/y/z#q",
            );
        }),
        // The `Simple` contribution's whole lowered document rides OUTSIDE the
        // graph: its identity, its tree's directives, and its `#use` bindings
        // are all the address phase's own (repair 2, finding 3).
        (
            "simple contribution address",
            "closure_artifact.json",
            |d| {
                let mut address = d["closure"]["nodes"][0]["address"].clone();
                address["address"]["raw"] = serde_json::json!("spec://x/y/z#q");
                d["closure"]["contributions"][1]["document"]["address"] = address;
            },
        ),
        (
            "simple contribution directive",
            "closure_artifact.json",
            |d| {
                let mut directive =
                    d["closure"]["nodes"][2]["tree"]["directives"]["directives"][0].clone();
                directive["line"] = serde_json::json!(0);
                directive["address"]["raw"] = serde_json::json!("spec://x/y/z#q");
                d["closure"]["contributions"][1]["document"]["tree"]["directives"]["directives"] =
                    serde_json::json!([directive]);
            },
        ),
        ("simple contribution alias", "closure_artifact.json", |d| {
            let mut alias =
                d["closure"]["nodes"][2]["tree"]["directives"]["aliases"]["Part"].clone();
            alias["raw"] = serde_json::json!("spec://x/y/z#q");
            d["closure"]["contributions"][1]["document"]["aliases"]["Part"] = alias;
        }),
        ("edge requested target", "closure_artifact.json", |d| {
            drift(
                d,
                &["closure", "edges", "0", "requested_target"],
                "spec://x/y/z#q",
            );
        }),
        ("closure seed", "closure_artifact.json", |d| {
            drift(
                d,
                &["closure", "contributions", "0", "seed_address"],
                "spec://x/y/z#q",
            );
        }),
        (
            "emission occurrence request",
            "closure_artifact.json",
            |d| {
                drift(
                    d,
                    &[
                        "closure",
                        "contributions",
                        "0",
                        "emission_order",
                        "0",
                        "requested_address",
                    ],
                    "spec://x/y/z#q",
                );
            },
        ),
        ("plan occurrence request", "closure_artifact.json", |d| {
            drift(
                d,
                &[
                    "closure",
                    "absorption",
                    "plan",
                    "contributions",
                    "0",
                    "occurrences",
                    "0",
                    "requested_address",
                ],
                "spec://x/y/z#q",
            );
        }),
        ("link witness seed", "closure_artifact.json", |d| {
            drift(
                d,
                &[
                    "closure",
                    "link",
                    "result",
                    "contributions",
                    "0",
                    "seed_address",
                ],
                "spec://x/y/z#q",
            );
        }),
        ("link occurrence address", "closure_artifact.json", |d| {
            drift(
                d,
                &["closure", "link", "result", "occurrences", "0", "address"],
                "spec://x/y/z#q",
            );
        }),
        ("lane seed", "lane_artifact.json", |d| {
            drift(
                d,
                &["lane", "contributions", "0", "seed_address"],
                "spec://x/y/z#q",
            );
        }),
        ("lane node requested address", "lane_artifact.json", |d| {
            drift(
                d,
                &[
                    "lane",
                    "contributions",
                    "0",
                    "chunks",
                    "1",
                    "node",
                    "requested_address",
                ],
                "spec://x/y/z#q",
            );
        }),
        ("emission witness seed", "emitted_artifact.json", |d| {
            drift(
                d,
                &[
                    "emitted",
                    "provenance",
                    "contributions",
                    "0",
                    "seed_address",
                ],
                "spec://x/y/z#q",
            );
        }),
        (
            "snapshot resolved address",
            "closure_artifact_compat.json",
            |d| {
                drift(
                    d,
                    &[
                        "closure",
                        "pending_sources",
                        "documents",
                        "spec://demo/manual/base.md#base",
                        "document",
                        "source",
                        "address",
                        "address",
                    ],
                    "spec://x/y/z#q",
                );
            },
        ),
        (
            "failed observation requested",
            "closure_artifact_compat.json",
            |d| {
                drift(
                    d,
                    &[
                        "closure",
                        "pending_sources",
                        "documents",
                        "spec://demo/manual/missing.md",
                        "requested",
                    ],
                    "spec://x/y/z#q",
                );
            },
        ),
        (
            "failed expansion requested",
            "closure_artifact_compat.json",
            |d| {
                drift(
                    d,
                    &[
                        "closure",
                        "pending_sources",
                        "expansions",
                        "spec://demo/manual/base.md#base.v1",
                        "requested",
                    ],
                    "spec://x/y/z#q",
                );
            },
        ),
        (
            "resolved expansion requested",
            "closure_artifact_compat.json",
            |d| {
                drift(
                    d,
                    &[
                        "closure",
                        "pending_sources",
                        "expansions",
                        "spec://demo/manual/base.md#base",
                        "requested",
                    ],
                    "spec://x/y/z#q",
                );
            },
        ),
        ("expansion target", "closure_artifact_compat.json", |d| {
            drift(
                d,
                &[
                    "closure",
                    "pending_sources",
                    "expansions",
                    "spec://demo/manual/base.md#base",
                    "targets",
                    "0",
                ],
                "spec://x/y/z#q",
            );
        }),
        ("link fence delimiter", "closure_artifact.json", |d| {
            d["closure"]["link"]["result"]["occurrences"][0]["fence_after"]["delimiter"] =
                serde_json::json!("~~");
        }),
        ("lane fence delimiter", "lane_artifact.json", |d| {
            d["lane"]["contributions"][0]["chunks"][1]["node"]["fence_after"]["delimiter"] =
                serde_json::json!("~~");
        }),
    ];
    for (name, base_doc, mutation) in cases {
        check(name, base_doc, *mutation, "address-reparse");
    }
}
