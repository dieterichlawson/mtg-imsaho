//! One replacement mechanism (CR 614), applied from one place.
//!
//! There used to be seven: a closed `ReplacementEffect` enum in the engine
//! plus six bespoke `CardBehavior` hooks, each consulted from exactly one call
//! site. Nothing could apply a replacement the engine's author had not thought
//! to ask for at that spot, and neither CR 614.5 (an effect applies at most
//! once to an event) nor CR 616.1 (replacements are evaluated against the
//! state *before* the event) was expressible.

mod common;
use common::*;
use mtg_engine::events::DamageTarget;
use mtg_engine::replacement::{ReplaceableEvent, apply};
use mtg_engine::types::*;
use mtg_engine::actions::Target;

// ---------------------------------------------------------------------------
// CR 614.1d — "enters tapped"
// ---------------------------------------------------------------------------

/// The check lands: "enters tapped unless you control a Mountain or a Plains".
#[test]
fn a_check_land_enters_tapped_only_without_its_land_types() {
    let reg = registry();
    for (land, enabler) in [
        ("Clifftop Retreat", "Mountain"),
        ("Isolated Chapel", "Plains"),
        ("Sulfur Falls", "Island"),
        ("Woodland Cemetery", "Forest"),
        ("Hinterland Harbor", "Forest"),
    ] {
        let mut state = game_at_step(Step::PrecombatMain, P0);
        let alone = spell_in_hand(&mut state, &reg, land, P0);
        state.move_object(alone, Zone::Battlefield, &reg);
        assert!(state.get_object(alone).unwrap().tapped,
            "{land} alone enters tapped");

        let mut state = game_at_step(Step::PrecombatMain, P0);
        named_permanent(&mut state, &reg, enabler, P0);
        let with = spell_in_hand(&mut state, &reg, land, P0);
        state.move_object(with, Zone::Battlefield, &reg);
        assert!(!state.get_object(with).unwrap().tapped,
            "{land} enters untapped with a {enabler} out");
    }
}

/// Essence of the Wild: "Creatures you control enter as copies of this
/// creature." CR 614.1d — a replacement effect, so it applies however the
/// creature arrives. It used to run out of `on_resolve`, which meant only
/// creatures cast from hand were affected; a token created by an ability, or a
/// creature reanimated, walked straight past it.
#[test]
fn essence_of_the_wild_applies_to_a_token_it_did_not_resolve() {
    let registry = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Just being on the battlefield is enough — no hook is fired here on
    // purpose, because a replacement effect is not a trigger.
    let _eotw = named_permanent(&mut state, &registry, "Essence of the Wild", P0);

    // Create a token — it should enter as a copy of Essence of the Wild
    let token = state.create_token_with_subtypes(
        "Spirit", P0, 1, 1,
        vec![Color::White],
        vec![CardType::Creature],
        vec![Keyword::Flying],
        vec!["Spirit".into()],
        &registry,
    )[0];

    // The token should be a 6/6 copy of Essence of the Wild
    let token_power = state.get_object(token).and_then(|o| o.power).unwrap_or(0);

    assert_eq!(token_power, 6,
        "Token should enter as 6/6 Essence of the Wild copy, got power {token_power}");
}

// ---------------------------------------------------------------------------
// CR 616.1 — replacements read the state BEFORE the event
// ---------------------------------------------------------------------------

/// Unbreathing Horde enters "with a +1/+1 counter for each other Zombie you
/// control and each Zombie card in your graveyard". Reanimated, it is still in
/// the graveyard when the replacement is evaluated, so it counts itself.
///
/// This is also the case that proves a permanent's replacement about its own
/// arrival has to be consulted wherever it currently is, not only from the
/// battlefield — the unification initially got that wrong and this caught it.
#[test]
fn a_replacement_about_your_own_arrival_applies_from_any_zone() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    for _ in 0..3 {
        named_card_in_graveyard(&mut state, &reg, "Walking Corpse", P0);
    }
    let horde = named_card_in_graveyard(&mut state, &reg, "Unbreathing Horde", P0);
    state.move_object(horde, Zone::Battlefield, &reg);

    assert_eq!(counters_of(&state, horde, CounterType::PlusOnePlusOne), 4,
        "three Zombie cards in the graveyard plus the Horde itself, which is \
         still in the graveyard when the replacement is evaluated");
}

/// Dearly Departed does it from the graveyard, to somebody else — the case
/// `replacement_zones` exists for.
#[test]
fn a_replacement_can_apply_from_the_graveyard_to_another_permanent() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    named_card_in_graveyard(&mut state, &reg, "Dearly Departed", P0);

    let human = spell_in_hand(&mut state, &reg, "Avacyn's Pilgrim", P0);
    state.move_object(human, Zone::Battlefield, &reg);
    assert_eq!(counters_of(&state, human, CounterType::PlusOnePlusOne), 1,
        "an entering Human gets a +1/+1 counter from Dearly Departed");

    let nonhuman = spell_in_hand(&mut state, &reg, "Walking Corpse", P0);
    state.move_object(nonhuman, Zone::Battlefield, &reg);
    assert_eq!(counters_of(&state, nonhuman, CounterType::PlusOnePlusOne), 0,
        "a Zombie is not a Human");

    // "each Human creature **you control**" — the graveyard the card is in is
    // its owner's (CR 404.1), and only that player's Humans are covered.
    let theirs = spell_in_hand(&mut state, &reg, "Avacyn's Pilgrim", P1);
    state.move_object(theirs, Zone::Battlefield, &reg);
    assert_eq!(counters_of(&state, theirs, CounterType::PlusOnePlusOne), 0,
        "the opponent's Human is not one you control");
}

/// Ruling: "The effect is cumulative. Human creatures you control will enter
/// with a +1/+1 counter for each Dearly Departed in your graveyard."
///
/// The counterpart to `a_replaced_event_is_not_replaced_again` below. An
/// effect that *modifies* an event passes the modified event to the next
/// candidate, so two of them each apply once; CR 614.5 stops one effect
/// applying twice to the same event, not two effects applying once each.
#[test]
fn each_dearly_departed_in_the_graveyard_adds_its_own_counter() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    named_card_in_graveyard(&mut state, &reg, "Dearly Departed", P0);
    named_card_in_graveyard(&mut state, &reg, "Dearly Departed", P0);

    let human = spell_in_hand(&mut state, &reg, "Avacyn's Pilgrim", P0);
    state.move_object(human, Zone::Battlefield, &reg);
    assert_eq!(counters_of(&state, human, CounterType::PlusOnePlusOne), 2,
        "one counter for each Dearly Departed in the graveyard");
}

// ---------------------------------------------------------------------------
// CR 614.5 — an effect applies at most once to a given event
// ---------------------------------------------------------------------------

/// Two Undead Alchemists: the first replaces the damage, and the event is then
/// gone, so the second has nothing to replace. The player mills 2, not 4.
#[test]
fn a_replaced_event_is_not_replaced_again() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);
    named_permanent(&mut state, &reg, "Undead Alchemist", P0);
    named_permanent(&mut state, &reg, "Undead Alchemist", P0);
    let zombie = named_permanent(&mut state, &reg, "Walking Corpse", P0);

    for _ in 0..10 {
        let id = state.create_object(
            reg.get_id_by_name("Chapel Geist").unwrap(), P1, Zone::Library, None, None);
        state.get_player_mut(P1).library_order.push(id);
    }
    let before = state.get_player(P1).library_order.len();

    let outcome = apply(&mut state, ReplaceableEvent::DealsDamage {
        source: zombie,
        target: DamageTarget::Player(P1),
        amount: 2,
        combat: true,
    }, &reg);

    assert!(outcome.is_none(), "the damage was replaced");
    assert_eq!(before - state.get_player(P1).library_order.len(), 2,
        "one replacement, so two cards — not four");
}

/// Two Parallel Lives do compound, because each modifies rather than replaces:
/// one token becomes two, then four.
#[test]
fn modifying_replacements_compound() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    named_permanent(&mut state, &reg, "Parallel Lives", P0);
    named_permanent(&mut state, &reg, "Parallel Lives", P0);

    let before = state.objects_in_zone(Zone::Battlefield, P0).iter().filter(|o| o.is_token).count();
    state.create_token_with_subtypes(
        "Spirit", P0, 1, 1, vec![Color::White], vec![CardType::Creature],
        vec![Keyword::Flying], vec!["Spirit".into()], &reg);
    let made = state.objects_in_zone(Zone::Battlefield, P0).iter()
        .filter(|o| o.is_token).count() - before;

    assert_eq!(made, 4, "two doublers: one token becomes two, then four");
}

// ---------------------------------------------------------------------------
// The guard.
// ---------------------------------------------------------------------------

/// Every card that replaces anything does it through `replace_event`.
///
/// The point of the unification is that there is one hook and one place it is
/// applied. This fails the build if a second mechanism reappears — which is
/// how the previous seven accumulated.
#[test]
fn replacement_has_exactly_one_mechanism() {
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

    // The hooks this replaced. If any comes back, so has the fragmentation.
    // Trailing `(` so this matches the hooks themselves and not the helpers
    // named after them — `helpers::enters_tapped_unless` is how a card says
    // "enters tapped" through the one mechanism, not a second one.
    const GONE: &[&str] = &[
        "fn replacement_effects(",
        "fn enters_tapped(",
        "fn entering_with_counters(",
        "fn modify_creature_entering_counters(",
        "fn entering_modifier_zones(",
        "fn replace_combat_damage_to_player(",
        "fn enters_as_copy(",
        "enum ReplacementEffect",
    ];
    let mut found = Vec::new();
    for f in &files {
        let text = std::fs::read_to_string(f).expect("readable");
        for (n, line) in text.lines().enumerate() {
            for gone in GONE {
                if line.contains(gone) {
                    found.push(format!("{}:{}: {}", f.display(), n + 1, line.trim()));
                }
            }
        }
    }
    assert!(found.is_empty(),
        "replacement effects have fragmented again — these are the hooks the \
         single `replace_event` mechanism replaced:\n{}", found.join("\n"));
}


/// Ruling: "when determining whether a creature entering under your control
/// should get a +1/+1 counter, you'll simply look at what the creature will
/// look like on the battlefield. You'll consider any effects affecting a
/// creature entering under your control."
///
/// A token entering as a copy of a Human is a Human as it enters (CR 706.2),
/// so it is covered — even though nothing about the token before the copy
/// effect applied said "Human".
#[test]
fn a_token_entering_as_a_copy_of_a_human_gets_the_counter() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    named_card_in_graveyard(&mut state, &reg, "Dearly Departed", P0);

    let original = named_permanent(&mut state, &reg, "Avacyn's Pilgrim", P0);
    assert!(state.has_subtype(original, "Human", &reg), "test precondition");

    let spell = castable_spell(&mut state, &reg, "Cackling Counterpart", P0);
    let state = cast_and_resolve(&state, &reg, spell, vec![Target::Object(original)]);

    let token = state.objects.values()
        .find(|o| o.is_token && o.zone == Zone::Battlefield)
        .map(|o| o.id)
        .expect("Cackling Counterpart makes a token copy");
    assert!(state.has_subtype(token, "Human", &reg), "the copy is a Human");
    assert_eq!(counters_of(&state, token, CounterType::PlusOnePlusOne), 1,
        "so Dearly Departed sees it entering");
}

/// Mutation-motivated (#44 backlog, `enters_tapped_unless`): the helper's
/// three-way OR guard — wrong object, already tapped, or the untapped
/// condition holds — each independently means "leave the event alone". The
/// `|| -> &&` survivor tapped the wrong object's entry and ignored the
/// unless-condition; each arm gets its own assertion.
#[test]
fn enters_tapped_unless_taps_only_its_own_conditional_entry() {
    use mtg_engine::cards::helpers::enters_tapped_unless;
    use mtg_engine::ids::{ObjectId, PlayerId};
    use mtg_engine::replacement::{EnteringPermanent, ReplaceableEvent, Replacement};

    let me = ObjectId(7);
    let someone_else = ObjectId(8);
    let entering = |object: ObjectId, tapped: bool| {
        ReplaceableEvent::EntersBattlefield(EnteringPermanent {
            object,
            from: Some(Zone::Hand),
            controller: PlayerId(0),
            tapped,
            counters: vec![],
            copy_of: None,
        })
    };

    // The plain case: my own untapped entry, condition false — tap it.
    let replaced = enters_tapped_unless(me, &entering(me, false), || false);
    assert!(matches!(replaced,
        Some(Replacement::Modified(ReplaceableEvent::EntersBattlefield(e))) if e.tapped),
        "the permanent enters tapped when its condition does not hold");

    // The unless-condition holds — the event is left alone.
    assert!(enters_tapped_unless(me, &entering(me, false), || true).is_none(),
        "when the condition holds it enters untapped (CR 614.1d)");

    // Someone else's entry is never this card's business.
    assert!(enters_tapped_unless(me, &entering(someone_else, false), || false).is_none(),
        "another permanent's entry is not modified");

    // An entry already tapped needs nothing done.
    assert!(enters_tapped_unless(me, &entering(me, true), || false).is_none(),
        "an already-tapped entry is left alone");
}
