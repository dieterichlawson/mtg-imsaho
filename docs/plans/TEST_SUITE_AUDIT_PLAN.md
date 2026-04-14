# Test Suite Audit & Refactor Plan

Plan for auditing and improving the MTG engine test suite at `mtg-engine/tests/`.
See [reports/test_suite_audit.md](../../reports/test_suite_audit.md) for the full evaluation (to be written after coverage run).

## Current state (baseline)

- 85 integration test files, ~1,053 tests, ~32k LOC of test code
- 47 inline unit tests across 6 `src/` files
- 0 `#[ignore]` / `#[should_panic]`; ~69 expected-to-fail audit regressions
- No coverage tooling configured

## Organizing principles (from research)

- **Testing pyramid / purpose** — unit-heavy, integration for contracts; tests serve risk control, not gatekeeping.
- **Behavior over implementation** — assert observable state (view layer, effective P/T), not internal counter maps or object IDs.
- **DRY applies to ideas, not arrange-act-assert blocks** — factor setup, not assertions. Over-DRYing tests makes them brittle.
- **Test smells to hunt:** Mystery Guest, Fragile Test, Eager Test, Assertion Roulette, Obscure Test, Conditional Test Logic, Test Code Duplication.

## Findings summary

### Structure — needs consolidation
Three overlapping organizing axes (implementation tier, per-card file, mechanic family, bug provenance). Target layout:
```
tests/
  mechanics/     combat, triggers, sba, stack, mana, funding, x_cost, ...
  cards/         grouped by set or mechanic — replaces per-card and tier files
  regressions/   merges bug_fixes.rs + audit_*.rs family
  common/
```

### Shared helpers — good foundation, gaps to fill
`tests/common/mod.rs` well-adopted (~80% of files). Missing:
- `graveyard_creature_named(state, registry, name, owner)` — hand-rolled 20+ times
- `attach_equipment(state, equip, creature)` — repeated in equipment tests
- `process_triggers_with_choices()` — 53-line helper buried in `apnap.rs`
- `test_registry!` macro to replace per-binary `fn registry()` duplication

### Brittle/prescriptive tests — contained (~30 examples)
- Hardcoded `ids::CardId(170)` in audit files
- Asserting internal counter maps instead of `effective_power`
- `name == "Wolf"` string matches on tokens
- 4 `eprintln!` debug statements in `audit_bugs2.rs`

### Coverage gaps — measured 2026-04-14 (overall 85.07% lines)
- **Real gaps:** `view.rs` 71.8%, `engine.rs` 75.8% (881 uncovered lines — biggest absolute), `triggers.rs` 81.7%, `cards/helpers.rs` 62.6%.
- **Hypotheses disproven:** `state.rs` 91.4%, `stack.rs` 92.5%, `destruction.rs` 89.0% are all well-covered despite size.
- **Under-tested card cluster:** ISD dual lands (`sulfur_falls` 28.8%, `isolated_chapel` 28.8%, `hinterland_harbor` 63.0%, `woodland_cemetery` 64.4%) share a pattern only tested once each.
- Full report: [reports/test_suite_audit.md](../../reports/test_suite_audit.md)

## Execution order

Low-risk → high-risk, measurable before invasive.

1. ~~**Measure baseline.**~~ ✅ 2026-04-14. Overall 85.07% lines; see `reports/test_suite_audit.md`.
2. **Low-risk cleanup** (one commit each):
   - Strip `eprintln!` from `audit_bugs2.rs`
   - Replace hardcoded `CardId(n)` with registry lookups
   - Convert a handful of counter-map asserts to view-layer asserts
3. **Helper extraction** (one commit each):
   - Add `graveyard_creature_named`, `attach_equipment` to `common/mod.rs`
   - Promote `process_triggers_with_choices` from `apnap.rs`
   - Introduce a `test_registry!` macro
4. **Close real coverage gaps** (coverage-informed):
   - Add tests for `view.rs` (48 uncovered lines)
   - Inspect `engine.rs` HTML report; close the 881 uncovered lines (likely error/edge paths)
   - Parameterized test over ISD dual lands instead of per-card
   - Cover `cards/helpers.rs` (62.6%)
5. **Reorganize files** — only after above. High-churn; do once with a clear target.

## Coverage tooling choice

`cargo-llvm-cov`:
- Works on macOS (tarpaulin is Linux-first)
- More accurate source mapping than tarpaulin
- LCOV output for future CI integration

CI policy when added: publish coverage as an artifact first; add "coverage may not decrease" gate only once the baseline stabilizes. No numeric minimum target initially.
