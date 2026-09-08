use super::*;

impl Contract {
    pub fn validate(&self) -> Result<(), ScrapeError> {
        if self.schema != 1 {
            return invalid("schema must equal 1");
        }
        validate_token(&self.id, "contract id")?;
        if self.classify.is_empty() {
            return invalid("at least one classify row is required");
        }
        if self.assertions.is_empty() {
            return invalid("at least one assertion is required");
        }
        if self.healthcheck.is_empty() {
            return invalid("at least one healthcheck is required");
        }
        if self.scope.closed_roots.is_empty() {
            return invalid("scope.closed_roots must be nonempty");
        }
        if self.health.parallel {
            return invalid("health.parallel must be false in schema 1");
        }
        if self.health.max_stdout_bytes == 0
            || self.health.max_stderr_bytes == 0
            || self.health.max_result_bytes == 0
        {
            return invalid("health output/result caps must be positive");
        }
        if self.health.max_stdout_bytes > MAX_HEALTH_STREAM_BYTES
            || self.health.max_stderr_bytes > MAX_HEALTH_STREAM_BYTES
            || self.health.max_result_bytes > MAX_HEALTH_RESULT_BYTES
        {
            return invalid(format!(
                "health caps exceed engine maxima (stdout/stderr {MAX_HEALTH_STREAM_BYTES}, result {MAX_HEALTH_RESULT_BYTES})"
            ));
        }
        if self.health.termination_grace_seconds == 0 {
            return invalid("health.termination_grace_seconds must be positive");
        }
        if !self.healthcheck.iter().any(|check| check.when().is_none()) {
            return invalid("at least one healthcheck must be unconditional");
        }

        let mut ids = BTreeSet::new();
        for rule in &self.classify {
            unique_id(&mut ids, rule.id())?;
            validate_nonempty_globs(rule.patterns(), "classify.patterns")?;
            match rule {
                ClassifyRule::Keep { owner, .. } if *owner != Owner::Project => {
                    return invalid("keep requires owner = project");
                }
                ClassifyRule::Delete { owner, .. } | ClassifyRule::Generated { owner, .. }
                    if *owner != Owner::Vibe =>
                {
                    return invalid("delete/generated require owner = vibe");
                }
                _ => {}
            }
            reject_git_selection(rule.patterns(), &[])?;
        }
        validate_unique_literals(&self.scope.closed_roots, "scope.closed_roots")?;
        reject_git_literals(&self.scope.closed_roots, "scope.closed_roots")?;

        let mut baselines = BTreeMap::new();
        for baseline in &self.baseline {
            PortablePath::parse(&baseline.path)?;
            reject_git_literal(&baseline.path, "baseline.path")?;
            validate_digest(&baseline.sha256, "baseline.sha256")?;
            if baselines
                .insert(baseline.path.as_str(), baseline.sha256.as_str())
                .is_some()
            {
                return invalid(format!("duplicate baseline path `{}`", baseline.path));
            }
        }

        for rewrite in &self.rewrite {
            unique_id(&mut ids, rewrite.id())?;
            validate_rewrite(rewrite)?;
        }
        for relocation in &self.relocate {
            unique_id(&mut ids, &relocation.id)?;
            PortablePath::parse(&relocation.from)?;
            PortablePath::parse(&relocation.to)?;
            reject_git_literal(&relocation.from, "relocate.from")?;
            reject_git_literal(&relocation.to, "relocate.to")?;
            if relocation.from == relocation.to {
                return invalid(format!(
                    "relocation `{}` has identical source and destination",
                    relocation.id
                ));
            }
        }
        validate_relocation_graph(&self.relocate)?;
        for assertion in &self.assertions {
            unique_id(&mut ids, assertion.id())?;
            validate_assertion(assertion)?;
        }
        for check in &self.healthcheck {
            unique_id(&mut ids, check.id())?;
            validate_healthcheck(check, self.health.baseline)?;
        }
        Ok(())
    }
}

fn validate_rewrite(rule: &RewriteRule) -> Result<(), ScrapeError> {
    match rule {
        RewriteRule::ManagedBlockRemoveV1 { paths, marker, .. } => {
            validate_unique_literals(paths, "rewrite.paths")?;
            reject_git_literals(paths, "rewrite.paths")?;
            validate_token(marker, "managed marker")?;
            if marker != "vibevm" {
                return invalid(format!(
                    "managed marker `{marker}` is not a registered schema-1 provider identity"
                ));
            }
        }
        RewriteRule::RustSpecmarkStripV1 {
            patterns,
            exclude,
            forms,
            matches,
            ..
        } => {
            validate_nonempty_globs(patterns, "rewrite.patterns")?;
            validate_globs(exclude, "rewrite.exclude")?;
            reject_git_selection(patterns, exclude)?;
            if forms.is_empty() || forms.iter().collect::<BTreeSet<_>>().len() != forms.len() {
                return invalid("rust forms must be nonempty and unique");
            }
            if *matches == SetMatches::ExactlyOne {
                return invalid("rust-specmark-strip-v1 does not admit exactly-one");
            }
        }
        RewriteRule::CargoPackageRemoveV1 {
            manifests,
            package,
            aliases,
            ..
        } => {
            validate_nonempty_globs(manifests, "rewrite.manifests")?;
            nonempty(package, "Cargo package")?;
            validate_optional_unique_nonempty(aliases, "Cargo aliases")?;
            reject_git_selection(manifests, &[])?;
        }
        RewriteRule::NodePackageRemoveV1 {
            package_json,
            lockfile,
            packages,
            script_paths,
            config_paths,
            ..
        } => {
            PortablePath::parse(package_json)?;
            PortablePath::parse(lockfile)?;
            reject_git_literal(package_json, "rewrite.package_json")?;
            reject_git_literal(lockfile, "rewrite.lockfile")?;
            validate_unique_nonempty(packages, "Node packages")?;
            validate_component_paths(script_paths, "script_paths")?;
            validate_component_paths(config_paths, "config_paths")?;
        }
        RewriteRule::GoModuleRemoveV1 {
            go_mod,
            go_sum,
            modules,
            ..
        } => {
            PortablePath::parse(go_mod)?;
            if let Some(path) = go_sum {
                PortablePath::parse(path)?;
            }
            reject_git_literal(go_mod, "rewrite.go_mod")?;
            if let Some(path) = go_sum {
                reject_git_literal(path, "rewrite.go_sum")?;
            }
            validate_unique_nonempty(modules, "Go modules")?;
        }
        RewriteRule::TomlArrayValuesRemoveV1 {
            path,
            table,
            key,
            values,
            ..
        } => {
            PortablePath::parse(path)?;
            reject_git_literal(path, "rewrite.path")?;
            validate_components(table, "TOML table")?;
            nonempty(key, "TOML key")?;
            validate_unique_nonempty(values, "TOML values")?;
        }
        RewriteRule::TypeScriptSpecCommentsStripV1 {
            patterns, exclude, ..
        }
        | RewriteRule::GoSpecDirectivesStripV1 {
            patterns, exclude, ..
        } => {
            validate_nonempty_globs(patterns, "rewrite.patterns")?;
            validate_globs(exclude, "rewrite.exclude")?;
            reject_git_selection(patterns, exclude)?;
        }
        RewriteRule::JsonMemberRemoveV1 {
            path,
            object,
            members,
            ..
        } => {
            PortablePath::parse(path)?;
            reject_git_literal(path, "rewrite.path")?;
            validate_components(object, "JSON object")?;
            validate_unique_nonempty(members, "JSON members")?;
        }
        RewriteRule::TextExactReplaceV1 {
            path,
            sha256,
            before,
            occurrences,
            ..
        } => {
            PortablePath::parse(path)?;
            reject_git_literal(path, "rewrite.path")?;
            validate_digest(sha256, "rewrite.sha256")?;
            if before.is_empty() {
                return invalid("text exact-replace before string must be nonempty");
            }
            if *occurrences == 0 {
                return invalid("text exact-replace occurrences must be positive");
            }
        }
    }
    Ok(())
}

fn validate_assertion(assertion: &Assertion) -> Result<(), ScrapeError> {
    match assertion {
        Assertion::PathsAbsentV1 { patterns, .. } => {
            validate_nonempty_globs(patterns, "assert.patterns")?;
            reject_git_selection(patterns, &[])
        }
        Assertion::TextLiteralAbsentV1 {
            patterns, needles, ..
        } => {
            validate_nonempty_globs(patterns, "assert.patterns")?;
            reject_git_selection(patterns, &[])?;
            validate_unique_nonempty(needles, "assert.needles")
        }
        Assertion::CargoPathPrefixAbsentV1 {
            manifests,
            prefixes,
            ..
        } => {
            validate_nonempty_globs(manifests, "assert.manifests")?;
            reject_git_selection(manifests, &[])?;
            validate_unique_nonempty(prefixes, "assert.prefixes")
        }
        Assertion::LanguageMetadataAbsentV1 { patterns, .. } => {
            validate_nonempty_globs(patterns, "assert.patterns")?;
            reject_git_selection(patterns, &[])
        }
        Assertion::DependencyIdentitiesAbsentV1 {
            manifests,
            identities,
            ..
        } => {
            validate_nonempty_globs(manifests, "assert.manifests")?;
            reject_git_selection(manifests, &[])?;
            validate_unique_nonempty(identities, "assert.identities")
        }
    }
}

fn validate_healthcheck(check: &Healthcheck, baseline: BaselineMode) -> Result<(), ScrapeError> {
    let (root, timeout, when, network) = match check {
        Healthcheck::Cargo {
            root,
            timeout_seconds,
            when,
            network,
            ..
        }
        | Healthcheck::Npm {
            root,
            timeout_seconds,
            when,
            network,
            ..
        }
        | Healthcheck::Maven {
            root,
            timeout_seconds,
            when,
            network,
            ..
        }
        | Healthcheck::PythonPip {
            root,
            timeout_seconds,
            when,
            network,
            ..
        } => (root, timeout_seconds, when, *network),
        Healthcheck::Custom {
            root,
            timeout_seconds,
            when,
            network,
            ..
        } => (root, timeout_seconds, when, Some(*network)),
    };
    validate_root(root)?;
    if *timeout == 0 {
        return invalid(format!(
            "healthcheck `{}` timeout must be positive",
            check.id()
        ));
    }
    if let Some(when) = when {
        PortablePath::parse(&when.path_exists)?;
    }
    if matches!(network, Some(NetworkPolicy::ToolOffline))
        && matches!(check, Healthcheck::Custom { .. })
    {
        return invalid("custom health cannot use tool-offline network policy");
    }
    match check {
        Healthcheck::Npm {
            build_script,
            typecheck_script,
            tests,
            test_script,
            lockfile,
            ..
        } => {
            PortablePath::parse(lockfile)?;
            if build_script.as_ref().is_some_and(|s| s.is_empty())
                || typecheck_script.as_ref().is_some_and(|s| s.is_empty())
            {
                return invalid("npm script names must be nonempty");
            }
            if build_script.is_some() == typecheck_script.is_some() {
                return invalid("npm requires exactly one build_script or typecheck_script");
            }
            validate_test_runner(*tests, test_script.as_deref(), "npm test_script")?;
        }
        Healthcheck::Maven { goal, .. } => nonempty(goal, "Maven goal")?,
        Healthcheck::PythonPip {
            interpreter,
            source_roots,
            tests,
            test_runner,
            ..
        } => {
            nonempty(interpreter, "Python interpreter")?;
            validate_unique_literals(source_roots, "Python source_roots")?;
            validate_test_runner(*tests, test_runner.as_deref(), "Python test_runner")?;
        }
        Healthcheck::Custom {
            source,
            snapshot,
            interpreter,
            argv,
            protocol,
            reads,
            writes,
            ..
        } => {
            PortablePath::parse(source)?;
            nonempty(interpreter, "custom interpreter")?;
            validate_nonempty_globs(snapshot, "custom snapshot")?;
            validate_globs(reads, "custom reads")?;
            validate_globs(writes, "custom writes")?;
            let mut contains_source = false;
            for pattern in snapshot {
                contains_source |= Glob::parse(pattern)?.matches(source);
            }
            if !contains_source {
                return invalid("custom snapshot must contain its source");
            }
            validate_argv(argv, *protocol)?;
            if baseline == BaselineMode::NoRegression
                && *protocol != CustomProtocol::VibeHealthJsonV1
            {
                return invalid("no-regression requires structured custom health");
            }
        }
        Healthcheck::Cargo { .. } => {}
    }
    if let Healthcheck::Cargo { features, .. } = check {
        validate_optional_unique_nonempty(features, "Cargo features")?;
    }
    if let Healthcheck::Custom { writes, .. } = check {
        reject_git_selection(writes, &[])?;
    }
    if baseline == BaselineMode::NoRegression
        && !matches!(
            check,
            Healthcheck::Custom {
                protocol: CustomProtocol::VibeHealthJsonV1,
                ..
            }
        )
    {
        return invalid("no-regression admits only structured custom healthchecks");
    }
    Ok(())
}

fn validate_argv(argv: &[String], protocol: CustomProtocol) -> Result<(), ScrapeError> {
    let allowed = ["{root}", "{phase}", "{scratch}", "{result}"];
    let result_count = argv.iter().filter(|arg| arg.as_str() == "{result}").count();
    for arg in argv {
        if (arg.contains('{') || arg.contains('}')) && !allowed.contains(&arg.as_str()) {
            return invalid(format!("invalid or embedded custom placeholder `{arg}`"));
        }
    }
    match protocol {
        CustomProtocol::VibeHealthJsonV1 if result_count != 1 => {
            invalid("JSON custom protocol requires {result} exactly once")
        }
        CustomProtocol::ExitCode if result_count != 0 => {
            invalid("exit-code custom protocol forbids {result}")
        }
        _ => Ok(()),
    }
}

fn validate_test_runner(
    mode: TestsMode,
    value: Option<&str>,
    field: &str,
) -> Result<(), ScrapeError> {
    if mode == TestsMode::Skip {
        if value.is_some() {
            return invalid(format!("{field} is forbidden when tests = skip"));
        }
    } else {
        nonempty(value.unwrap_or(""), field)?;
    }
    Ok(())
}

fn validate_relocation_graph(rows: &[Relocation]) -> Result<(), ScrapeError> {
    for (index, left) in rows.iter().enumerate() {
        for right in &rows[index + 1..] {
            if paths_overlap(&left.from, &right.from)
                || paths_overlap(&left.to, &right.to)
                || paths_overlap(&left.from, &right.to)
                || paths_overlap(&left.to, &right.from)
            {
                return invalid(format!(
                    "relocations `{}` and `{}` overlap",
                    left.id, right.id
                ));
            }
        }
        if left.to.starts_with(&(left.from.clone() + "/")) {
            return invalid(format!("relocation `{}` moves into itself", left.id));
        }
    }
    Ok(())
}

fn paths_overlap(a: &str, b: &str) -> bool {
    a == b || a.starts_with(&(b.to_owned() + "/")) || b.starts_with(&(a.to_owned() + "/"))
}

fn reject_git_selection(patterns: &[String], excludes: &[String]) -> Result<(), ScrapeError> {
    let excluded_all = excludes.iter().any(|p| p == ".git/**" || p == "**");
    for pattern in patterns {
        let glob = Glob::parse(pattern)?;
        if glob.can_match_git() && !excluded_all {
            return invalid(format!(
                "pattern `{pattern}` can select protected .git internals without a complete .git/** exclusion"
            ));
        }
    }
    Ok(())
}

fn validate_nonempty_globs(values: &[String], field: &str) -> Result<(), ScrapeError> {
    if values.is_empty() {
        return invalid(format!("{field} must be nonempty"));
    }
    validate_globs(values, field)
}

fn validate_globs(values: &[String], field: &str) -> Result<(), ScrapeError> {
    let mut seen = BTreeSet::new();
    for value in values {
        Glob::parse(value)?;
        if !seen.insert(value) {
            return invalid(format!("duplicate {field} entry `{value}`"));
        }
    }
    Ok(())
}

fn validate_unique_literals(values: &[String], field: &str) -> Result<(), ScrapeError> {
    if values.is_empty() {
        return invalid(format!("{field} must be nonempty"));
    }
    let mut seen = BTreeSet::new();
    for value in values {
        PortablePath::parse(value)?;
        if !seen.insert(value) {
            return invalid(format!("duplicate {field} entry `{value}`"));
        }
    }
    Ok(())
}

fn validate_component_paths(values: &[Vec<String>], field: &str) -> Result<(), ScrapeError> {
    let mut seen = BTreeSet::new();
    for value in values {
        validate_components(value, field)?;
        if !seen.insert(value) {
            return invalid(format!("duplicate {field} entry"));
        }
    }
    Ok(())
}

fn validate_components(values: &[String], field: &str) -> Result<(), ScrapeError> {
    if values.is_empty() {
        return invalid(format!("{field} must be nonempty"));
    }
    for value in values {
        if value.is_empty() || value == "." || value == ".." || value.contains(['/', '\\', ':']) {
            return invalid(format!("invalid {field} component `{value}`"));
        }
    }
    Ok(())
}

fn validate_unique_nonempty(values: &[String], field: &str) -> Result<(), ScrapeError> {
    if values.is_empty() {
        return invalid(format!("{field} must be nonempty"));
    }
    let mut seen = BTreeSet::new();
    for value in values {
        nonempty(value, field)?;
        if !seen.insert(value) {
            return invalid(format!("duplicate {field} value `{value}`"));
        }
    }
    Ok(())
}

fn validate_optional_unique_nonempty(values: &[String], field: &str) -> Result<(), ScrapeError> {
    let mut seen = BTreeSet::new();
    for value in values {
        nonempty(value, field)?;
        if !seen.insert(value) {
            return invalid(format!("duplicate {field} value `{value}`"));
        }
    }
    Ok(())
}

fn reject_git_literals(values: &[String], field: &str) -> Result<(), ScrapeError> {
    for value in values {
        reject_git_literal(value, field)?;
    }
    Ok(())
}

fn reject_git_literal(value: &str, field: &str) -> Result<(), ScrapeError> {
    if value == ".git" || value.starts_with(".git/") {
        invalid(format!("{field} addresses protected .git metadata"))
    } else {
        Ok(())
    }
}
