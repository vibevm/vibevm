//! The per-item fingerprint: a stable `tok1:<sha256>` over an element's
//! token stream, walked node-for-node so that formatting and ordinary
//! comments do not move it while a rewritten doc comment (or a space inside
//! a string literal) does. PROP-014 §2.5: the code item carries this
//! identity alongside its span.

specmark::scope!("spec://org.vibevm.ai-native/core-ai-native/mechanisms/PROP-014#addressing-code");

use quote::ToTokens;
use sha2::Digest;

/// Stable per-item fingerprint: `tok1:<sha256>` over the element's token
/// stream walked node-for-node. Walking the tree — rather than
/// `to_token_stream().to_string().replace(' ', "")` — keeps whitespace
/// *inside* string literals significant, so `"a b"` and `"ab"` get distinct
/// fingerprints. The scheme name (`tok1`) lives inside the value because
/// readers compare fingerprints, they do not parse them: changing the hashed
/// substance is a regeneration, not a format change.
pub(crate) fn fingerprint_of(tokens: impl ToTokens) -> String {
    use sha2::Sha256;
    let mut hasher = Sha256::new();
    feed_tokens(&mut hasher, tokens.into_token_stream());
    let digest = hasher.finalize();
    let mut out = String::with_capacity(5 + digest.len() * 2);
    out.push_str("tok1:");
    for b in digest {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

/// Feed a length-prefixed byte string into the hasher. The length prefix
/// makes the encoding injective — `[Ident("a"), Ident("b")]` cannot collide
/// with `[Ident("ab")]` — so two distinct token streams never share a digest.
fn feed_str(hasher: &mut sha2::Sha256, s: &str) {
    let bytes = s.as_bytes();
    hasher.update((bytes.len() as u64).to_le_bytes());
    hasher.update(bytes);
}

/// A one-token spelling of a group delimiter. `None` (the invisible group)
/// maps to the empty string, which still differs from the three real ones.
fn delimiter_str(d: proc_macro2::Delimiter) -> &'static str {
    match d {
        proc_macro2::Delimiter::Parenthesis => "(",
        proc_macro2::Delimiter::Brace => "{",
        proc_macro2::Delimiter::Bracket => "[",
        proc_macro2::Delimiter::None => "",
    }
}

/// Walk a token stream into the hasher one node at a time: groups carry
/// their delimiter plus a close marker (so `()a` differs from `(a)`), idents
/// and literals go in verbatim, and punctuation carries its `Joint`/`Alone`
/// spacing (so `<=` differs from `< =`). Doc comments survive the lexer as
/// `#[doc = …]` attributes and so move the hash; ordinary `//` comments are
/// not tokens at all and so do not.
fn feed_tokens(hasher: &mut sha2::Sha256, stream: proc_macro2::TokenStream) {
    use proc_macro2::TokenTree;
    for tt in stream {
        match tt {
            TokenTree::Group(g) => {
                hasher.update(b"G");
                feed_str(hasher, delimiter_str(g.delimiter()));
                feed_tokens(hasher, g.stream());
                hasher.update(b"C");
            }
            TokenTree::Ident(i) => {
                hasher.update(b"I");
                feed_str(hasher, &i.to_string());
            }
            TokenTree::Punct(p) => {
                hasher.update(b"P");
                feed_str(hasher, &p.as_char().to_string());
                hasher.update(match p.spacing() {
                    proc_macro2::Spacing::Joint => b"J",
                    proc_macro2::Spacing::Alone => b"A",
                });
            }
            TokenTree::Literal(l) => {
                hasher.update(b"L");
                feed_str(hasher, &l.to_string());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fingerprint_of;

    /// Fingerprint of the first item in `src` — isolates the hash from the
    /// full scan so the three §2.5 properties and the §2.4 trap are tested
    /// straight against tokenisation, not the inventory plumbing.
    fn first_item_fingerprint(src: &str) -> String {
        let ast = syn::parse_file(src).expect("test source parses");
        let item = ast.items.into_iter().next().expect("exactly one item");
        fingerprint_of(&item)
    }

    #[test]
    fn fingerprint_ignores_whitespace_and_layout() {
        // Token-identical sources (same punctuation, no trailing comma in
        // either) that differ only in whitespace, indentation and line
        // breaks — the fingerprint must not move.
        let compact = "pub fn add(x: u32, y: u32) -> u32 { x + y }";
        let airy = "pub   fn   add(  x:u32 ,   y:u32 )\n\t-> u32\n{\n    x + y\n}";
        let a = first_item_fingerprint(compact);
        let b = first_item_fingerprint(airy);
        println!("whitespace: compact={a} airy={b}");
        assert_eq!(a, b, "formatting must not move the fingerprint");
    }

    #[test]
    fn fingerprint_ignores_ordinary_line_comments() {
        // An ordinary `//` comment is not a token; adding one must not move
        // the fingerprint.
        let plain = "pub fn add(x: u32, y: u32) -> u32 { x + y }";
        let commented =
            "// a plain note, not a doc comment\npub fn add(x: u32, y: u32) -> u32 { x + y }";
        let a = first_item_fingerprint(plain);
        let b = first_item_fingerprint(commented);
        println!("ordinary-comment: plain={a} commented={b}");
        assert_eq!(a, b, "an ordinary comment must not move the fingerprint");
    }

    #[test]
    fn fingerprint_tracks_doc_comments() {
        // A `///` doc comment lowers to `#[doc = "…"]`, which IS a token —
        // rewriting it must move the fingerprint.
        let original = "/// original wording\npub fn add(x: u32, y: u32) -> u32 { x + y }";
        let rewritten = "/// rewritten wording\npub fn add(x: u32, y: u32) -> u32 { x + y }";
        let a = first_item_fingerprint(original);
        let b = first_item_fingerprint(rewritten);
        println!("doc-comment: original={a} rewritten={b}");
        assert_ne!(a, b, "a rewritten doc comment must move the fingerprint");
    }

    #[test]
    fn fingerprint_distinguishes_spaces_inside_string_literals() {
        // §2.4 trap: a `to_string().replace(' ', "")` shortcut would strip
        // the space and collide these. Walking the tree keeps the literal
        // verbatim, so they differ.
        let spaced = "pub const MSG: &str = \"a b\";";
        let tight = "pub const MSG: &str = \"ab\";";
        let a = first_item_fingerprint(spaced);
        let b = first_item_fingerprint(tight);
        println!("literal-space: spaced={a} tight={b}");
        assert_ne!(
            a, b,
            "a space inside a string literal must move the fingerprint"
        );
    }
}
