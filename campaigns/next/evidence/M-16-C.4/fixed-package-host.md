# Trusted fixed package host boundary

`FixedPackageHost` receives an explicit physical `Project` and host-issued C1
namespace/scope. It validates their binding at construction and under the
incumbent lifecycle lease on each call. `submit` supplies a host clock and
calls the exact lifecycle fixed static-file package owner once; `inspect`
uses its cold result path. The object stores no second reservation or result
cache and advertises no descriptor or transport endpoint.

Focused Linux proof: `vibe-orchestrator --lib
machine_operation::fixed_package` passed 4/4 in the worker and primary
targets; two focused doctests passed in the worker. The tests cover one copy,
same-key replay after another host clock reading, cold reconstruction,
changed source, foreign scope, physical-root replacement and damaged retained
completion without recopy. The lifecycle owner keeps C1 Ready, the capsule,
output and A2 verification authoritative. No full panel or mutation campaign
was run.

The embedding still must authenticate its physical root and issue the C1
namespace. An owner `Err` after submit cannot establish no effects. Generated
package action formats and an orchestrator envelope adapter are separate work
before any public CLI/MCP/local frame advertises this operation.
