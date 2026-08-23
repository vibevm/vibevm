# Sync-from-Code — the WAL binding {#root}

@fact:SYNC-WAL-BINDING With `flow:wal` installed, the durable session state
of this flow's rules is `spec/WAL.md`: the top-down river reads
head → WAL → spec → code, and a temporary hack is recorded in the WAL. @status:impl/done
