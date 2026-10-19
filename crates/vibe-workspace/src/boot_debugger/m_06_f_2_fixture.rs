// Synthetic recipe m06f2/v1: 128 ordered contributions, 8192 whitespace
// units each (1,048,576 total), repeated source identity, multibyte payload.
// These units are NOT a model tokenizer measurement.
fn scale_static() -> String {
    let payload = "λ ".repeat(8192);
    let mut text = String::new();
    for _ in 0..128 {
        text.push_str("<!-- vibe:static org.example/scale — repeated.md -->\n");
        text.push_str(&payload);
        text.push('\n');
    }
    text
}
const SCALE_INDEX: &str = "[[entry]]\npath='ordinary.txt'\nkind='dynamic'\n[[entry]]\npath='ordinary.txt'\nkind='dynamic'\nwhen='os:linux'\n";
