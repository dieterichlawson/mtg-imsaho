---
id: cackling_counterpart-03
status: fixed
card: Cackling Counterpart
audit_run_id: 2026-04-18-cackling_counterpart-audit
audit_model: sonnet
audit_tokens: 15636
audit_duration: 272
fixed_sha: 778ed4738894357d762d118ea082f892dcb0d2c4
fixed_at: 2026-08-23T23:24:11Z
test_file: mtg-engine/tests/copy_effects.rs
fix_note: coverage added: token copy of a creature, and the copied creature's ETB firing on the token
---

## Audit Finding

**Oracle text:**
> The token copies exactly what was printed on the original creature and nothing else (unless that creature is copying something else or is a token; see below). It doesn't copy whether that creature is tapped or untapped, whether it has any counters on it or Auras and Equipment attached to it, or any non-copy effects that have changed its power, toughness, types, color, or so on.

**Code:**
> let token_id = state.create_token_copy(*target_id, controller, registry);

**Description:**
The basic resolve path of Cackling Counterpart — creating a token copy of a targeted creature you control — has zero test coverage. No test in `mtg-engine/tests/` calls Cackling Counterpart's `on_resolve` or exercises it end-to-end. This ruling covers the primary interaction (non-token real-card copies) that the implementation handles correctly via registry lookup, but without a test there is no regression protection for the basic case.

**Required check:** 8j

## Tests

### cackling_counterpart_creates_token_copy_of_target_creature
Scenario: Cackling Counterpart resolves targeting a 2/2 Grizzly Bears; a new token with name 'Grizzly Bears', power 2, toughness 2 should appear on the battlefield under the caster's control.

### cackling_counterpart_flashback_exiles_spell
Scenario: Cackling Counterpart is cast from the graveyard via its flashback cost {5}{U}{U}; after resolving it should be in the Exile zone, not the graveyard.

