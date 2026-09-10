//! `#[spec(deviates = "…", reason = "…")]` deviation scope. The verb
//! detection and the `reason` extraction share ONE token-stream walk over the
//! attribute — never two parsers of the same attribute that could drift.
//! [`DeviationStack`] is the fn-grain scope the AST visit pushes and pops;
//! the three deviation facts (`UnsafeUse` / `UnwrapUse` / `EnvRead`) read
//! `in_deviation` and the active `reason` off its top.

/// The `spec(...)` attribute whose first token (the verb) is `deviates`, as a
/// borrowed token stream. specmark parses verb-first, so only the `deviates`
/// verb matches — `spec(implements = …)` does not. The single source of truth
/// the verb-presence flag and the reason text both derive from.
fn spec_deviates_tokens(attrs: &[syn::Attribute]) -> Option<&proc_macro2::TokenStream> {
    attrs.iter().find_map(|a| {
        if a.path().segments.last().is_none_or(|s| s.ident != "spec") {
            return None;
        }
        let syn::Meta::List(list) = &a.meta else {
            return None;
        };
        match list.tokens.clone().into_iter().next() {
            Some(proc_macro2::TokenTree::Ident(i)) if i == "deviates" => Some(&list.tokens),
            _ => None,
        }
    })
}

/// The deviation scope of an item's attributes: `Some(reason)` when it carries
/// `#[spec(deviates = …)]` — `reason` is the attribute's `reason = "…"` text,
/// or `None` when it deviated on record without that key (a legitimate
/// degenerate case). `None` entirely when the item does not deviate. One
/// token-stream walk answers both the verb-presence and the reason (the second
/// key of the one attribute, not a second parser beside it); the reason value
/// is parsed as a `syn::LitStr`, so escapes (`\"`, `\t`, …) unescape as rustc
/// would, and a non-string value yields `None`.
fn deviation_scope(attrs: &[syn::Attribute]) -> Option<Option<String>> {
    let tokens = spec_deviates_tokens(attrs)?;
    let mut reason = None;
    let mut it = tokens.clone().into_iter();
    while let Some(t) = it.next() {
        if let proc_macro2::TokenTree::Ident(i) = t
            && i == "reason"
            && matches!(it.next(), Some(proc_macro2::TokenTree::Punct(p)) if p.as_char() == '=')
            && let Some(proc_macro2::TokenTree::Literal(l)) = it.next()
            && let syn::Lit::Str(s) = syn::Lit::new(l)
        {
            reason = Some(s.value());
        }
    }
    Some(reason)
}

/// The fn-grain deviation scope: a stack of each enclosing
/// `#[spec(deviates = …)]` fn's reason (`Some(text)` when the attribute gave
/// one, `None` when it deviated without prose), so nested deviation fns carry
/// the NEAREST reason. The v11 generalization of the v4 `deviating_depth`
/// counter: `in_deviation` is `!is_empty()`, and the active `reason` rides on
/// the top.
#[derive(Default)]
pub(crate) struct DeviationStack(Vec<Option<String>>);

impl DeviationStack {
    /// Enter the deviation scope of `attrs` if it carries `#[spec(deviates)]`,
    /// returning whether a scope was entered so the caller can pair this with
    /// [`leave`](Self::leave).
    pub(crate) fn enter_if_deviates(&mut self, attrs: &[syn::Attribute]) -> bool {
        let Some(reason) = deviation_scope(attrs) else {
            return false;
        };
        self.0.push(reason);
        true
    }

    /// Leave the nearest deviation scope. Paired with
    /// [`enter_if_deviates`](Self::enter_if_deviates).
    pub(crate) fn leave(&mut self) {
        self.0.pop();
    }

    /// True while inside a `#[spec(deviates = …)]` fn (fn-grain only — a
    /// deviates edge on an impl/struct/mod records a different deviation and
    /// grants no amnesty).
    pub(crate) fn in_deviation(&self) -> bool {
        !self.0.is_empty()
    }

    /// The reason text of the nearest enclosing deviation fn, or `None` when
    /// there is no scope or the scope's attribute carried no `reason` key.
    pub(crate) fn reason(&self) -> Option<String> {
        self.0.last().and_then(|r| r.clone())
    }
}
