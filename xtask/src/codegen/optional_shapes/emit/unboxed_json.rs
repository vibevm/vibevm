//! Exact pinned unboxed JSON emission; the caller owns site counting.
use super::*;

pub(super) fn emit_unboxed_json(
    line_text: (&str, &str),
    attrs: &[(usize, &str, &str)],
    shapes: &OptionalShapes,
    file: &str,
    line: usize,
    out: &mut String,
) -> Result<bool> {
    let (body, chunk) = line_text;
    let text = body.trim();
    if let Some((ident, "Option<Value>")) = field_parts(text) {
        let wire = attrs
            .iter()
            .find_map(|(_, _, attr)| rename_wire(attr))
            .unwrap_or(ident)
            .to_string();
        let required = if attrs.iter().any(|(_, _, attr)| *attr == OPTION_SKIP) {
            Required::Optional
        } else {
            Required::Required
        };
        let counted = required == Required::Optional
            || shapes
                .map
                .keys()
                .any(|(name, _, r)| name == &wire && *r == required);
        if counted {
            match lookup(
                shapes,
                &wire,
                (ShapeClass::Any, required),
                file,
                line,
                ident,
                "Value",
            )? {
                Decision::OptionValue => {
                    emit_rewritten_run(attrs, out, file, line, ident, OPTION_FORM)?
                }
                Decision::RequiredNullable => {
                    emit_required_nullable_run(attrs, out, file, line, ident, body, chunk)?
                }
                Decision::BoolFalse => {
                    bail!("{file}:{line}: arbitrary JSON cannot have a false default")
                }
            }
            out.push_str(chunk);
            return Ok(true);
        }
    }

    Ok(false)
}
