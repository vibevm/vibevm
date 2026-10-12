# M-02-B.2 incidents

- Initial batched source output was truncated; standing rules/contract/required seams reread in bounded chunks. No truncated tail treated as complete reading.

- I ran direct rustfmt on module owners with edition2021 and recursion enabled. It changed formatting in four unchanged requirements A1/A2/B1/read files. Inspected exact diff (format-only), restored their original HEAD bytes through file writes; no Git index/history mutation. Subsequent touched formatting uses edition2024 and skip_children=true. Previous tests were behavioral source-equivalent; final narrow compatibility runs use restored source bytes.

- Initial source review found derived enum ordering for needs instead of lexical kind/target ordering. Corrected before family execution; canonical nested-collection case proves lexical sorting. No claimed test red.
- Required full fmt exits1 with14 inherited findings (packet named13). Extra reported B1 fixture finding is byte-identical to HEAD; exact inherited subject proof retained. No changed B2 owner formatting failure. No compile/test failure or zero-selection occurred in B2.
