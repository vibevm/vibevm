//! T10B lowering acceptance: row for row, refusal for refusal.
//!
//! Every registry here is COLLECTED by the kernel from a real
//! `ExtensionWorld` (`lowering_worlds`), and the lowering is handed exactly
//! `enabled_compile_rows()` — the input contract the workspace supplies — so
//! nothing in this cell can pass by shaping a slice the collector would never
//! produce.

use specmark::verifies;
use vibe_core::manifest::ExtensionConfig;
use vibe_extension_registry::ExtensionRegistry;

use super::config::{ConfigDate, ConfigDatetime, ConfigOffset, ConfigTime, ConfigValue};
use super::config_lowering::ConfigLoweringError;
use super::fault::{LoweringFault, TransformLoweringError};
use super::lowering_worlds::{Declared, collected, collected_host, dependency_key, host_key};
use super::plan::{TransformPlan, TransformStage};
use super::plan_validate::bounded;
use super::registry::TransformRegistryError;
use super::registry_test_support::identity_registry;

/// Lower one collected registry's compile family through the crate-internal
/// seam, against the cfg-test identity catalog.
///
/// The production catalog ships one emitted behavior, so a test that wants a
/// plan with an entry at every staged tier must inject the same four-vehicle
/// catalog the execution tests use. The workspace still supplies only a name —
/// the epoch comes off the catalog either way.
fn lower(registry: &ExtensionRegistry) -> Result<TransformPlan, TransformLoweringError> {
    TransformPlan::from_effective_rows_with(&registry.enabled_compile_rows(), &identity_registry())
}

/// The four staged tiers, each named by its catalog behavior.
fn staged_host() -> Vec<Declared> {
    vec![
        Declared::builtin("src", "compile:source", "test-identity-source"),
        Declared::builtin("doc", "compile:document", "test-identity-document"),
        Declared::builtin("lane", "compile:lane", "test-identity-lane"),
        Declared::builtin("emit", "compile:emitted", "test-identity-emitted"),
    ]
}

/// The typed fault one refusal carries, or a named panic.
#[track_caller]
fn fault(error: &TransformLoweringError) -> &LoweringFault {
    error.inner()
}

/// §4.1: the plan is the rows, in the rows' own effective order, with each
/// row's exact key, stage, provider, config and selector.
///
/// The order assertion is load-bearing rather than decorative: the world
/// authors its four staged rows in the sequence document → emitted → source →
/// lane, which is neither the stage ordinal order nor a key sort, so a
/// lowering that re-tiered or sorted would move it.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn a_collected_registry_lowers_row_for_row_in_the_registrys_own_effective_order() {
    let registry = collected_host(vec![
        Declared::builtin("doc", "compile:document", "test-identity-document"),
        Declared::builtin("emit", "compile:emitted", "test-identity-emitted"),
        Declared::builtin("src", "compile:source", "test-identity-source").scoped(&["boot/*.md"]),
        Declared::builtin("lane", "compile:lane", "test-identity-lane"),
    ]);
    let rows = registry.enabled_compile_rows();
    let plan = lower(&registry).expect("a lawful collected registry lowers");

    assert_eq!(plan.len(), 4, "every enabled compile row became an entry");
    let observed: Vec<(&str, TransformStage, u32)> = plan
        .entries()
        .iter()
        .map(|entry| {
            (
                entry.seed().key().as_str(),
                entry.seed().stage().clone(),
                entry.order(),
            )
        })
        .collect();
    assert_eq!(
        observed,
        vec![
            (host_key("doc").as_str(), TransformStage::Document, 0),
            (host_key("emit").as_str(), TransformStage::Emitted, 1),
            (host_key("src").as_str(), TransformStage::Source, 2),
            (host_key("lane").as_str(), TransformStage::Lane, 3),
        ],
        "the authored order is the plan order, and each point maps to its own stage"
    );

    // Provider: the same typed identity the row carries, through the one
    // root-dropping conversion.
    for (entry, row) in plan.entries().iter().zip(&rows) {
        assert_eq!(
            entry.seed().provider(),
            &super::plan::TransformProvider::from(row.provider()),
            "entry {} carries its row's provider",
            entry.order()
        );
        // Config: nothing was authored, so nothing is claimed.
        assert!(entry.seed().config().is_none());
        assert!(entry.config_digest().is_none());
    }

    // Selector: the scoped source row supplies its row's compiled selector;
    // every unscoped row supplies none, at every stage.
    let scoped = &plan.entries()[2];
    assert_eq!(
        scoped.seed().selector(),
        Some(rows[2].compiled_selector()),
        "the authored `paths` dimension rides along, cloned off the row"
    );
    for index in [0, 1, 3] {
        assert!(
            plan.entries()[index].seed().selector().is_none(),
            "an unscoped row supplies no selector — including at lane/emitted, \
             where manifest presence itself is illegal"
        );
    }
}

/// §4.1: the plan digest is stable across two lowerings of one registry, and
/// it moves when the rows move.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn one_registry_lowers_to_one_digest_twice_and_a_different_order_to_another() {
    let registry = collected_host(staged_host());
    let first = lower(&registry).expect("the first lowering succeeds");
    let second = lower(&registry).expect("the second lowering succeeds");
    assert_eq!(
        first.digest(),
        second.digest(),
        "lowering is deterministic: the same rows, the same identity"
    );
    assert_eq!(first, second);
    assert!(first.digest().is_some(), "a nonempty plan has a digest");

    let mut reordered = staged_host();
    reordered.swap(0, 3);
    let other = lower(&collected_host(reordered)).expect("the reordered world lowers");
    assert_ne!(
        first.digest(),
        other.digest(),
        "entry order is semantic, so a different effective order is a different plan"
    );
}

/// The empty case, and the empty-plan law it feeds: an owner with no compile
/// rows shares the empty plan, digest and all.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn an_owner_with_no_compile_rows_lowers_to_the_empty_plan() {
    let registry = collected_host(vec![Declared::builtin("hook", "phase:build", "log")]);
    assert!(
        registry.enabled_compile_rows().is_empty(),
        "the world's one row is not in the compile family"
    );
    let plan = lower(&registry).expect("an empty compile family lowers");
    assert_eq!(plan, TransformPlan::empty());
    assert!(plan.is_empty());
    assert_eq!(plan.digest(), None);
}

/// The activation tier is inside the ONE effective order: host activation
/// MOVES a dependency compile row into the last tier, behind the host's own
/// declarations, and the lowering preserves exactly that.
///
/// This is what separates the registry's one authored order from any order
/// the lowering could invent: the activated row is a `compile:source`
/// declaration and the host's is `compile:document`, so a lowering that
/// grouped by stage — or by provider, or by key — would put them the other
/// way round.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#COMPILE-ACTIVATION")]
fn an_activated_dependency_row_keeps_its_place_in_the_one_effective_order() {
    let registry = collected(
        vec![Declared::builtin(
            "doc",
            "compile:document",
            "test-identity-document",
        )],
        vec![Declared::builtin(
            "src",
            "compile:source",
            "test-identity-source",
        )],
        &["src"],
    );
    let plan = lower(&registry).expect("the activated world lowers");
    let keys: Vec<&str> = plan
        .entries()
        .iter()
        .map(|entry| entry.seed().key().as_str())
        .collect();
    assert_eq!(
        keys,
        vec![host_key("doc").as_str(), dependency_key("src").as_str()],
        "the host-declaration tier precedes the host-activation tier, and the plan says so"
    );
}

/// §4.2, refusal 1: a `compile:pass` row refuses typed, naming the row.
///
/// It is INSIDE `enabled_compile_rows()` — the whole compile family — so this
/// is the lowering's own judgment about a lawful input, not a caller-contract
/// violation, and it has its own arm for exactly that reason.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn a_compile_pass_row_refuses_typed_until_r6_owns_the_pass_tier() {
    let registry = collected_host(vec![
        Declared::builtin("doc", "compile:document", "test-identity-document"),
        Declared::pass("pass", "test-identity-source"),
    ]);
    assert_eq!(
        registry.enabled_compile_rows().len(),
        2,
        "the pass row really is inside the compile family view"
    );
    let error = lower(&registry).expect_err("the pass tier is not a staged transform");
    let LoweringFault::PassTier { row, preview } = fault(&error) else {
        panic!("the pass tier owns its own arm: {error}")
    };
    assert_eq!(*row, 1, "the fault names the offending row");
    assert_eq!(*preview, bounded(&host_key("pass")));
    assert!(error.to_string().contains("compile:pass"));
}

/// §4.2, refusal 2: a non-builtin handler at a compile point refuses typed
/// and names the handler kind it saw.
///
/// `native` is the ONE such kind that can reach the lowering at all: the
/// manifest grammar already refuses `script` / `binary` / `agent` at a
/// compile point ("compile points accept `builtin` or `native` only"), so
/// this is the exact arm R5 fills in — under the same registry authority,
/// not by widening the caller's.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn a_non_builtin_handler_at_a_compile_point_refuses_typed() {
    let registry = collected_host(vec![Declared::native("compiled", "compile:document")]);
    let error = lower(&registry).expect_err("a native handler is not a staged transform yet");
    let LoweringFault::UnsupportedHandler { row, preview, kind } = fault(&error) else {
        panic!("a non-builtin handler has its own arm: {error}")
    };
    assert_eq!(*row, 0);
    assert_eq!(*kind, "native");
    assert_eq!(*preview, bounded(&host_key("compiled")));
}

/// §4.2, refusal 3: an off-catalog builtin name is the existing bounded
/// `UnknownBuiltin`, raised AT LOWERING rather than deferred.
///
/// Both halves matter. The refusal is the registry's own typed error, so
/// there is one spelling of "no such builtin" in the crate; and it happens
/// while the plan is being built, so a plan that exists is a plan whose every
/// implementation was cataloged when it was built.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn an_off_catalog_builtin_name_is_the_bounded_unknown_builtin_refusal_at_lowering() {
    let registry = collected_host(vec![
        Declared::builtin("doc", "compile:document", "test-identity-document"),
        Declared::builtin("ghost", "compile:source", "no-such-builtin"),
    ]);
    let error = lower(&registry).expect_err("an off-catalog name has no epoch");
    let LoweringFault::Implementation {
        row,
        preview,
        source,
    } = fault(&error)
    else {
        panic!("an unknown builtin refuses through the implementation arm: {error}")
    };
    assert_eq!(*row, 1);
    assert_eq!(*preview, bounded(&host_key("ghost")));
    assert_eq!(
        *source,
        TransformRegistryError::UnknownBuiltin {
            preview: bounded("no-such-builtin"),
        }
    );

    // And the PRODUCTION entry says the same thing: the shipping catalog is
    // exactly the behaviors that exist, so this fixture's `test-identity-*`
    // name is off-catalog there — which is why no host in this repository can
    // declare an arbitrary compile-point builtin and quietly get one.
    let production = TransformPlan::from_effective_rows(&registry.enabled_compile_rows())
        .expect_err("a cfg-test vehicle is not public manifest vocabulary");
    assert!(matches!(
        fault(&production),
        LoweringFault::Implementation { .. }
    ));
}

/// The caller-contract refusal: a row at a NON-compile point is never
/// skipped. `enabled_compile_rows()` cannot produce one, so reaching the
/// lowering with it means the caller passed the wrong view — and a skip would
/// let that wrong view produce a plausible plan.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn a_row_at_a_non_compile_point_is_a_caller_error_and_never_a_skip() {
    let registry = collected_host(vec![
        Declared::builtin("doc", "compile:document", "test-identity-document"),
        Declared::builtin("hook", "phase:build", "log"),
    ]);
    // The wrong view, deliberately: every enabled row, not the compile family.
    let wrong: Vec<_> = registry
        .rows()
        .iter()
        .filter(|row| row.is_enabled())
        .collect();
    assert_eq!(wrong.len(), 2);
    let error = TransformPlan::from_effective_rows_with(&wrong, &identity_registry())
        .expect_err("a phase row is not a staged transform");
    let LoweringFault::NonCompilePoint {
        row,
        preview,
        point,
    } = fault(&error)
    else {
        panic!("the caller-contract violation has its own arm: {error}")
    };
    assert_eq!(*row, 1);
    assert_eq!(*point, "phase:build");
    assert_eq!(*preview, bounded(&host_key("hook")));
}

/// Config presence, the part that is decidable today: absence stays absence
/// and an authored CLEARED table stays a present, digesting empty table. The
/// two are different plan identities and the digest says so.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn absent_config_and_authored_empty_config_stay_two_identities() {
    let absent = lower(&collected_host(vec![Declared::builtin(
        "doc",
        "compile:document",
        "test-identity-document",
    )]))
    .expect("an unconfigured row lowers");
    let cleared = lower(&collected_host(vec![
        Declared::builtin("doc", "compile:document", "test-identity-document")
            .configured(ExtensionConfig::default()),
    ]))
    .expect("an authored empty configuration lowers");

    assert!(absent.entries()[0].seed().config().is_none());
    assert!(absent.entries()[0].config_digest().is_none());
    assert!(cleared.entries()[0].seed().config().is_some());
    assert!(cleared.entries()[0].config_digest().is_some());
    assert_ne!(
        absent.digest(),
        cleared.digest(),
        "`None` and `Some(empty)` are two claims, and plan identity keeps them apart"
    );
}

/// The law T10B's interim refusal stood in for: a row carrying real
/// configuration VALUES lowers LOSSLESSLY, and its digest differs from both
/// the absent and the cleared identity.
///
/// Every `ConfigValue` arm is driven at once, through a REAL collected
/// registry row rather than a hand-built tree — string, integer, float,
/// boolean, all four datetime shapes, a nested array and a nested table —
/// because §3's losslessness is a claim about the whole tower and a walk that
/// dropped one arm would still pass an arm-by-arm test of the others.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn a_row_with_real_configuration_lowers_losslessly_and_digests_differently() {
    let table = concat!(
        "message = 'hello'\n",
        "count = -7\n",
        "ratio = 1.5\n",
        "enabled = true\n",
        "local_date = 1979-05-27\n",
        "local_time = 07:32:00.999999999\n",
        "local_datetime = 1979-05-27T07:32:00\n",
        "zulu = 1979-05-27T07:32:00Z\n",
        "shifted = 1979-05-27T00:32:00-07:00\n",
        "list = [1, 3, 2]\n",
        "[nested]\n",
        "inner = 'deep'\n",
    )
    .parse::<toml::Table>()
    .expect("the fixture config parses");
    let configured = lower(&collected_host(vec![
        Declared::builtin("doc", "compile:document", "test-identity-document")
            .configured(ExtensionConfig::from_table(table)),
    ]))
    .expect("a non-empty effective configuration lowers");

    let seed = configured.entries()[0].seed();
    let lowered = seed.config().expect("the row authored configuration");
    let value = |key: &str| lowered.as_table().get(key).cloned();

    assert_eq!(
        value("message"),
        Some(ConfigValue::String("hello".to_owned()))
    );
    assert_eq!(value("count"), Some(ConfigValue::Integer(-7)));
    assert_eq!(
        value("ratio"),
        Some(ConfigValue::Float(super::config::ConfigFloat::new(1.5)))
    );
    assert_eq!(value("enabled"), Some(ConfigValue::Boolean(true)));
    // Array ORDER is semantic and retained; table order is not and sorts.
    assert_eq!(
        value("list"),
        Some(ConfigValue::Array(vec![
            ConfigValue::Integer(1),
            ConfigValue::Integer(3),
            ConfigValue::Integer(2),
        ]))
    );
    let ConfigValue::Table(nested) = value("nested").expect("the nested table lowers") else {
        panic!("a nested table stays a table")
    };
    assert_eq!(
        nested.get("inner"),
        Some(&ConfigValue::String("deep".to_owned()))
    );

    // Datetime, component for component: all four legal shapes, and the two
    // offset spellings kept apart — `Z` is not `+00:00`.
    let date = ConfigDate::new(1979, 5, 27).expect("a legal date");
    let midnight_time = ConfigTime::new(7, 32, 0, 0).expect("a legal time");
    assert_eq!(
        value("local_date"),
        Some(ConfigValue::Datetime(
            ConfigDatetime::new(Some(date), None, None).expect("a legal local date")
        ))
    );
    assert_eq!(
        value("local_time"),
        Some(ConfigValue::Datetime(
            ConfigDatetime::new(
                None,
                Some(ConfigTime::new(7, 32, 0, 999_999_999).expect("a legal time")),
                None
            )
            .expect("a legal local time")
        ))
    );
    assert_eq!(
        value("local_datetime"),
        Some(ConfigValue::Datetime(
            ConfigDatetime::new(Some(date), Some(midnight_time), None)
                .expect("a legal local datetime")
        ))
    );
    assert_eq!(
        value("zulu"),
        Some(ConfigValue::Datetime(
            ConfigDatetime::new(Some(date), Some(midnight_time), Some(ConfigOffset::Z))
                .expect("a legal offset datetime")
        ))
    );
    let shifted = ConfigDatetime::new(
        Some(date),
        Some(ConfigTime::new(0, 32, 0, 0).expect("a legal time")),
        Some(ConfigOffset::custom(-7 * 60).expect("a legal offset")),
    )
    .expect("a legal offset datetime");
    assert_eq!(value("shifted"), Some(ConfigValue::Datetime(shifted)));
    assert_ne!(
        shifted.offset(),
        Some(ConfigOffset::Z),
        "offset identity survives the walk"
    );

    // The three config states are three plan identities.
    let absent = lower(&collected_host(vec![Declared::builtin(
        "doc",
        "compile:document",
        "test-identity-document",
    )]))
    .expect("an unconfigured row lowers");
    let cleared = lower(&collected_host(vec![
        Declared::builtin("doc", "compile:document", "test-identity-document")
            .configured(ExtensionConfig::default()),
    ]))
    .expect("an authored empty configuration lowers");
    assert_ne!(configured.digest(), absent.digest());
    assert_ne!(configured.digest(), cleared.digest());
    assert_ne!(absent.digest(), cleared.digest());
}

/// The value-tower walk's one remaining refusal: a datetime component the
/// neutral tree's checked constructors reject refuses TYPED, naming the row,
/// rather than panicking inside the lowering.
///
/// A parsed manifest cannot reach this arm — `toml_datetime`'s parser
/// enforces the same laws — but `toml::value::Datetime` is a struct of public
/// fields, so a value built around the parser is representable and must be
/// refused rather than trusted.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn an_illegal_datetime_component_refuses_typed_and_names_its_row() {
    let mut table = toml::Table::new();
    table.insert(
        "when".to_owned(),
        toml::Value::Datetime(toml::value::Datetime {
            // Five digits: unrepresentable as a TOML `date-fullyear`, and so
            // unrepresentable as a config identity.
            date: Some(toml::value::Date {
                year: 10_000,
                month: 5,
                day: 27,
            }),
            time: None,
            offset: None,
        }),
    );
    let registry = collected_host(vec![
        Declared::builtin("doc", "compile:document", "test-identity-document")
            .configured(ExtensionConfig::from_table(table)),
    ]);
    let error = lower(&registry).expect_err("an illegal datetime component is not an identity");
    let LoweringFault::Config {
        row,
        preview,
        source,
    } = fault(&error)
    else {
        panic!("configuration has its own arm: {error}")
    };
    assert_eq!(*row, 0);
    assert_eq!(*preview, bounded(&host_key("doc")));
    let ConfigLoweringError::Datetime { source } = source;
    assert!(
        source.to_string().contains("date.year"),
        "the refusal names the offending component: {source}"
    );
}

/// A plan-level refusal keeps its own typed source: the lowering does not
/// re-describe what `TransformPlan::build` already refuses.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn a_plan_refusal_rides_through_the_lowering_as_itself() {
    // Two rows, one key: the collector refuses a duplicate key inside one
    // world, so the duplicate is built by handing the lowering the SAME row
    // twice — the shape a caller error would produce.
    let registry = collected_host(vec![Declared::builtin(
        "doc",
        "compile:document",
        "test-identity-document",
    )]);
    let rows = registry.enabled_compile_rows();
    let doubled = vec![rows[0], rows[0]];
    let error = TransformPlan::from_effective_rows_with(&doubled, &identity_registry())
        .expect_err("one key cannot be two entries");
    let LoweringFault::Plan { source } = fault(&error) else {
        panic!("the plan's own refusal rides through: {error}")
    };
    assert!(source.to_string().contains("duplicate transform key"));
}
