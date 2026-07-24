<status stage="spec" state="done" comment="fixture doc marker"/>

# Foreign grammars must stay opaque {#root}

<status stage="impl" state="work"/>

This paragraph cites @spec://vibevm/modules/vibe-resolver/PROP-003#conditional-deps
in place, and the scanner must not mistake it for a shorthand. @impl

#use spec://org.vibevm.world/wal/flows/wal/WAL-PROTOCOL#root pulls a dependency;
#embed spec://x/y#z splices a node; #source spec://a/b#c declares realization. @spec/done

<!-- REVIEW: a conflict marker that must stay untouched -->

Fenced code is never scanned: @doc/done

```markdown
<status stage="idea" state="plan"/>
@test/plan
@spec://not/a/shorthand
```

Inline code like `<status stage="idea" state="plan"/>` and `@test` is also
opaque, while the paragraph itself is marked. @doc/plan

A wrapped fragment: <status stage="test" state="plan">only these words are
scoped</status> — and the paragraph carries its own closing marker. @impl/done
