//! The step template grammar (`PROP-065#templates`): the one-pass grammar
//! of `PROP-059#launcher-paths` extended with `$work`, `$sources`, `$out`,
//! `$package`, `$feature:ID` and `$param:ID`. Expansion never evaluates a shell; the
//! caller supplies every value.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#templates");

use super::InstallerOptionDecl;

impl InstallerOptionDecl {
    /// Check template references without expanding values or reading files.
    /// Secret parameters are execution inputs, not framework template values.
    ///
    /// ```
    /// use vibe_core::manifest::{InstallerOptionDecl, ParameterDecl};
    /// let mut option: InstallerOptionDecl = toml::from_str(
    ///     "id='main'\nname='Main'\nplatform='linux'\nplatform_name='Linux'\narch='x86_64'"
    /// )?;
    /// option.parameters.push(ParameterDecl { id: "token".into(), secret: true, ..Default::default() });
    /// assert!(option.validate_step_template("$param:token").is_err());
    /// assert!(option.validate_step_template("$$param:token").is_ok());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#SECRET-TEMPLATES")]
    pub fn validate_step_template(&self, value: &str) -> Result<(), String> {
        expand_step_template(value, &mut |var| match var {
            StepVar::Feature(id) => {
                let known = self.configuration.as_ref().map_or_else(
                    || self.features.iter().any(|f| f.id == id),
                    |c| c.features.iter().any(|f| f.id == id),
                );
                if known { Ok(String::new()) } else { Err(format!("`$feature:{id}` names an undeclared feature")) }
            }
            StepVar::Param(id) => {
                let parameters = self.configuration.as_ref().map_or(self.parameters.as_slice(), |c| c.parameters.as_slice());
                match parameters.iter().find(|p| p.id == id) {
                    Some(parameter) if parameter.secret => Err(format!("`$param:{id}` refers to a secret parameter; use the recipe/shell parameter environment instead")),
                    Some(_) => Ok(String::new()),
                    None => Err(format!("`$param:{id}` names an undeclared parameter")),
                }
            }
            StepVar::Named(_) => Ok(String::new()),
        }).map(|_| ())
    }

    /// Validate inline or file-backed prompt prose with the same reference law.
    /// Newlines, carriage returns and tabs are valid prose controls.
    ///
    /// ```
    /// use vibe_core::manifest::InstallerOptionDecl;
    /// let option: InstallerOptionDecl = toml::from_str(
    ///     "id='main'\nname='Main'\nplatform='linux'\nplatform_name='Linux'\narch='x86_64'"
    /// )?;
    /// assert!(option.validate_prompt_template("A\n\t$work").is_ok());
    /// assert!(option.validate_prompt_template("A\u{7}").is_err());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#SECRET-TEMPLATES")]
    pub fn validate_prompt_template(&self, value: &str) -> Result<(), String> {
        if value
            .chars()
            .any(|ch| ch.is_control() && !matches!(ch, '\n' | '\r' | '\t'))
        {
            return Err(
                "prompt must not contain control characters other than line breaks and tabs".into(),
            );
        }
        self.validate_step_template(value)
    }
}

/// One variable reference of a step template.
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#templates")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepVar<'a> {
    /// `$work`, `$sources`, `$out`, `$package`, `$vibe_home`, `$app_root`, …
    Named(&'a str),
    /// `$feature:ID`, which expands to `true` or `false`.
    Feature(&'a str),
    /// `$param:ID`, the canonical value of a parameter (`PROP-065#parameters`).
    Param(&'a str),
}

/// The plain variable names a step template may reference; `$app_bin` is
/// `$app_root/bin`, known like `$app_root` only to `finish` steps.
const STEP_VARIABLES: [&str; 7] = [
    "work",
    "sources",
    "out",
    "package",
    "vibe_home",
    "app_root",
    "app_bin",
];

/// Expand `template` in one pass: `$name`, `${name}`, `$feature:ID`,
/// `${feature:ID}`, `$param:ID`, `${param:ID}` and `$$` for a literal
/// dollar.
///
/// ```
/// use vibe_core::manifest::{StepVar, expand_step_template};
///
/// let expanded = expand_step_template("$out/doc-${feature:fixes}-$$", &mut |var| match var {
///     StepVar::Named("out") => Ok("/w/out".into()),
///     StepVar::Feature("fixes") => Ok("true".into()),
///     StepVar::Param("lang") => Ok("en".into()),
///     other => Err(format!("{other:?}")),
/// });
/// assert_eq!(expanded.unwrap(), "/w/out/doc-true-$");
/// let param = expand_step_template("$param:lang/x", &mut |_| Ok("en".into()));
/// assert_eq!(param.unwrap(), "en/x");
/// ```
pub fn expand_step_template(
    template: &str,
    value: &mut dyn FnMut(StepVar<'_>) -> Result<String, String>,
) -> Result<String, String> {
    let mut result = String::new();
    let mut rest = template;
    while let Some(position) = rest.find('$') {
        result.push_str(&rest[..position]);
        rest = &rest[position + 1..];
        if let Some(after) = rest.strip_prefix('$') {
            result.push('$');
            rest = after;
            continue;
        }
        let (name, after) = if let Some(braced) = rest.strip_prefix('{') {
            let end = braced
                .find('}')
                .ok_or_else(|| "unterminated template variable".to_owned())?;
            (&braced[..end], &braced[end + 1..])
        } else {
            let mut end = rest
                .find(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_')
                .unwrap_or(rest.len());
            if matches!(&rest[..end], "feature" | "param") && rest[end..].starts_with(':') {
                end += 1;
                end += rest[end..]
                    .find(|ch: char| {
                        !ch.is_ascii_lowercase() && !ch.is_ascii_digit() && ch != '-' && ch != '_'
                    })
                    .unwrap_or(rest.len() - end);
            }
            (&rest[..end], &rest[end..])
        };
        let var = match (name.strip_prefix("feature:"), name.strip_prefix("param:")) {
            (Some(id), _) if !id.is_empty() => StepVar::Feature(id),
            (_, Some(id)) if !id.is_empty() => StepVar::Param(id),
            (Some(_), _) => return Err("`$feature:` needs a feature id".into()),
            (_, Some(_)) => return Err("`$param:` needs a parameter id".into()),
            _ if STEP_VARIABLES.contains(&name) => StepVar::Named(name),
            _ => return Err(format!("unknown template variable `${name}`")),
        };
        result.push_str(&value(var)?);
        rest = after;
    }
    result.push_str(rest);
    Ok(result)
}
