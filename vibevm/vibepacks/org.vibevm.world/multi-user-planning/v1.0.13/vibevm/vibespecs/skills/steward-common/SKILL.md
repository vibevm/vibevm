---
name: steward-common
description: Find the steward home and the context that belongs to the current repository (read-only), or, on explicit request, write a portable schema-2 binding for an existing context. Shared by the other steward skills; use it whenever a context must be located or bound.
---

<status stage="impl" state="done"/>

# Steward common {#root}

@fact:COMMON-SKILL-SCOPE `scripts/steward_home.py` is the one shared helper
for the steward home, repository identity, binding files and context resolution
defined by
`spec://org.vibevm.world/multi-user-planning/flows/multi-user-planning/user-local-state#CONTEXT-RESOLUTION`.
The goal renderer and the tree renderer load it by path; it needs Python 3.11+
and Git, and no third-party module. @status:impl/done

@fact:COMMON-SKILL-HOME The steward home is `$VIBE_SETTINGS/steward` when that
variable is set and non-empty, otherwise `~/.vibe/steward`. `resolve_context.py`
accepts `--home` to override it, as do `render_goal.py` and `render_tree.py`
with `--resolve`. `bind_context.py` takes the home from its `--context`
directory, which must lie in that home's `contexts/` directory. @status:impl/done

## Resolve {#resolve}

@fact:COMMON-SKILL-RESOLVE Run the resolver inside the checkout. It never
creates, repairs or rebinds anything: @status:impl/done

```text
python -B scripts/resolve_context.py [--home <steward home>] [--repo <checkout>] [--context-id <id>]
```

@fact:COMMON-SKILL-RESOLVE-OUTPUT It prints one JSON object with `status`
(`found`, `ambiguous`, `none` or `error`), `context_id`, `context_dir`, the
matching `rule`, `candidates` and any `invalid` bindings it skipped. Exit 0
found, 1 none, 2 ambiguous, 3 error (for example a directory outside Git, or a shallow clone, which has
no stable repository identity).
Report the candidates of an ambiguous result to the owner; never pick one.
@status:impl/done

## Bind {#bind}

@fact:COMMON-SKILL-BIND Only on the owner's explicit request, write a schema-2
binding for an existing context directory: @status:impl/done

```text
python -B scripts/bind_context.py --context <steward home>/contexts/<context-id> --repo <checkout> [--default | --take-default] [--revision <selector>]... [--project-name <name>]
```

@fact:COMMON-SKILL-BIND-RULES A schema-1 binding is converted by keeping
`context_id` and `project_name`, computing `repository_id` from `--repo`, and
moving the old path tuple into one `[[seen]]` entry. `--revision` replaces the
stored selectors; without it they are kept. A request for `default = true` is
refused while another context of the same repository holds it, unless
`--take-default` moves the default here. A schema-2 binding of another
repository is refused rather than silently rebound. @status:impl/done
