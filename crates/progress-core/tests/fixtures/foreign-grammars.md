<status stage="spec" state="done" comment="fixture doc marker"/>

# Foreign grammars must stay opaque {#root}

<status stage="impl" state="work"/>

##p-cite This paragraph cites @spec://vibevm/modules/vibe-resolver/PROP-003#conditional-deps
in place, and the scanner must not mistake it for a shorthand. @impl

##p-use #use spec://org.vibevm.world/wal/flows/wal/WAL-PROTOCOL#root pulls a dependency;
#embed spec://x/y#z splices a node; #source spec://a/b#c declares realization. @spec/done

<!-- REVIEW: a conflict marker that must stay untouched -->

##p-fence Fenced code is never scanned: @doc/done

```markdown
<status stage="idea" state="plan"/>
@test/plan
@spec://not/a/shorthand
##NOT-AN-ID a fenced line is never a fact
```

##p-inline Inline code like `<status stage="idea" state="plan"/>` and `@test` is also
opaque, and `##not-an-id` in code is opaque too, while the paragraph
itself is marked. @doc/plan

##p-wrap A wrapped fragment: <status stage="test" state="plan">only these words are
scoped</status> — and the paragraph carries its own closing marker. @impl/done

## Fact units {#facts}

##lead-rules The rules of the fact grammar, one fact each: @spec/done

1. ##RULE-001 The first rule is a shorthand-marked numbered item. @freeze/done
2. ##RULE-002 The second rule carries an XML point marker. <status stage="impl" state="work"/>
   - ##RULE-002a A nested item is a unit of its own. @idea
3. ##RULE-003 <status stage="test" state="plan">an inline wrapped fact</status> lives inside a marked item. @impl/done

##lead-table The carrier table — body cells are units: @doc/done

| Carrier | Form |
|---|---|
| ##ROW-PKGREF pkgref @impl/done | one coordinate @impl/done |
| spec authority @spec/done | first path segment @spec/done |
