# Ordinary changed-default acknowledgment — qualification candidate

Baseline: accepted governed update `49558dc40`, finite qualification contract `7cadf356c`. Test-only, no production or format change.

## Existing owners and first missing seam

Ordinary package automatic history belongs to `vibe-core/src/manifest/configuration/package.rs::PackageConfiguration` and its typed AutomaticBasis. The existing resolver `configuration.rs::ConfigurationSnapshot::evaluate_candidate` compares admitted prior data against the replacement candidate. A mismatch produces `PackageConfigurationError::ReplayMismatch { package }`; this carries only package identity, not the actual consumer/group, before/after default/dependency projection or a complete proposed subject. Selected validation withholds effects. Ordinary update's existing error transport renders a failure, not a displayable proposal that an operator can complete.

The nearest actual review owner is application-specific: `vibe-app/src/configuration/plan.rs` evaluates a historical record, asks an attended chooser using `configuration/review.rs::summary`, and supplies reviewed authority. Its evaluator accepts `ConfigurationSubmission.expected_definition`; strict `configuration/input.rs::parse_submission` and transfer admission bind the supported app input. CLI app grammar exposes `--configuration-input`. This does not implement ordinary package update review, and no app record/store is copied into the package path.

Ordinary `cli/pkg.rs::UpdateArgs` has no package proposal response input. Existing assume-yes merely skips operation confirmation, never acknowledges changed default meaning. No normal package command consumes a proposal identity that can bind package, version/source, historical basis, actual consumer set, current definition and proposed effect graph. That is the first missing step. A generic restore/reconfigure diagnostic is not a completed user journey.

## Public qualification

The added focused test reuses the accepted local registry fixture: omitted default disabled at1.0, reachable1.1 changes it to enabled. It invokes actual public update --all with --assume-yes and JSON in a separate process, captures both streams/status, and verifies the refusal does not mutate lock/boot/hooks or create the optional slot. Actual transport is empty stdout and one stderr JSON failure with `ok:false,error`; no proposal or exact expected-definition response identity exists. `ack-qualification-test-r1.log` records one PASS covering this missing-seam observation; captured `ack-response-r1/{stdout,stderr.json,exit}` retains the actual response, not an invented proposal. Scoped strict Clippy, format and diff checks passed. Final binary/source hashes are bound in the private candidate manifest.

Canceling a shown proposal, stale response after subject drift, accepted exact response and post-review cold replay are NOT REACHED, not failed or PASS. There is no public proposal/response authority to exercise. The independent explicit-input behavior is already accepted by the governed-update discriminator, and is not relabeled acknowledgment. No token is fabricated, lock edited, definition expanded into authored answers or assume-yes upgraded into consent.

A subsequent implementation requires an architect-owned typed package proposal/response binding through the same candidate snapshot and selected graph, exact subject freshness checks and unchanged-state refusal/cancellation; it must retain existing lock/publication owners. This qualification does not authorize that implementation or close NST/common configuration.
