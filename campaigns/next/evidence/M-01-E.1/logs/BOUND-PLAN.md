# M-01-E.1 bound cases

Base: accepted D2 23dce2e71. Only new test children, incumbent registrations, PROP070 observation fixture clause.

| Target / family / case | Owner and expected outcome | Execution class / state |
|---|---|---|
| vibe-spec --lib resolver::m_01_e_1_cases::unique_nested_source_reaches_held_consumer | FileResolver + held read + FsSectionSource: exact nested body/digest | Linux filesystem, full tree unchanged |
| portable_components_refuse_before_source_observation | shared component preflight: UnsafeSourcePath for traversal/drive/UNC/ADS/device/trailing-dot/control | portable lexical on Linux; outside fixture unchanged, no read hook |
| unicode_and_case_aliases_preserve_exact_fixture_spelling | native entries + NFC/case law: PortableSourceAlias | portable identity on Linux; full tree unchanged |
| duplicate_anchor_refuses_both_candidates | actual FsSectionSource: ambiguity naming same anchor; no first content | Linux parser/consumer; full tree unchanged |
| failed_enumeration_is_unreadable_not_absent | actual read_dir(file) and resolver installed-slot enumeration: SourceUnreadable | Linux native NotADirectory, not chmod permission proof; unchanged |
| same_byte_replacement_refuses_without_query_writes | actual held resolver: SourceObservationUnavailable, final-name race | Linux rename after held open; post-injection tree unchanged |
| vibe-safefs --lib file::m_01_e_1_cases::large_verified_stream_can_be_cancelled_without_writes | bounded held stream: Incomplete then exact dropped-before-completion Uncertain; complete control exact digest | Linux held stream, 4 MiB; unchanged |
| native_links_refuse_without_touching_targets | Project snapshot rejects symlink and hardlink at actual ownership boundary | Linux native; no outside target change |
| held_replacement_refuses_equal_bytes_and_preserves_injected_state | final-name identity mismatch refuses | Linux native rename; compare post-injection state |
| portable_identity_mismatch_refuses_without_namespace_change | existing identity-check hook -> final-name race, then stable re-read | deterministic portable simulation, not native Windows proof |
| growth_after_open_refuses_a_truncated_prefix | existing bounded-read hook grows beyond cap -> cap refusal | Linux actual write after open; post-injection state unchanged |

Exact family filters: `resolver::m_01_e_1_cases::` (6), `file::m_01_e_1_cases::` (5), no ignored cases on Linux. Bind before Cargo. New child paths checked absent with rg --files.

Windows reparse/ADS-native/device-native and macOS native alias/replacement remain unperformed on this Linux host. Actual host uid is 1000. Permission-denied chmod enumeration was not executed; the bound native ENOTDIR fixture is the measured failed-enumeration case, not a permission-denied claim. No host settings/mounts/network or real installation touched. Snapshot inventory includes paths/types/content/link targets and Unix identity/mode; atime excluded because reads may update it. Race fixtures compare the intended injected state, not claim the injection preserved pre-race state. No atomic or hostile-ABA guarantee.
