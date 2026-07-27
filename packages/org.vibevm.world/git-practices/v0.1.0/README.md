# `flow:git-practices` — a repository's commit-and-push discipline, as a family {#root}

<status stage="doc" state="done" audience="user"/>

##AGG-ROLE The **git-practices** family aggregates the distinct disciplines a repository applies to its
Git history. @impl/done

##AGG-CLOSURE-PULLS-THE-WHOLE-FAMILY Each is its own installable `flow` package — adoptable alone — and this umbrella
names a tested set of them; requiring `flow:git-practices` pulls the whole family through the
dependency closure, each member contributing its own boot snippet. @impl/done

##deliberately-separate-packages-lead They are deliberately **separate packages** because they are different things: @impl/done

- ##AGG-MEMBER-CONVENTIONAL-COMMITS **`conventional-commits`** — the message *format*: `type(scope): subject`, a why-not-what body,
  the allowed-type set, scope convention, worked examples, anti-patterns. @impl/done
- ##AGG-MEMBER-ATOMIC-COMMITS **`atomic-commits`** — the *atomicity* discipline: one commit = one logical idea; when to split
  a mixed working tree, when to batch, why it matters more in a human-AI team. @impl/done

##EACH-IS-ADOPTABLE-ON-ITS-OWN A message can be valid Conventional Commits and non-atomic (`feat: add foo, bar, baz`), or atomic
without the format — so each is adoptable on its own, and the family is how a project takes the
whole posture at once. @impl/done

##THE-FAMILY-GROWS-TO-INCLUDE-ATTRIBUTION-AND-AUTONOMY The family grows to include **human-authored attribution** and **commit
autonomy** as those members land. @spec/done

##CONTENT-MINIMAL-NO-BOOT-SNIPPET-OF-ITS-OWN Content-minimal by design (PROP-028): no boot snippet of its own — the members ship theirs. @impl/done

## Install {#install}

```bash
vibe install flow:git-practices
```

## License {#license}

##license-line UPL-1.0 — see `LICENSE`. @impl/done
