---
id: instigator_gang-02
status: fixed
card: Instigator Gang
audit_run_id: 2026-04-19-instigator_gang-audit
audit_model: sonnet
audit_tokens: 32082
audit_duration: 573
fixed_sha: 3e83233
fixed_at: 2026-08-24T01:31:30Z
test_file: mtg-engine/tests/attacking_creatures_anthem.rs
fix_note: Modelled as a static ContinuousEffect::ModifyPT over CreatureFilter::Attacking rather than an attack trigger pushing an until-end-of-turn buff.
---

## Audit Finding

**Oracle text:**
> Attacking creatures you control get +1/+0.
(Wildblood Pack) Attacking creatures you control get +3/+0.

**Code:**
> // Watch ALL creatures attacking (not just self).
TriggeredAbilityDef {
    kind: TriggerKind::AnyCreatureAttacks,
    description: "attacking creatures you control get +1/+0".into(),
target_requirement: None,
},
...
state.until_end_of_turn.push(
    crate::state::TemporaryEffect::ModifyPTWhileSourceInPlay {
        target: attacker_id,
        source: self_id,
        power_mod: bonus,
        toughness_mod: 0,
    }
);

**Description:**
Oracle text 'Attacking creatures you control get +X/+0' is a static ability (CR 604.1) that creates a continuous effect (CR 611.3b) applying to all creatures currently in the attacking state. The implementation models it instead as an `AnyCreatureAttacks` triggered ability that fires once per attacker and pushes a `TemporaryEffect::ModifyPTWhileSourceInPlay` into `state.until_end_of_turn`. This diverges from the rules in three ways. First, the effect persists until the cleanup step (CR 514.2), not until the creature ceases to be attacking (CR 506.4b); creatures that attacked this turn therefore retain +1/+0 (or +3/+0) during the post-combat main phase and end step, inflating their apparent power after combat ends. Second, a creature that enters the battlefield already attacking through a 'put onto the battlefield tapped and attacking' effect never triggers `AnyCreatureAttacks` and receives no bonus, even though it is an attacking creature the player controls. Third, if Instigator Gang enters the battlefield after attackers have been declared (e.g., via flash or other effects), existing attacking creatures do not receive the bonus because no new attack trigger fires. A static continuous effect evaluated in the layer system would handle all three cases correctly.

**Engine path:** mtg-engine/src/cards/isd/instigator_gang.rs:48

**Required check:** 8h

**Affected cards:**
- Instigator Gang / Wildblood Pack

## Tests

### instigator_gang_buff_persists_post_combat
Scenario: A 2/2 creature attacks while Instigator Gang is on the battlefield; after the end-of-combat step, during the second main phase, the creature's power is verified to still read 3 (incorrectly retaining +1/+0 past the end of combat).

### instigator_gang_misses_etb_attacking_creature
Scenario: An effect puts a creature token onto the battlefield tapped and attacking while Instigator Gang is on the battlefield; the token deals combat damage equal to its printed power (no +1/+0 bonus), demonstrating that the AnyCreatureAttacks trigger never fired for it.

