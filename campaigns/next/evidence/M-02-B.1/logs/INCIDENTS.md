# M-02-B.1 incidents

- Batched long source reads exceeded output caps; relevant defining seams reread in bounded segments. Truncation is not complete-read evidence.
- Initial codegen succeeded but registry implicit unknown_fields default was Allow while shared authored parser is closed. Before acceptance added explicit deny and corpus parity case; regenerate mechanically. No accepted wire/readiness claim from first generation.

- check-codegen exit1 compares regenerated files to HEAD: the two intentional unstaged FormatId changes make it red. All194 generated files were byte-identical before/after repeat generation; no global green claim.
- Full fmt exit1 retains13 existing findings, including the pre-existing generated FormatId arm formatting. No baseline cleanup.
