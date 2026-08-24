# sqlx 0.8.x specific guidance

sqlx-0.8-specific query-builder, migration, and connection-pool
conventions. Activates whenever the project graph carries any
`pkg:cargo/...` describes — the alpha package itself binds to
`pkg:cargo/sqlx@0.8.0` so this fires by default once alpha is
installed.
