//! The bare shell a Node-less build ships (PROP-057
//! `##SHELL-RELEASE-BUILD`, `##LOCAL-OFFLINE-SHELL`).
//!
//! `cargo build` on a machine with no Node produces a working `vibe`, and
//! `vibe doc serve` on it produces a readable page. That is the whole
//! claim, and everything about this file follows from it: the page is
//! typography and the semantic tokens, and NOT ONE SCRIPT — no theme
//! switch, no settings, no table of contents, no return to place. The
//! features are gone and the reading is not.
//!
//! Scripts are absent on purpose rather than by omission. A fallback with
//! a script would need its hash in the policy, which would make the
//! narrowest configuration of the reader the one with the widest policy;
//! and a reader that cannot fetch its own shell is the configuration
//! where a stray script is least welcome. The stylesheet is a FILE for
//! the same reason: `##LOCAL-CSP` names `style-src 'self'`, and a
//! fallback that needed `'unsafe-inline'` would loosen the policy for the
//! case that needs it least.
//!
//! The tokens are the light half of the design system's palette, written
//! out here rather than shared: this crate must not depend on the web
//! package, which is a pnpm workspace that is not built on this machine
//! by definition when this shell is the one in use.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#SHELL-RELEASE-BUILD");

/// The stylesheet's address under the reader's base.
pub const STYLESHEET: &str = "fallback.css";

/// The bare page, with the island marker where the documentation goes and
/// `{base}` where the reader's mount does.
pub const TEMPLATE: &str = concat!(
    "<!doctype html>\n",
    "<html lang=\"en\">\n",
    "<head>\n",
    "<meta charset=\"utf-8\">\n",
    "<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n",
    "<title>vibe doc</title>\n",
    "<link rel=\"stylesheet\" href=\"{base}fallback.css\">\n",
    "</head>\n",
    "<body>\n",
    "<main class=\"prose\">\n",
    "<!--vibe-doc-island-->\n",
    "</main>\n",
    "<footer class=\"bare\">\n",
    "<p>This is the bare reader. The full shell — contents, reading settings, \
     the rule panel, the address for an agent — is a build product of the \
     documentation site package, and this binary carries none. \
     <code>vibe doc shell install</code> downloads the one that matches this \
     version, and asks first.</p>\n",
    "</footer>\n",
    "</body>\n",
    "</html>\n",
);

/// The stylesheet: the light half of the palette and the reading measure.
pub const STYLESHEET_CSS: &str = concat!(
    ":root{color-scheme:light;--ivory-100:#faf9f5;--ivory-200:#f0eee6;",
    "--text:#1f1e1b;--muted:#63625b;--border:#e0ddd2;--accent:#c15f3c;",
    "--measure:40rem}\n",
    "html{-webkit-text-size-adjust:100%}\n",
    "body{margin:0;background:var(--ivory-100);color:var(--text);",
    "font:16px/1.65 Georgia,'Times New Roman',serif}\n",
    ".prose{max-width:var(--measure);margin:0 auto;padding:3rem 1.25rem}\n",
    ".prose h1,.prose h2,.prose h3{line-height:1.25;margin:2em 0 .5em;font-weight:600}\n",
    ".prose h1{margin-top:0;font-size:2rem}\n",
    ".prose p,.prose li{margin:0 0 1em}\n",
    ".prose a{color:var(--accent)}\n",
    ".prose code,.prose pre{font-family:ui-monospace,'JetBrains Mono',Consolas,monospace;",
    "font-size:.9em}\n",
    ".prose pre{overflow-x:auto;padding:1rem;background:var(--ivory-200);",
    "border:1px solid var(--border);border-radius:4px}\n",
    ".prose blockquote{margin:0 0 1em;padding-left:1rem;border-left:3px solid var(--border);",
    "color:var(--muted)}\n",
    ".prose table{width:100%;border-collapse:collapse;margin:0 0 1em}\n",
    ".prose th,.prose td{border:1px solid var(--border);padding:.4rem .6rem;text-align:left}\n",
    ".prose img{max-width:100%;height:auto}\n",
    ".bare{max-width:var(--measure);margin:0 auto;padding:0 1.25rem 3rem;",
    "color:var(--muted);font-size:.875rem}\n",
    "@media(prefers-color-scheme:dark){:root{color-scheme:dark;--ivory-100:#14120e;",
    "--ivory-200:#1c1a15;--text:#f4f1e8;--muted:#a8a197;--border:#2e2a22;--accent:#d97757}}\n",
);

/// The template with the reader's base substituted in.
///
/// ```
/// use vibe_doc_shell::fallback;
/// let page = fallback::template("/doc/");
/// assert!(page.contains("href=\"/doc/fallback.css\""));
/// assert!(!page.contains("<script"));
/// ```
pub fn template(base: &str) -> String {
    TEMPLATE.replace("{base}", base)
}
