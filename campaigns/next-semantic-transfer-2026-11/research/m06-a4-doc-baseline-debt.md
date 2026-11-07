# Documentation baseline debt: all 62 shared failures

Read-only source/log classification; no Cargo, tests or repository writes. Root acceptance and runtime verification remain pending.

## Admission boundary

pages::read_package observes policy before walk; observe_package_policy calls Manifest::parse_str, which performs typed deserialize AND validate. Missing manifest compatibility is distinct from present invalid manifest. Preserve this boundary.

## Findings

| Group | Count | Classification |
|---|---:|---|
| derived | 5 | fixture-invalid |
| glossary | 6 | fixture-invalid |
| manifest-doc | 3 | fixture-invalid |
| manifest-bridge | 1 | fixture-invalid |
| manifest-kinds | 1 | fixture-invalid-plus-product-seam |
| prompts | 7 | fixture-invalid |
| site-render | 2 | product-behavioral-regression |
| style-glossary | 4 | fixture-invalid |
| style-terms | 13 | fixture-invalid |
| style | 8 | fixture-invalid |
| surface-diff | 3 | fixture-invalid |
| surface-fields | 1 | stale-expectation |
| todo | 8 | fixture-invalid |

Do not classify these as all missing version. Most doc fixtures already have version and lack documents. Two failures exercise a real broken production fallback writer. The kind-card test also exposes a generated-render identity collision. The lockfield test is an independent stale schema expectation.

## derived

Helper/source: `crates/vibe-doc/src/derived/tests.rs::package (and product-change rewrite)`

Both initial and rewritten manifests omit required kind and version.

**Bounded correction:** Use a complete doc package and [[documents]] subject in both manifest writes; retain title changes, exact hash/size, vanished-block and invalid-reference assertions.

**Uncertainty:** Source inspection establishes the admission defect; downstream assertions have not been rerun after correction.

Mapped tests:

- `derived::tests::a_block_that_left_the_pages_is_reported_rather_than_forgotten` — `crates/vibe-doc/src/derived/tests.rs:100:41`
- `derived::tests::a_first_build_reports_every_block_as_new_and_accept_records_it` — `crates/vibe-doc/src/derived/tests.rs:49:55`
- `derived::tests::a_product_change_shows_up_as_a_changed_block_and_is_red` — `crates/vibe-doc/src/derived/tests.rs:84:41`
- `derived::tests::a_reference_the_generators_cannot_build_stops_the_check_by_name` — `crates/vibe-doc/src/derived/tests.rs:120:5`
- `derived::tests::the_record_keeps_a_hash_and_a_size_and_never_the_text` — `crates/vibe-doc/src/derived/tests.rs:68:41`
## glossary

Helper/source: `crates/vibe-doc/src/glossary/tests.rs::DECLARING/package`

DECLARING omits version and documents; bare manifests contain only package.name and omit group/kind/version.

**Bounded correction:** Give both declared and bare variants valid doc identity/version/subject; bare means no glossary, not no package metadata. Preserve missing-page/unreadable-page and entry assertions.

**Uncertainty:** Source inspection establishes the admission defect; downstream assertions have not been rerun after correction.

Mapped tests:

- `glossary::tests::a_declaration_is_read_off_the_manifest` — `crates/vibe-doc/src/glossary/tests.rs:40:37`
- `glossary::tests::a_glossary_page_that_is_not_there_is_a_defect_and_not_a_refusal` — `crates/vibe-doc/src/glossary/tests.rs:40:37`
- `glossary::tests::a_package_with_no_declaration_has_nothing_to_check` — `crates/vibe-doc/src/glossary/tests.rs:40:37`
- `glossary::tests::an_entry_is_a_section_with_its_first_paragraph` — `crates/vibe-doc/src/glossary/tests.rs:40:37`
- `glossary::tests::an_entry_needs_a_term_and_a_definition` — `crates/vibe-doc/src/glossary/tests.rs:40:37`
- `glossary::tests::an_unreadable_glossary_page_is_reported_as_one` — `crates/vibe-doc/src/glossary/tests.rs:40:37`
## manifest-doc

Helper/source: `crates/vibe-doc/src/manifest/tests.rs inline doc fixtures`

Three inline doc fixtures have name/group/kind/version but omit mandatory [[documents]].

**Bounded correction:** Add a valid subject to authorship/navigation/learning-path manifests, preserving metadata and reversed-page ordering; both learning-path writes share corrected card.

**Uncertainty:** Source inspection establishes the admission defect; downstream assertions have not been rerun after correction.

Mapped tests:

- `manifest::tests::the_card_carries_who_wrote_the_prose_when_the_package_says` — `crates/vibe-doc/src/manifest/tests.rs:118:77`
- `manifest::tests::the_learning_path_is_carried_whole_and_moves_no_page` — `crates/vibe-doc/src/manifest/tests.rs:181:14`
- `manifest::tests::the_navigation_is_carried_and_the_page_order_is_left_alone` — `crates/vibe-doc/src/manifest/tests.rs:139:77`
## manifest-bridge

Helper/source: `crates/vibe-doc/src/manifest/tests.rs::a_bridge_carries_the_two_authorships_it_keeps_apart source closure/card_of`

Embedded sources use commit abc, content_hash sha256:0, and unpinned license_url. All violate EmbeddedSourceDecl::validate.

**Bounded correction:** Use deterministic full lowercase Git OIDs, sha256-tree/1: plus 64 lowercase hex, and license URLs with the exact commit as a path segment; keep overlapping authors and agreeing/disagreeing licenses.

**Uncertainty:** Source inspection establishes the admission defect; downstream assertions have not been rerun after correction.

Mapped tests:

- `manifest::tests::a_bridge_carries_the_two_authorships_it_keeps_apart` — `crates/vibe-doc/src/manifest/tests.rs:366:10`
## manifest-kinds

Helper/source: `crates/vibe-doc/src/manifest/tests.rs::the_card_carries_the_kind_the_package_calls_itself_by card closure/card_of`

First known invalid variant is mcp without [[mcp_server]]. Later pack is not a PackageKind, yet the test explicitly requires absence of original kind in a synthetic level-0 card. Production site/level0/card.rs::synthesise writes that pack fallback too.

**Bounded correction:** Supply valid MCP fixture metadata (e.g. one valid remote HTTPS streamable-http server). Preserve the unknown-original-kind card behavior through an explicit generated-render input/card boundary; do not add pack to PackageKind or invent an original kind. Keep full typed admission for actual authored package roots.

**Uncertainty:** No per-variant runtime trace; source order identifies MCP as first known refusal, while pack is a separately proven latent collision. Precise generated-render carrier/API is a design decision for the page/census slice; app variant may need additional metadata under future validation.

Mapped tests:

- `manifest::tests::the_card_carries_the_kind_the_package_calls_itself_by` — `crates/vibe-doc/src/manifest/tests.rs:366:10`
## prompts

Helper/source: `crates/vibe-doc/src/prompts/tests.rs::Package::new`

Doc fixture has identity/version but no documents subject.

**Bounded correction:** Add valid [[documents]] to shared fixture; keep real fake-agent subprocess, sandbox isolation, skips, sample and assert outcomes.

**Uncertainty:** Source inspection establishes the admission defect; downstream assertions have not been rerun after correction.

Mapped tests:

- `prompts::tests::a_declared_skip_is_honoured_and_named` — `crates/vibe-doc/src/prompts/tests.rs:116:58`
- `prompts::tests::a_prompt_outside_the_sample_is_skipped_and_says_so` — `crates/vibe-doc/src/prompts/tests.rs:116:58`
- `prompts::tests::a_runner_that_fails_is_failed_and_not_broken` — `crates/vibe-doc/src/prompts/tests.rs:116:58`
- `prompts::tests::an_agent_that_does_nothing_breaks_the_assert` — `crates/vibe-doc/src/prompts/tests.rs:116:58`
- `prompts::tests::an_illustrative_prompt_is_skipped_with_its_reason` — `crates/vibe-doc/src/prompts/tests.rs:116:58`
- `prompts::tests::the_agent_does_the_task_and_the_page_s_asserts_judge_the_result` — `crates/vibe-doc/src/prompts/tests.rs:116:58`
- `prompts::tests::two_prompts_on_one_fixture_cannot_see_each_other_s_work` — `crates/vibe-doc/src/prompts/tests.rs:116:58`
## site-render

Helper/source: `crates/vibe-doc/src/site/render.rs::failed_manifest/refused; tests.rs::Unreachable`

Production failed_manifest emits name/group/version/title/abstract but omits required kind. The error page itself then fails source-policy admission and the promised graceful failure page never renders.

**Bounded correction:** Represent failure pages through a typed generated-render input retaining original coordinate and unknown original kind. A dedicated valid synthetic doc carrier is possible only if rendered subject/card semantics stay distinct. Never guess the failed package kind, delete the guard, or turn tests into expected refusal.

**Uncertainty:** Observed regression is proven by baseline panic and production writer. Minimal architecture must account for level0 synthetic pack and retained metadata rather than blindly adding kind=doc (which also needs documents and changes projection semantics).

Mapped tests:

- `site::render::tests::a_package_that_cannot_be_fetched_becomes_a_page_and_not_a_stop` — `crates/vibe-doc/src/site/render/tests.rs:204:66`
- `site::render::tests::a_retry_that_succeeds_replaces_the_failure_page` — `crates/vibe-doc/src/site/render/tests.rs:230:62`
## style-glossary

Helper/source: `crates/vibe-doc/src/style/glossary/tests.rs::declared_glossary and standalone missing-page fixture`

Both fixture manifest writers contain only [glossary], with no role; glossary is also doc-only.

**Bounded correction:** Add complete doc identity/version/documents to both writes, retaining missing page and term extraction cases.

**Uncertainty:** Source inspection establishes the admission defect; downstream assertions have not been rerun after correction.

Mapped tests:

- `style::glossary::tests::a_disambiguating_parenthesis_is_not_part_of_the_term` — `crates/vibe-doc/src/style/glossary/tests.rs:30:54`
- `style::glossary::tests::a_package_with_no_declaration_has_no_terms` — `crates/vibe-doc/src/style/glossary/tests.rs:84:54`
- `style::glossary::tests::the_six_ordinary_words_are_not_terms` — `crates/vibe-doc/src/style/glossary/tests.rs:30:54`
- `style::glossary::tests::the_terms_are_the_glossary_sections_longest_first` — `crates/vibe-doc/src/style/glossary/tests.rs:30:54`
## style-terms

Helper/source: `crates/vibe-doc/src/style/terms/tests.rs::declared`

Shared fixture manifest contains only [glossary], with no role.

**Bounded correction:** Add complete doc identity/version/documents; preserve exact glossary and first-use prose assertions.

**Uncertainty:** Source inspection establishes the admission defect; downstream assertions have not been rerun after correction.

Mapped tests:

- `style::terms::tests::a_code_span_between_the_term_and_its_gloss_is_part_of_the_phrase` — `crates/vibe-doc/src/style/terms/tests.rs:30:54`
- `style::terms::tests::a_comma_somewhere_else_in_the_sentence_is_not_a_gloss` — `crates/vibe-doc/src/style/terms/tests.rs:30:54`
- `style::terms::tests::a_first_use_in_the_italics_of_a_definition_introduces_the_term` — `crates/vibe-doc/src/style/terms/tests.rs:30:54`
- `style::terms::tests::a_first_use_linked_to_the_glossary_entry_introduces_the_term` — `crates/vibe-doc/src/style/terms/tests.rs:30:54`
- `style::terms::tests::a_gloss_in_the_same_sentence_introduces_the_term` — `crates/vibe-doc/src/style/terms/tests.rs:30:54`
- `style::terms::tests::a_link_that_comes_after_the_first_use_does_not_introduce_it` — `crates/vibe-doc/src/style/terms/tests.rs:30:54`
- `style::terms::tests::a_link_to_another_entry_introduces_nothing` — `crates/vibe-doc/src/style/terms/tests.rs:30:54`
- `style::terms::tests::a_link_to_the_entry_introduces_the_term_whatever_its_text_says` — `crates/vibe-doc/src/style/terms/tests.rs:30:54`
- `style::terms::tests::a_term_in_the_first_paragraph_is_an_error` — `crates/vibe-doc/src/style/terms/tests.rs:30:54`
- `style::terms::tests::containers_and_prompts_do_not_carry_a_first_use` — `crates/vibe-doc/src/style/terms/tests.rs:30:54`
- `style::terms::tests::only_the_first_use_on_a_page_is_judged` — `crates/vibe-doc/src/style/terms/tests.rs:30:54`
- `style::terms::tests::the_glossary_page_is_not_judged_against_itself` — `crates/vibe-doc/src/style/terms/tests.rs:30:54`
- `style::terms::tests::three_terms_in_one_sentence_is_a_warning` — `crates/vibe-doc/src/style/terms/tests.rs:30:54`
## style

Helper/source: `crates/vibe-doc/src/style/tests.rs::Package::declaring`

Doc fixture has identity/version/i18n but omits documents.

**Bounded correction:** Add valid subject in shared writer; preserve language, banned lists, glossary omission and prompt/style expectations. Unsupported-language test must still reach its intended language refusal.

**Uncertainty:** Source inspection establishes the admission defect; downstream assertions have not been rerun after correction.

Mapped tests:

- `style::tests::a_lower_bar_lets_an_unfinished_corpus_run` — `crates/vibe-doc/src/style/tests.rs:77:51`
- `style::tests::a_package_that_declares_no_glossary_is_judged_without_terms` — `crates/vibe-doc/src/style/tests.rs:77:51`
- `style::tests::a_package_whose_language_has_no_list_is_refused` — `crates/vibe-doc/src/style/tests.rs:307:5`
- `style::tests::a_prompt_without_an_assert_on_a_scenario_page_is_an_error` — `crates/vibe-doc/src/style/tests.rs:77:51`
- `style::tests::an_illustrative_prompt_on_an_explanation_page_is_left_alone` — `crates/vibe-doc/src/style/tests.rs:77:51`
- `style::tests::the_law_s_own_bad_page_is_caught_on_every_pair` — `crates/vibe-doc/src/style/tests.rs:77:51`
- `style::tests::the_law_s_own_good_page_is_clean` — `crates/vibe-doc/src/style/tests.rs:77:51`
- `style::tests::the_russian_list_judges_the_russian_page` — `crates/vibe-doc/src/style/tests.rs:77:51`
## surface-diff

Helper/source: `crates/vibe-doc/src/surface/diff/tests.rs::package`

Doc fixture has identity/version but no documents.

**Bounded correction:** Add valid subject, retaining the three generated-page relationships and known diff assertions.

**Uncertainty:** Source inspection establishes the admission defect; downstream assertions have not been rerun after correction.

Mapped tests:

- `surface::diff::tests::a_change_no_page_answers_to_asks_for_a_new_page` — `crates/vibe-doc/src/surface/diff/tests.rs:181:6`
- `surface::diff::tests::a_known_difference_names_a_known_list_of_pages` — `crates/vibe-doc/src/surface/diff/tests.rs:129:6`
- `surface::diff::tests::two_readings_of_one_surface_produce_an_empty_answer_that_says_so` — `crates/vibe-doc/src/surface/diff/tests.rs:211:6`
## surface-fields

Helper/source: `crates/vibe-doc/src/surface/fields/tests.rs::a_required_member_does_not_hide_the_list`

Actual Lockfile parser now lists meta/package/configurations/remap. Expected exact list omits configurations; baseline output matches current Lockfile definition.

**Bounded correction:** Add configurations in parser declaration order and update wording to one required/three optional/four total. Retain exact list assertion: do not weaken to subset or change parser.

**Uncertainty:** No behavioral regression established; schema source and actual panic agree. Runtime confirmation remains pending.

Mapped tests:

- `surface::fields::tests::a_required_member_does_not_hide_the_list` — `crates/vibe-doc/src/surface/fields/tests.rs:70:5`
## todo

Helper/source: `crates/vibe-doc/src/todo/tests.rs::package`

Doc fixture has identity/version/i18n but no documents.

**Bounded correction:** Add valid subject in helper, preserving all eight-number/null/queue/reading/debt expectations.

**Uncertainty:** Source inspection establishes the admission defect; downstream assertions have not been rerun after correction.

Mapped tests:

- `todo::tests::a_journal_entry_owing_a_decision_is_the_seventh_number` — `crates/vibe-doc/src/todo/tests.rs:157:6`
- `todo::tests::a_number_nobody_measured_is_null_and_not_zero` — `crates/vibe-doc/src/todo/tests.rs:85:46`
- `todo::tests::a_package_with_no_readings_owes_one_for_every_page` — `crates/vibe-doc/src/todo/tests.rs:102:46`
- `todo::tests::a_queue_full_of_rows_is_still_a_successful_reading` — `crates/vibe-doc/src/todo/tests.rs:195:46`
- `todo::tests::every_section_is_emitted_even_when_it_was_not_measured` — `crates/vibe-doc/src/todo/tests.rs:69:46`
- `todo::tests::the_debt_file_feeds_the_queue_and_the_p1_number` — `crates/vibe-doc/src/todo/tests.rs:131:6`
- `todo::tests::the_json_carries_the_eight_numbers_by_the_names_the_table_uses` — `crates/vibe-doc/src/todo/tests.rs:205:46`
- `todo::tests::the_recorded_reading_gives_the_median_and_the_reconciliation_date` — `crates/vibe-doc/src/todo/tests.rs:172:46`

## Execution recommendation

First correct fixture manifests without altering their assertions or page bodies, then rerun all mapped tests and the full vibe-doc library suite under the existing lease. Handle the synthetic level-0/failure-card carrier as product work in the page/census slice: authored admission must stay strict, synthetic metadata must not masquerade as valid original package metadata. Update the exact lock-field expectation against the current parser. Any remaining failure is new evidence requiring classification, not permission to blanket-ignore it.

The baseline-only declared-point citation and candidate-only empty-world refusal tests are excluded from these 62 and remain the point-read owner’s work. Evidence hashes and every original panic are in the JSON.
