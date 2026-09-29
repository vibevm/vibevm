# Review rules {#root}

<status stage="spec" state="done"/>

The rules a reviewer holds every change to. Each rule is one anchored
fact with a status.

## One idea per change {#one-idea}

@fact:ONE-IDEA A change under review carries one idea, named in its first line. @status:spec/done

## Tests before fixes {#tests-first}

@fact:TESTS-FIRST A change that alters behaviour carries a test that fails without it. @status:spec/done
