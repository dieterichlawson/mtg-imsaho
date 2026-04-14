# Test Suite Audit — Findings & Coverage Baseline

Companion to [docs/plans/TEST_SUITE_AUDIT_PLAN.md](../docs/plans/TEST_SUITE_AUDIT_PLAN.md).

Generated 2026-04-14.

## Headline numbers

| Metric | Value |
|---|---|
| Integration test files | 85 |
| Integration test functions | 1,053 |
| Inline unit tests | ~47 (6 src files) |
| `#[ignore]` / `#[should_panic]` | 0 |
| Expected-to-fail audit regressions | ~69 |
| **Line coverage (mtg-engine)** | **85.07%** |
| Function coverage | 83.36% |
| Region coverage | 86.23% |

Full per-file breakdown: `reports/coverage/summary.txt`. HTML report: `reports/coverage/html/index.html`.

## Coverage — engine modules

```
actions.rs        0.00%  (36 lines — almost all enum/struct defs, not really code)
ids.rs            0.00%  (9 lines — pure type aliases)
view.rs          71.83%  (186 lines; 48 uncovered)  ← real gap
engine.rs        75.78%  (3,666 lines; 881 uncovered) ← biggest absolute gap
triggers.rs      81.69%  (977 lines; 197 uncovered) ← meaningful gap
types.rs         87.16%
funding.rs       87.03%
destruction.rs   89.04%
state.rs         91.41%  ← hypothesis was wrong: well-covered
combat.rs        92.78%
stack.rs         92.50%  ← hypothesis was wrong: well-covered
mana.rs          96.84%
sba.rs           98.63%
```

### Calibration against pre-coverage hypotheses

- **Wrong hypotheses:** `state.rs` (thought under-tested → 91%), `stack.rs` (thought under-tested → 92%), `destruction.rs` (thought under-tested → 89%). The huge size of `state.rs` (~88KB) made it look under-tested, but it's exercised thoroughly by the card-level integration tests.
- **Right hypotheses:** `view.rs` (71.8%), card-level over-testing (most cards 95-100%).
- **Misleading:** `actions.rs` / `ids.rs` show 0% but are ~all type definitions with no branching code. Not a real gap.

## Coverage — worst-covered cards

```
cards/helpers.rs                      62.57%  ← shared, worth fixing
cards/isd/sulfur_falls.rs             28.77%
cards/isd/isolated_chapel.rs          28.77%
cards/isd/bloodline_keeper.rs         29.23%
cards/isd/geistcatchers_rig.rs        46.00%
cards/isd/full_moons_rise.rs          46.67%
cards/isd/selhoff_occultist.rs        48.33%
cards/isd/ancient_grudge.rs           51.43%
cards/island.rs                       53.85%
cards/isd/witchbane_orb.rs            54.17%
cards/isd/into_the_maw_of_hell.rs     55.88%
cards/isd/mikaeus_the_lunarch.rs      57.61%
cards/isd/murder_of_crows.rs          58.73%
```

The dual-land cluster (`sulfur_falls`, `isolated_chapel`, `clifftop_retreat`, `hinterland_harbor`, `woodland_cemetery`) all sit between 29%–86% — each probably shares a pattern tested only once, leaving the other branches cold. A single shared test matrix over dual lands would fix most of these.

## Test smells catalogued

### Debug leakage — small, mechanical fix
- 4 `eprintln!` statements in `audit_bugs2.rs` lines 87-94.

### Hardcoded IDs
- `mtg_engine::ids::CardId(170)` and similar in audit files — breaks under registry reshuffling.

### Internal-state assertions (not observable behavior)
~30 sites assert on `counters.get(&PlusOnePlusOne)` instead of `effective_power`/`effective_toughness`. Example: `tier15_cards.rs:43-44`.

### Brittle string matches
~15–20 `name == "Wolf"` token matches — prefer matching by creature type / registry lookup.

### Exact ordering / count assertions
~8–10 places. `assert_eq!(combat_attackers, 4)` style; membership assertions would be more durable.

## Missed shared-helper opportunities

`tests/common/mod.rs` is already well-adopted (~80% of files use `game_at_step`, `castable_spell`, `named_creature`, etc.). Additions worth pulling out:

- `graveyard_creature_named(state, registry, name, owner)` — hand-rolled in 20+ sites (especially audit files).
- `attach_equipment(state, equip, creature)` — one-liner repeated throughout equipment tests.
- Promote `process_triggers_with_choices()` (53-line local helper inside `tests/apnap.rs`) into `common/`.
- `test_registry!` macro — each integration-test binary currently redeclares `fn registry() -> CardRegistry { CardRegistry::with_all_cards() }`.

## Structural observations

Three overlapping organizing axes today:
- **Implementation tier** — `tier2_spells.rs`…`tier15_cards.rs` (gaps at 4, 13)
- **Per-card** — ~14 small files (often <150 lines / <5 tests)
- **Mechanic family** — `combat.rs`, `keywords.rs`, `activated_abilities.rs`, etc.
- **Bug provenance** — `bug_fixes.rs`, `card_fixes.rs`, `engine_bugs.rs`, `death_trigger_bugs.rs`, `audit_bugs.rs`, `audit_bugs2.rs`, `audit_*_family.rs` (11 more)

Target layout when reorganizing:
```
tests/
  mechanics/     combat, triggers, sba, stack, mana, funding, x_cost, …
  cards/         grouped by set or mechanic (replaces per-card + tier files)
  regressions/   merges bug_fixes + audit_* (prefix files with bug tag)
  common/
```

Do not reorganize until the low-risk cleanup and helper extraction passes are done — file moves over 85 files will churn blame history heavily.

## Revised priorities (coverage-informed)

1. **Add tests for `view.rs`** (28% uncovered, 48 lines). Serialization/display path — important for logs and the CLI game.
2. **Find the 881 uncovered lines in `engine.rs`** — look at the HTML report. Likely error/edge-case paths; each uncovered branch is a potential hidden bug.
3. **Cover the under-tested dual lands as a group** — single parameterized test matrix instead of per-card tests.
4. **Cover `cards/helpers.rs`** (62.57%) — shared card-side helper, uncovered paths likely affect many cards.
5. **`triggers.rs` (197 uncovered lines)** — inspect HTML to identify which trigger kinds are untested.
6. **Skip:** `state.rs`, `stack.rs`, `destruction.rs`, `actions.rs`, `ids.rs`. Either already well-covered or trivially uncoverable.

## Tooling

- **Tool:** `cargo-llvm-cov` 0.8.5, installed.
- **Local run:** `cargo llvm-cov --package mtg-engine --html --output-dir reports/coverage`
- **Text summary:** `cargo llvm-cov report --summary-only`
- **For CI:** add `--lcov --output-path reports/coverage/lcov.info` to emit LCOV. Publish as artifact; defer numeric gates until the gap-closing pass is done.
