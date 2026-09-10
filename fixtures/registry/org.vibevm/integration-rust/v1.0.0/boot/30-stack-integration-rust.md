# Integration Rust stack — boot snippet

Project is targeting the Rust ecosystem via the `integration-rust`
stack. Activates Rust-specific subskills across other packages
through `if_present = ["stack:integration-rust"]` and lights up
conditional deps that probe the same key.
