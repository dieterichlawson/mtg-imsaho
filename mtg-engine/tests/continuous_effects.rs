//! One walk over continuous effects, and one way to say "as long as".
//!
//! `ContinuousEffect` had four `Conditional*` variants that differed from
//! their unconditional twins by one field, and the engine walked the
//! battlefield five separate times to read them: `has_continuous_effect`,
//! `count_continuous_effect`, `has_conditional_prevent`,
//! `has_conditional_keyword`, and the loop inside `continuous_pt_mods`.
//! `When { condition, effect }` wraps instead of duplicating, so a condition
//! can qualify any effect and there is one place that evaluates it.

mod common;
use common::*;
use mtg_engine::actions::Target;
use mtg_engine::cards::CardRegistry;
use mtg_engine::types::*;

// ---------------------------------------------------------------------------
// The conditional effects that already existed still work through the wrapper
// ---------------------------------------------------------------------------

/// Bonds of Faith: "+2/+2 as long as it's a Human. Otherwise it can't attack
/// or block." One aura, three conditional effects, two different conditions.
#[test]
fn bonds_of_faith_reads_the_condition_in_both_directions() {
    let reg = registry();

    let mut state = game_at_step(Step::PrecombatMain, P0);
    let human = named_permanent(&mut state, &reg, "Doomed Traveler", P0); // Human Soldier 1/1
    let bonds = named_permanent(&mut state, &reg, "Bonds of Faith", P0);
    state.get_object_mut(bonds).unwrap().attached_to = Some(human);

    assert_eq!(state.effective_power(human, &reg), Some(3),
        "a Human gets +2/+2");
    assert!(state.can_attack(human, &reg), "and is not restricted");
    assert!(state.can_block(human, &reg));

    let mut state = game_at_step(Step::PrecombatMain, P0);
    let beast = ready_creature(&mut state, P0, 2, 2); // no subtypes
    let bonds = named_permanent(&mut state, &reg, "Bonds of Faith", P0);
    state.get_object_mut(bonds).unwrap().attached_to = Some(beast);

    assert_eq!(state.effective_power(beast, &reg), Some(2),
        "a non-Human gets no bonus");
    assert!(!state.can_attack(beast, &reg), "and can't attack");
    assert!(!state.can_block(beast, &reg), "or block");
}

/// Angelic Overseer: "has hexproof and indestructible as long as you control
/// a Human." Two conditional keyword grants off one condition.
#[test]
fn angelic_overseer_gains_and_loses_its_keywords_with_the_board() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let overseer = named_permanent(&mut state, &reg, "Angelic Overseer", P0);
    assert!(!state.has_keyword(overseer, Keyword::Hexproof, &reg),
        "no Human out, no hexproof");

    let human = named_permanent(&mut state, &reg, "Doomed Traveler", P0);
    assert!(state.has_keyword(overseer, Keyword::Hexproof, &reg));
    assert!(state.has_keyword(overseer, Keyword::Indestructible, &reg));

    state.move_object(human, Zone::Graveyard, &reg);
    assert!(!state.has_keyword(overseer, Keyword::Hexproof, &reg),
        "the condition is re-read, not snapshotted");
}

/// Manor Gargoyle: "has indestructible as long as it has defender." The
/// condition asks about a keyword, which is what makes eager evaluation of
/// conditions unsafe — `has_keyword` would re-enter itself. The walk tests
/// what the caller asked for *before* the condition, so it terminates.
#[test]
fn manor_gargoyle_condition_on_a_keyword_does_not_recurse() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let gargoyle = named_permanent(&mut state, &reg, "Manor Gargoyle", P0);
    assert!(state.has_keyword(gargoyle, Keyword::Defender, &reg));
    assert!(state.has_keyword(gargoyle, Keyword::Indestructible, &reg),
        "indestructible while it has defender");

    // Asking about an unrelated keyword must not evaluate the condition into
    // an infinite regress either.
    assert!(!state.has_keyword(gargoyle, Keyword::Trample, &reg));
}

// ---------------------------------------------------------------------------
// What the unification opened up
// ---------------------------------------------------------------------------

/// A condition can now qualify any continuous effect, not the four somebody
/// happened to write a `Conditional*` variant for. `CantBeBlocked` never had
/// one.
#[test]
fn a_condition_can_wrap_an_effect_that_never_had_a_conditional_twin() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);

    let attacker = ready_creature(&mut state, P0, 2, 2);
    let enchantment = ready_creature(&mut state, P0, 0, 1);
    state.get_object_mut(enchantment).unwrap().instance_continuous_effects = Some(vec![
        ContinuousEffect::when(
            EffectCondition::YouControlSubtype("Human".into()),
            ContinuousEffect::CantBeBlocked { scope: EffectScope::Global(CreatureFilter::ControlledByYou) },
        ),
    ]);

    assert!(!state.cant_be_blocked(attacker, &reg),
        "no Human out, so the condition does not hold");

    named_permanent(&mut state, &reg, "Doomed Traveler", P0);
    assert!(state.cant_be_blocked(attacker, &reg),
        "with a Human out the wrapped effect applies");
}

/// Instance-level effects go through the same walk as printed ones, so a
/// conditional granted at runtime works. `continuous_pt_mods` used to read
/// instance effects with its own loop that handled `ModifyPT` but not
/// `ConditionalModifyPT`, so this was silently impossible.
#[test]
fn a_conditional_pt_bonus_granted_at_runtime_applies() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 2, 2);
    let anthem = ready_creature(&mut state, P0, 0, 1);
    state.get_object_mut(anthem).unwrap().instance_continuous_effects = Some(vec![
        ContinuousEffect::when(
            EffectCondition::YouControlSubtype("Human".into()),
            ContinuousEffect::ModifyPT { power: 3, toughness: 3, scope: EffectScope::Global(CreatureFilter::ControlledByYou) },
        ),
    ]);

    assert_eq!(state.effective_power(creature, &reg), Some(2),
        "condition does not hold yet");
    named_permanent(&mut state, &reg, "Doomed Traveler", P0);
    assert_eq!(state.effective_power(creature, &reg), Some(5),
        "an instance-level conditional P/T bonus applies like a printed one");
}

// ---------------------------------------------------------------------------
// Structural guard
// ---------------------------------------------------------------------------

/// There is one walk over the battlefield's continuous effects.
///
/// Five functions used to iterate `objects.values()` looking for effects, each
/// re-deriving "on the battlefield", "instance effects override the face",
/// "does the scope cover this object". A sixth and seventh lived in
/// `combat.rs` and `mana_sources.rs`. Anything that needs continuous effects
/// goes through `walk_effects` (or `global_effects`, for the rule
/// modifications that have no scope).
#[test]
fn continuous_effects_are_read_in_one_place() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for e in std::fs::read_dir(dir).expect("readable").flatten() {
            let p = e.path();
            if p.is_dir() { walk(&p, out); }
            else if p.extension().is_some_and(|x| x == "rs") { out.push(p); }
        }
    }
    walk(&src, &mut files);

    // Reading an object's own effects is fine; walking every permanent looking
    // for effects that apply elsewhere is what must go through `walk_effects`.
    let mut offenders = Vec::new();
    for f in &files {
        let rel = f.to_string_lossy().replace('\\', "/");
        if rel.ends_with("src/state.rs") {
            continue; // where the one walk lives
        }
        let text = std::fs::read_to_string(f).expect("readable");
        for (n, line) in text.lines().enumerate() {
            if line.contains("effect_applies_to(") {
                offenders.push(format!("{rel}:{}: {}", n + 1, line.trim()));
            }
        }
    }
    assert!(offenders.is_empty(),
        "scope resolution belongs to GameState::walk_effects, not to each caller:\n{}",
        offenders.join("\n"));
}

// -------------------------------------------------------------------------
// From the bug-audit files, re-filed by the rule each one exercises.
// -------------------------------------------------------------------------

/// Bug: Bonds of Faith snapshots the Human check at ETB.
/// Oracle: "gets +2/+2 as long as it's a Human. Otherwise, it can't attack or block."
/// If the creature later stops being a Human (e.g., transforms), the effect
/// should switch, but it doesn't because `instance_continuous_effects` is set once.
#[test]
fn bug_bonds_of_faith_snapshot_instead_of_continuous() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Cloistered Youth is a Human 1/1 whose back face, Unholy Fiend, is a
    // Horror — transforming is the way a creature in this set actually stops
    // being a Human. (Overwriting `obj.subtypes` would not model anything the
    // engine does: outside transform, subtypes are only ever added to, so
    // `has_subtype` unions the object's runtime grants with its active face.)
    let human = named_permanent(&mut state, &registry, "Cloistered Youth", P0);

    // Cast Bonds of Faith on it
    let bonds = castable_spell(&mut state, &registry, "Bonds of Faith", P0);
    state = cast_and_resolve(&state, &registry, bonds, vec![Target::Object(human)]);

    // Fire ETB triggers so Bonds sets instance_continuous_effects
    mtg_engine::triggers::process_triggers(&mut state, &registry);

    // Verify the Human gets +2/+2 (base 1/1 -> 3/3)
    let p = state.effective_power(human, &registry).unwrap_or(0);
    assert_eq!(p, 3, "Human with Bonds should have power 3 (1 base + 2 buff)");

    // Transform into Unholy Fiend: no longer a Human.
    mtg_engine::cards::helpers::apply_transform(&mut state, human, &registry);
    assert!(!state.has_subtype(human, "Human", &registry),
        "test precondition: Unholy Fiend is a Horror, not a Human");

    // The "as long as" condition is no longer true, so the effect must switch
    // from +2/+2 to can't-attack-or-block. Unholy Fiend's printed power is 3,
    // so 3 means the buff is gone and 5 would mean Bonds snapshotted the Human
    // check at ETB and never re-read it.
    let p_after = state.effective_power(human, &registry).unwrap_or(0);
    assert_eq!(p_after, 3,
        "a non-Human should lose the +2/+2 from Bonds — power {p_after} \
         (3 = Unholy Fiend's printed power, 5 = buff wrongly still applied)");
}

/// Ruling: "Once the enchanted creature has been declared as an attacking or
/// blocking creature, causing it to stop being a Human won't remove it from
/// combat. It will lose the +2/+2 bonus, however."
///
/// Both halves in one board. "Can't attack" is a restriction on *declaring* an
/// attacker (CR 508.1), so it has nothing to say to a creature already
/// attacking; the +2/+2 is a continuous effect read live, so it goes the moment
/// the condition stops holding.
#[test]
fn bonds_of_faith_loses_its_bonus_mid_combat_without_removing_the_attacker() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let youth = named_permanent(&mut state, &reg, "Cloistered Youth", P0);
    let bonds = named_permanent(&mut state, &reg, "Bonds of Faith", P0);
    state.get_object_mut(bonds).unwrap().attached_to = Some(youth);
    assert_eq!(state.effective_power(youth, &reg), Some(3),
        "test precondition: a 1/1 Human with +2/+2");

    submit_declare_attackers(&mut state, &[(youth, P1)], &reg);
    assert!(state.combat.as_ref().is_some_and(|c| c.attackers.contains_key(&youth)),
        "test precondition: it was declared as an attacker");

    // It stops being a Human.
    mtg_engine::cards::helpers::apply_transform(&mut state, youth, &reg);
    assert!(!state.has_subtype(youth, "Human", &reg),
        "test precondition: Unholy Fiend is a Horror");

    assert!(state.combat.as_ref().is_some_and(|c| c.attackers.contains_key(&youth)),
        "\"can't attack\" restricts declaring an attacker, so it does not pull \
         one already attacking out of combat");
    assert_eq!(state.effective_power(youth, &reg), Some(3),
        "but the +2/+2 is gone — 3 is Unholy Fiend's printed power, 5 would be \
         the bonus still applying to a creature that is no longer a Human");
    assert!(!state.can_attack(youth, &reg),
        "and it could not be declared again");
}

/// A targeted until-end-of-turn pump changes exactly its target's numbers,
/// by exactly the printed amounts (Moment of Heroism: +2/+2). The full
/// mutation sweep showed the targeted `ModifyPT` accumulation in
/// `effective_power`/`effective_toughness` — the guard that matches the
/// target and the `+=` itself — was pinned by no test.
#[test]
fn a_targeted_pump_changes_only_its_target_by_the_printed_amount() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let pumped = ready_creature(&mut state, P0, 2, 3);
    let bystander = ready_creature(&mut state, P0, 2, 3);

    let heroism = castable_spell(&mut state, &reg, "Moment of Heroism", P0);
    let state = cast_and_resolve(&state, &reg, heroism, vec![Target::Object(pumped)]);

    assert_eq!(state.effective_power(pumped, &reg), Some(4), "2 + 2");
    assert_eq!(state.effective_toughness(pumped, &reg), Some(5), "3 + 2");
    assert_eq!(state.effective_power(bystander, &reg), Some(2),
        "the bystander is untouched");
    assert_eq!(state.effective_toughness(bystander, &reg), Some(3));
}
