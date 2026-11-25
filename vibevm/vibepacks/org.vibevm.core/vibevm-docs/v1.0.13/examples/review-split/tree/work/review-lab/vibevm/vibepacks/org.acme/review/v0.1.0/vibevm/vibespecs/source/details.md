# Review rules, the reasons {#details}

## Why one idea per change {#one-idea-why}

@fact:ONE-IDEA-WHY A change with two ideas cannot be reverted one idea at a time, and its review takes twice as long. @status:spec/done

## Why tests before fixes {#tests-first-why}

@fact:TESTS-FIRST-WHY A fix without a failing test proves nothing: the test states what was wrong. @status:spec/done

## What a reviewer checks {#checklist}

- @fact:CHECK-SCOPE The first line names the one idea, and every hunk serves it. @status:spec/done
- @fact:CHECK-TEST The test fails on the parent commit and passes on this one. @status:spec/done
