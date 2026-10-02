//! Pure descriptor projection of the authoritative closed JTD subset.
use serde_json::{Map, Value, json};
pub const JTD: &str =
    include_str!("../../../../../schemas/machine/e1/lifecycle_action_request.jtd.json");
fn project(node: &Value, defs: &Value, depth: usize) -> Result<Value, &'static str> {
    if depth > 16 {
        return Err("descriptor depth");
    }
    for key in node.as_object().ok_or("JTD object")?.keys() {
        if ![
            "metadata",
            "definitions",
            "nullable",
            "properties",
            "ref",
            "discriminator",
            "mapping",
            "enum",
            "type",
        ]
        .contains(&key.as_str())
        {
            return Err("unsupported JTD form");
        }
    }
    let mut out = if let Some(r) = node.get("ref") {
        project(
            defs.get(r.as_str().ok_or("ref")?).ok_or("undefined ref")?,
            defs,
            depth + 1,
        )?
    } else if let Some(p) = node.get("properties") {
        let mut properties = Map::new();
        let mut required = Vec::new();
        for (key, v) in p.as_object().ok_or("properties")? {
            properties.insert(key.clone(), project(v, defs, depth + 1)?);
            required.push(key.clone());
        }
        json!({"type":"object","properties":properties,"required":required,"additionalProperties":false})
    } else if let Some(tag) = node.get("discriminator") {
        let tag = tag.as_str().ok_or("tag")?;
        let mut arms = Vec::new();
        for (kind, v) in node["mapping"].as_object().ok_or("mapping")? {
            let mut arm = project(v, defs, depth + 1)?;
            arm["properties"][tag] = json!({"type":"string","const":kind});
            arm["required"]
                .as_array_mut()
                .ok_or("required")?
                .push(json!(tag));
            arms.push(arm);
        }
        json!({"oneOf":arms})
    } else if let Some(values) = node.get("enum") {
        json!({"type":"string","enum":values})
    } else {
        match node["type"].as_str().ok_or("type")? {
            "string" => json!({"type":"string"}),
            "boolean" => json!({"type":"boolean"}),
            "uint8" => json!({"type":"integer","minimum":0,"maximum":255}),
            _ => return Err("unsupported type"),
        }
    };
    if node.get("nullable") == Some(&Value::Bool(true)) {
        out = json!({"anyOf":[out,{"type":"null"}]});
    }
    Ok(out)
}
pub fn input_schema() -> Value {
    let doc: Value = serde_json::from_str(JTD).expect("included authoritative JTD");
    project(&doc, &doc["definitions"], 0).expect("bounded supported JTD subset")
}
