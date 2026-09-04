//! The schema-1 portable path and glob grammar.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-056#IMPL-A");

use crate::model::ScrapeError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortablePath(String);

impl PortablePath {
    pub fn parse(value: &str) -> Result<Self, ScrapeError> {
        validate_prefix(value)?;
        if value.split('/').any(invalid_literal_component) {
            return Err(ScrapeError::contract(format!(
                "invalid portable path `{value}`"
            )));
        }
        if value.contains('*') {
            return Err(ScrapeError::contract(format!(
                "literal path `{value}` contains `*`"
            )));
        }
        vibe_safefs::split_relative(value).map_err(|error| {
            ScrapeError::contract(format!("invalid portable path `{value}`: {error:#}"))
        })?;
        Ok(Self(value.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Glob {
    source: String,
    components: Vec<Component>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Component {
    DoubleStar,
    Pattern(String),
}

impl Glob {
    pub fn parse(value: &str) -> Result<Self, ScrapeError> {
        validate_prefix(value)?;
        let mut components = Vec::new();
        for component in value.split('/') {
            if invalid_literal_component(component) {
                return Err(ScrapeError::contract(format!(
                    "invalid glob component in `{value}`"
                )));
            }
            if component == "**" {
                components.push(Component::DoubleStar);
                continue;
            }
            if component.contains("**")
                || component.contains(['?', '[', ']', '{', '}', '!', '\\', ':'])
            {
                return Err(ScrapeError::contract(format!(
                    "unsupported schema-1 glob `{value}`"
                )));
            }
            let literal_witness = component.replace('*', "x");
            vibe_safefs::ensure_safe_component(&literal_witness).map_err(|error| {
                ScrapeError::contract(format!(
                    "invalid portable glob component `{component}`: {error:#}"
                ))
            })?;
            components.push(Component::Pattern(component.to_owned()));
        }
        Ok(Self {
            source: value.to_owned(),
            components,
        })
    }

    pub fn as_str(&self) -> &str {
        &self.source
    }

    pub fn matches(&self, path: &str) -> bool {
        let parts = path.split('/').collect::<Vec<_>>();
        matches_components(&self.components, &parts)
    }

    /// Whether this selector's language intersects `.git` or its descendants.
    pub fn can_match_git(&self) -> bool {
        match self.components.first() {
            // A leading double-star can consume `.git`; every remaining legal
            // component pattern has at least one finite witness.
            Some(Component::DoubleStar) => true,
            Some(Component::Pattern(pattern)) => matches_component(pattern, ".git"),
            None => false,
        }
    }
}

fn validate_prefix(value: &str) -> Result<(), ScrapeError> {
    if value.is_empty()
        || value.starts_with('/')
        || value.ends_with('/')
        || value.contains(['\\', ':', '\0'])
    {
        return Err(ScrapeError::contract(format!(
            "non-portable project-relative path `{value}`"
        )));
    }
    Ok(())
}

fn invalid_literal_component(value: &str) -> bool {
    value.is_empty() || value == "." || value == ".."
}

fn matches_components(pattern: &[Component], path: &[&str]) -> bool {
    let mut reachable = vec![vec![false; path.len() + 1]; pattern.len() + 1];
    reachable[0][0] = true;
    for p in 0..pattern.len() {
        for s in 0..=path.len() {
            if !reachable[p][s] {
                continue;
            }
            match &pattern[p] {
                Component::DoubleStar => {
                    reachable[p + 1][s..].fill(true);
                }
                Component::Pattern(component)
                    if s < path.len() && matches_component(component, path[s]) =>
                {
                    reachable[p + 1][s + 1] = true
                }
                Component::Pattern(_) => {}
            }
        }
    }
    reachable[pattern.len()][path.len()]
}

fn matches_component(pattern: &str, value: &str) -> bool {
    let p = pattern.as_bytes();
    let v = value.as_bytes();
    let mut reachable = vec![vec![false; v.len() + 1]; p.len() + 1];
    reachable[0][0] = true;
    for pi in 0..p.len() {
        for vi in 0..=v.len() {
            if !reachable[pi][vi] {
                continue;
            }
            if p[pi] == b'*' {
                reachable[pi + 1][vi..].fill(true);
            } else if vi < v.len() && p[pi] == v[vi] {
                reachable[pi + 1][vi + 1] = true;
            }
        }
    }
    reachable[p.len()][v.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_component_and_recursive_stars_are_distinct() {
        assert!(Glob::parse("src/*.rs").unwrap().matches("src/lib.rs"));
        assert!(!Glob::parse("src/*.rs").unwrap().matches("src/a/lib.rs"));
        assert!(Glob::parse("src/**/*.rs").unwrap().matches("src/lib.rs"));
        assert!(Glob::parse("src/**/*.rs").unwrap().matches("src/a/lib.rs"));
    }

    #[test]
    fn rejects_every_foreign_glob_family() {
        for bad in [
            "a?", "[ab]", "{a,b}", "!a", "a\\b", "a:b", "a/***.rs", "a**b", "/a", "a//b", "a/./b",
            "a/../b",
        ] {
            assert!(Glob::parse(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn detects_git_intersection_independent_of_filename_suffix() {
        assert!(Glob::parse("**/*.go").unwrap().can_match_git());
        assert!(Glob::parse(".g*t/config").unwrap().can_match_git());
        assert!(!Glob::parse("src/**/*.go").unwrap().can_match_git());
    }
}
