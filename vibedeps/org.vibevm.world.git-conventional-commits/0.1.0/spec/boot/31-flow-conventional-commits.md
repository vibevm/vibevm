# Flow: Conventional Commits {#root}

<status stage="impl" state="done"/>

@fact:EVERY-COMMIT-MESSAGE-FOLLOWS-CONVENTIONAL-COMMITS Every commit message follows the [Conventional Commits](https://www.conventionalcommits.org/)
specification: a **typed header** and a body that explains *why*. @status:impl/done

## Header {#header}

```
type(scope): short imperative subject line
```

- @fact:HEADER-SUBJECT-LENGTH-MOOD-AND-CASE Subject **≤ 60 characters** (hard limit 72), imperative mood, lowercase after the prefix. @status:impl/done
- @fact:HEADER-THE-ALLOWED-TYPE-SET `type` is one of `feat` `fix` `chore` `docs` `build` `test` `refactor` `perf` `style`
  `ci` `revert`. @status:impl/done
- @fact:HEADER-SCOPE-IS-THE-NARROWEST-ACCURATE-SUBSYSTEM `scope` names the **narrowest accurate** subsystem (a crate, package, module, or area). @status:impl/done

## Body {#body}

@fact:BODY-A-BLANK-LINE-THEN-A-FREE-FORM-WHY A blank line after the subject, then a free-form body that answers *why*, not *what* — the
diff already shows what changed. @status:impl/done

@fact:CITE-SPEC-URIS-WHERE-RELEVANT Cite `spec://…` URIs where relevant. @status:impl/done

@fact:sibling-document-pointers Full format — the
allowed-type table, scope rules, body structure, worked examples, and anti-patterns — is in
@spec://org.vibevm.world/git-conventional-commits/flows/conventional-commits/conventional-commits#root. @status:impl/done

## Never {#never}

- @fact:NEVER-SUMMARISE-WHAT-CHANGED Never write a subject that summarises *what* changed — write *why*. @status:impl/done
- @fact:NEVER-CAPITALISE-OR-OMIT-THE-TYPE Never capitalise the first word after the `type(scope):` prefix, and never omit the type. @status:impl/done

## Note — format is not atomicity {#format-is-not-atomicity}

@fact:FORMAT-DOES-NOT-ENFORCE-ATOMICITY Conventional Commits is the message **format**; it does not by itself enforce **atomicity**
(one commit = one logical idea). @status:impl/done

@fact:A-VALID-MESSAGE-CAN-STILL-VIOLATE-THE-ATOMIC-RULE A `feat: add foo, bar, and baz` message is valid Conventional
Commits *and* a violation of the atomic rule. @status:impl/done

@fact:ATOMICITY-IS-THE-SEPARATE-FLOW-AND-THE-TWO-RUN-TOGETHER Atomicity is the separate `git-atomic-commits` flow;
the two run together. @status:impl/done
