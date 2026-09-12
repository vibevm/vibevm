//! The closed inline vocabulary, one law per convention.

use super::*;

/// The five conventions, each once.
#[test]
fn the_five_conventions_render_and_nothing_else_does() {
    assert_eq!(render("`vibe list`"), "<code>vibe list</code>");
    assert_eq!(render("**bold**"), "<strong>bold</strong>");
    assert_eq!(render("*soft*"), "<em>soft</em>");
    assert_eq!(render("_soft_"), "<em>soft</em>");
    assert_eq!(render("[a](/b/)"), "<a href=\"/b/\">a</a>");
    assert_eq!(
        render("<https://vibevm.org>"),
        "<a href=\"https://vibevm.org\">https://vibevm.org</a>"
    );
    // Not in the vocabulary: an image, a heading marker, a footnote.
    assert_eq!(render("![alt](x.png)"), "!<a href=\"x.png\">alt</a>");
    assert_eq!(render("# not a heading"), "# not a heading");
    assert_eq!(render("a[^1]"), "a[^1]");
}

/// A manual quotes markup constantly, so the code span has to win — and
/// its content has to survive untouched.
#[test]
fn a_code_span_binds_first_and_its_content_is_never_markup() {
    assert_eq!(render("`**x**`"), "<code>**x**</code>");
    assert_eq!(render("`[a](b)`"), "<code>[a](b)</code>");
    assert_eq!(render("`<rule/>`"), "<code>&lt;rule/&gt;</code>");
    // A doubled backtick span may hold a single one.
    assert_eq!(render("``a ` b``"), "<code>a ` b</code>");
}

/// Everything outside the vocabulary is text, and text is escaped —
/// including text a page quotes from an XML dialect.
#[test]
fn text_is_escaped_so_a_quoted_element_cannot_become_one() {
    assert_eq!(
        render("<script>x</script>"),
        "&lt;script&gt;x&lt;/script&gt;"
    );
    assert_eq!(render("a & b"), "a &amp; b");
}

/// An underscore inside a word is part of the word. A manual naming
/// `content_hash` must not sprout emphasis.
#[test]
fn an_underscore_inside_a_word_is_part_of_the_word() {
    assert_eq!(render("content_hash_field"), "content_hash_field");
    assert_eq!(render("a _b_ c"), "a <em>b</em> c");
}

/// An unclosed marker is text: a renderer that swallows the rest of a
/// paragraph because somebody typed one asterisk is worse than one that
/// prints the asterisk.
#[test]
fn an_unclosed_marker_stays_text() {
    assert_eq!(render("a * b"), "a * b");
    assert_eq!(render("`unclosed"), "`unclosed");
    assert_eq!(render("[a](b"), "[a](b");
    assert_eq!(render("**strong"), "**strong");
}

/// Markup nests: a link label carries its own, and strong carries code.
#[test]
fn markup_nests_inside_a_link_label_and_inside_strong() {
    assert_eq!(
        render("[the `list` verb](/doc/x/)"),
        "<a href=\"/doc/x/\">the <code>list</code> verb</a>"
    );
    assert_eq!(
        render("**the `list` verb**"),
        "<strong>the <code>list</code> verb</strong>"
    );
}

/// An href is attribute-escaped, which is a different escape from text.
#[test]
fn an_href_is_attribute_escaped() {
    assert_eq!(
        render("[x](/a?b=1&c=\"2\")"),
        "<a href=\"/a?b=1&amp;c=&quot;2&quot;\">x</a>"
    );
}

/// An autolink is a URL and nothing else; an angle bracket around
/// anything else is text.
#[test]
fn only_a_url_autolinks() {
    assert_eq!(render("<not a url>"), "&lt;not a url&gt;");
    assert_eq!(render("<https://a b>"), "&lt;https://a b&gt;");
    assert_eq!(
        render("<spec://org.demo/lib/guide#X>"),
        "<a href=\"spec://org.demo/lib/guide#X\">spec://org.demo/lib/guide#X</a>"
    );
}
