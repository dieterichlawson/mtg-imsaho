//! Tests for Innistrad Tier 2 cards: targeted removal, bounce, fight,
//! permanent destruction, and counter variants.
//!
//! Cards covered (12), so this is greppable by name as well as by rule:
//!
//! - Bramblecrush
//! - Dissipate
//! - Frightful Delusion
//! - Geistflame
//! - Lost in the Mist
//! - Naturalize
//! - Prey Upon
//! - Rebuke
//! - Silent Departure
//! - Smite the Monstrous
//! - Urgent Exorcism
//! - Victim of Night

mod common;

use common::*;
use mtg_engine::actions::{Action, Target};
use mtg_engine::engine;
use mtg_engine::sba::check_state_based_actions;
use mtg_engine::types::*;
// ── Simple damage spells ────────────────────────────────────────────

// Bump in the Night's 3-life-drain to a player is covered by the
// parametric `direct_damage_spells_drain_player_life` in spells.rs.
// Flashback behavior is covered in flashback.rs.

#[test]
fn geistflame_deals_1_damage() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P1, 2, 2);
    let card = castable_spell(&mut state, &reg, "Geistflame", P0);

    state = cast_and_resolve(&state, &reg, card, vec![Target::Object(creature)]);

    assert_eq!(state.get_object(creature).unwrap().damage_marked, 1);
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Battlefield,
        "2/2 with 1 damage should survive");
}

// Brimstone Volley's 3-damage-to-player case is covered by the
// parametric `direct_damage_spells_drain_player_life` in spells.rs.

// ── Counter variants ────────────────────────────────────────────────

/// Dissipate counters and exiles the spell (not graveyard).
#[test]
fn dissipate_counters_and_exiles() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // P0 casts a creature spell.
    let tusker = castable_spell(&mut state, &reg, "Kalonian Tusker", P0);

    state = cast_onto_stack(&state, &reg, tusker, vec![]);

    // P1 casts Dissipate targeting the Tusker on the stack.
    let diss = castable_spell(&mut state, &reg, "Dissipate", P1);
    state.priority_player = Some(P1);

    state = cast_and_resolve(&state, &reg, diss, vec![Target::Object(tusker)]);

    assert_eq!(state.get_object(tusker).unwrap().zone, Zone::Exile,
        "Dissipate should exile the countered spell, not put it in graveyard");
    assert_eq!(state.get_object(diss).unwrap().zone, Zone::Graveyard);
}

/// Frightful Delusion counters and forces a discard.
#[test]
fn frightful_delusion_counters_and_discards() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Give P0 a card in hand (to be discarded).
    let hand_card = spell_in_hand(&mut state, &reg, "Mountain", P0);

    // P0 casts a creature.
    let bears = castable_spell(&mut state, &reg, "Grizzly Bears", P0);

    state = cast_onto_stack(&state, &reg, bears, vec![]);

    // P1 casts Frightful Delusion.
    let fd = castable_spell(&mut state, &reg, "Frightful Delusion", P1);
    state.priority_player = Some(P1);

    state = cast_and_resolve(&state, &reg, fd, vec![Target::Object(bears)]);

    // CR 608.2g: P0 is asked whether to pay {1}. Their only Mountain is in
    // hand, so declining is the only legal answer.
    state = engine::submit_action(&state, &Action::ResolveChoice {
        choice: mtg_engine::actions::ResolvedChoice::PayDecision(false),
    }, &reg);

    assert_eq!(state.get_object(bears).unwrap().zone, Zone::Graveyard,
        "Spell should be countered");
    // P0's hand card should have been discarded.
    assert_eq!(state.get_object(hand_card).unwrap().zone, Zone::Graveyard,
        "Controller of countered spell should discard a card");
}

// ── What a removal spell is allowed to point at ─────────────────────

/// A candidate for a removal spell to consider, built fresh per row.
enum Candidate {
    /// A vanilla creature of this size.
    Creature(i32, i32),
    /// A named card put onto the battlefield (for its subtypes).
    Named(&'static str),
    /// A basic land.
    Land,
    /// An Aura, which needs a creature to enchant — so this also supplies the
    /// creature the row's "illegal" side uses.
    Enchantment,
}

fn place(state: &mut mtg_engine::state::GameState, reg: &mtg_engine::cards::CardRegistry, c: &Candidate) -> ObjectId {
    match *c {
        Candidate::Creature(p, t) => ready_creature(state, P1, p, t),
        Candidate::Named(name) => named_permanent(state, reg, name, P1),
        Candidate::Land => {
            let id = reg.get_id_by_name("Forest").unwrap();
            let land = state.create_object(id, P1, Zone::Battlefield, None, None);
            state.get_object_mut(land).unwrap().summoning_sick = false;
            land
        }
        Candidate::Enchantment => {
            let creature = ready_creature(state, P1, 2, 2);
            let pac = castable_spell(state, reg, "Pacifism", P1);
            // The Aura's controller has to hold priority to pay for it.
            state.priority_player = Some(P1);
            *state = cast_and_resolve(state, reg, pac, vec![Target::Object(creature)]);
            pac
        }
    }
}

/// Targeted removal, and what each spell's text does and does not let it point
/// at. CR 601.2c: the engine only offers legal targets, so both halves are
/// observable from `legal_actions`.
///
/// Every row carries a legal candidate as well as an illegal one. Without it, a
/// row asserts only "this target is not offered" — which an engine that offered
/// nothing at all would satisfy. Three of the tests this replaces were exactly
/// that shape.
#[test]
fn targeted_removal_offers_the_targets_its_text_allows() {
    // (spell, something it may target, something it may not, what the rule is)
    const CASES: &[(&str, Candidate, Candidate, &str)] = &[
        ("Victim of Night", Candidate::Creature(2, 2), Candidate::Named("Markov Patrician"),
         "'creature that isn't a Vampire, Werewolf, or Zombie' — the Patrician is a Vampire"),
        ("Smite the Monstrous", Candidate::Creature(5, 5), Candidate::Creature(2, 2),
         "'creature with power 4 or greater'"),
        ("Naturalize", Candidate::Enchantment, Candidate::Creature(3, 3),
         "'target artifact or enchantment'"),
        ("Bramblecrush", Candidate::Land, Candidate::Creature(3, 3),
         "'target noncreature permanent'"),
        ("Urgent Exorcism", Candidate::Named("Chapel Geist"), Candidate::Creature(3, 3),
         "'target Spirit or enchantment' — the Geist is a Spirit"),
        ("Maw of the Mire", Candidate::Land, Candidate::Creature(3, 3),
         "'target land'"),
    ];

    for (spell_name, legal, illegal, rule) in CASES {
        let reg = registry();
        let mut state = game_at_step(Step::PrecombatMain, P0);

        let good = place(&mut state, &reg, legal);
        let bad = place(&mut state, &reg, illegal);
        state.priority_player = Some(P0);
        let spell = castable_spell(&mut state, &reg, spell_name, P0);

        let offered = offered_targets(&state, &reg, spell);
        assert!(offered.contains(&Target::Object(good)),
            "{spell_name} should be able to target it: {rule}. offered: {offered:?}");
        assert!(!offered.contains(&Target::Object(bad)),
            "{spell_name} should not be able to target it: {rule}");

        // And the spell does what it says to the target it was allowed.
        let state = cast_and_resolve(&state, &reg, spell, vec![Target::Object(good)]);
        assert_eq!(state.get_object(good).unwrap().zone, Zone::Graveyard,
            "{spell_name} destroys what it targeted");
    }
}

/// Rebuke ("Destroy target attacking creature") needs a combat to have a legal
/// target at all, so it gets its own setup — same rule as the table above.
#[test]
fn rebuke_only_targets_a_creature_that_is_attacking() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let attacker = ready_creature(&mut state, P0, 3, 3);
    let bystander = ready_creature(&mut state, P0, 2, 2);
    submit_declare_attackers(&mut state, &[(attacker, P1)], &reg);
    state.priority_player = Some(P1);

    let rebuke = castable_spell(&mut state, &reg, "Rebuke", P1);
    let offered = offered_targets(&state, &reg, rebuke);
    assert!(offered.contains(&Target::Object(attacker)), "the attacking creature is a legal target");
    assert!(!offered.contains(&Target::Object(bystander)), "the one that stayed home is not");

    let state = cast_and_resolve(&state, &reg, rebuke, vec![Target::Object(attacker)]);
    assert_eq!(state.get_object(attacker).unwrap().zone, Zone::Graveyard);
}

// ── Bounce ──────────────────────────────────────────────────────────

/// Silent Departure returns a creature to its owner's hand.
#[test]
fn silent_departure_bounces_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P1, 3, 3);

    let card = castable_spell(&mut state, &reg, "Silent Departure", P0);

    state = cast_and_resolve(&state, &reg, card, vec![Target::Object(creature)]);

    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Hand,
        "Creature should be returned to hand");
}

// ── Fight ───────────────────────────────────────────────────────────

/// Prey Upon: your creature fights their creature. Both deal damage.
#[test]
fn prey_upon_fight() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let mine = ready_creature(&mut state, P0, 3, 3);
    let theirs = ready_creature(&mut state, P1, 2, 2);

    let pu = castable_spell(&mut state, &reg, "Prey Upon", P0);

    state = cast_and_resolve(&state, &reg, pu, vec![Target::Object(mine), Target::Object(theirs)]);

    // 3/3 deals 3 to 2/2, 2/2 deals 2 to 3/3.
    assert_eq!(state.get_object(mine).unwrap().damage_marked, 2);
    assert_eq!(state.get_object(theirs).unwrap().damage_marked, 3);

    // SBA kills the 2/2.
    check_state_based_actions(&mut state, &reg);
    assert_eq!(state.get_object(theirs).unwrap().zone, Zone::Graveyard);
    assert_eq!(state.get_object(mine).unwrap().zone, Zone::Battlefield);
}

// ── Two-target spells ───────────────────────────────────────────────

/// Lost in the Mist counters a spell and bounces a permanent.
#[test]
fn lost_in_the_mist_counters_and_bounces() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // P1 has a creature on the battlefield.
    let creature = ready_creature(&mut state, P0, 3, 3);

    // P0 casts a spell.
    let bears = castable_spell(&mut state, &reg, "Grizzly Bears", P0);

    state = cast_onto_stack(&state, &reg, bears, vec![]);

    // P1 casts Lost in the Mist targeting the spell + the creature.
    let litm = castable_spell(&mut state, &reg, "Lost in the Mist", P1);
    state.priority_player = Some(P1);

    state = cast_and_resolve(&state, &reg, litm, vec![Target::Object(bears), Target::Object(creature)]);

    assert_eq!(state.get_object(bears).unwrap().zone, Zone::Graveyard,
        "Spell should be countered");
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Hand,
        "Permanent should be bounced to hand");
}

/// Scryfall ruling (2011-09-22): "Lost in the Mist targets both the spell and
/// the permanent. You can only cast it if you can choose legal targets for
/// both parts."
///
/// With a permanent on the battlefield but nothing on the stack, the spell
/// half has no legal target and the card is uncastable — not castable with the
/// bounce half alone.
#[test]
fn lost_in_the_mist_needs_a_target_for_both_halves() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    ready_creature(&mut state, P1, 3, 3);
    let litm = castable_spell(&mut state, &reg, "Lost in the Mist", P0);
    add_mana(&mut state, P0, &[(ManaType::Colorless, 3), (ManaType::Blue, 2)]);

    let casts: Vec<_> = mtg_engine::engine::legal_actions(&state, &reg).actions.iter()
        .filter(|a| matches!(a, Action::CastSpell { object_id, .. } if *object_id == litm))
        .cloned()
        .collect();
    assert!(casts.is_empty(),
        "nothing is on the stack, so \"counter target spell\" has no legal \
         target and the card cannot be cast at all; got {casts:?}");

    // The control: put a spell on the stack and it becomes castable, so the
    // assertion above is about the missing spell and not about mana.
    let bears = castable_spell(&mut state, &reg, "Grizzly Bears", P1);
    state.priority_player = Some(P1);
    let mut state = cast_onto_stack(&state, &reg, bears, vec![]);
    state.priority_player = Some(P0);
    add_mana(&mut state, P0, &[(ManaType::Colorless, 3), (ManaType::Blue, 2)]);
    assert!(mtg_engine::engine::legal_actions(&state, &reg).actions.iter()
        .any(|a| matches!(a, Action::CastSpell { object_id, .. } if *object_id == litm)),
        "with a spell on the stack both halves have a target");
}

// -------------------------------------------------------------------------
// Smite the Monstrous
// -------------------------------------------------------------------------

/// "Creature with power **4 or greater**" is about power now, not printed
/// power. Every existing case uses a printed value — the target table has a
/// 5/5 and a 2/2, and `resolution_time_checks.rs` shrinks a target by writing
/// `obj.power` directly — so an implementation reading the printed number
/// passes all of them.
///
/// Both directions, since either one alone is explained by the other:
/// a 3/3 buffed to 4 power is a legal target, and a 5/5 shrunk to 3 is not.
#[test]
fn smite_the_monstrous_reads_power_as_it_is_now() {
    let reg = registry();

    // Buffed up to 4: legal, and destroyed.
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let small = ready_creature(&mut state, P1, 3, 3);
    state.until_end_of_turn.push(mtg_engine::state::TemporaryEffect::ModifyPT {
        target: small, power_mod: 1, toughness_mod: 1,
    });
    assert_eq!(state.effective_power(small, &reg), Some(4), "test precondition");

    let smite = castable_spell(&mut state, &reg, "Smite the Monstrous", P0);
    assert!(offered_targets(&state, &reg, smite).contains(&Target::Object(small)),
        "printed 3 power, but 4 right now");
    let state = cast_and_resolve(&state, &reg, smite, vec![Target::Object(small)]);
    assert_eq!(state.get_object(small).unwrap().zone, Zone::Graveyard);

    // Shrunk below 4: not offered at all.
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let big = ready_creature(&mut state, P1, 5, 5);
    state.until_end_of_turn.push(mtg_engine::state::TemporaryEffect::ModifyPT {
        target: big, power_mod: -2, toughness_mod: 0,
    });
    assert_eq!(state.effective_power(big, &reg), Some(3), "test precondition");

    let smite = castable_spell(&mut state, &reg, "Smite the Monstrous", P0);
    assert!(!offered_targets(&state, &reg, smite).contains(&Target::Object(big)),
        "printed 5 power, but 3 right now");
}

// -------------------------------------------------------------------------
// Naturalize
// -------------------------------------------------------------------------

/// "Destroy target artifact **or** enchantment" — both halves. The table above
/// gives Naturalize one row, with an enchantment, so dropping `Artifact` from
/// the type list passes every existing test.
#[test]
fn naturalize_destroys_either_half_of_its_type_line() {
    let reg = registry();
    for name in ["Cobbled Wings", "Claustrophobia"] {
        let mut state = game_at_step(Step::PrecombatMain, P0);
        // Claustrophobia is an Aura, so it needs a creature to enchant.
        let host = ready_creature(&mut state, P1, 2, 2);
        let permanent = named_permanent(&mut state, &reg, name, P1);
        if name == "Claustrophobia" {
            state.get_object_mut(permanent).unwrap().attached_to = Some(host);
        }

        let nat = castable_spell(&mut state, &reg, "Naturalize", P0);
        assert!(offered_targets(&state, &reg, nat).contains(&Target::Object(permanent)),
            "{name} should be a legal target");

        let state = cast_and_resolve(&state, &reg, nat, vec![Target::Object(permanent)]);
        assert_eq!(state.get_object(permanent).unwrap().zone, Zone::Graveyard,
            "{name} should be destroyed");
    }
}

/// The requirement says nothing about creatures, so an artifact creature is a
/// legal target — same reading as Ancient Grudge, and the opposite of
/// Bramblecrush's "noncreature permanent".
#[test]
fn naturalize_can_target_an_artifact_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let gargoyle = named_permanent(&mut state, &reg, "Manor Gargoyle", P1);
    let plain_creature = ready_creature(&mut state, P1, 3, 3);

    let nat = castable_spell(&mut state, &reg, "Naturalize", P0);
    let offered = offered_targets(&state, &reg, nat);

    assert!(offered.contains(&Target::Object(gargoyle)),
        "an artifact creature is an artifact; offered {offered:?}");
    assert!(!offered.contains(&Target::Object(plain_creature)),
        "but a creature that is neither is not; offered {offered:?}");
}

// -------------------------------------------------------------------------
// Ancient Grudge
// -------------------------------------------------------------------------

/// "Destroy target artifact." Ancient Grudge appears in this suite only as a
/// prop in Snapcaster Mage's tests — nothing cast it or watched it do
/// anything, in either of its two ways of being cast.
#[test]
fn ancient_grudge_destroys_an_artifact_from_hand() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let artifact = named_permanent(&mut state, &reg, "Cobbled Wings", P1);
    let bystander = ready_creature(&mut state, P1, 2, 2);

    let grudge = castable_spell(&mut state, &reg, "Ancient Grudge", P0);
    let offered = offered_targets(&state, &reg, grudge);
    assert!(offered.contains(&Target::Object(artifact)), "the artifact; offered {offered:?}");
    assert!(!offered.contains(&Target::Object(bystander)),
        "a plain creature is not an artifact");

    let state = cast_and_resolve(&state, &reg, grudge, vec![Target::Object(artifact)]);
    assert_eq!(state.get_object(artifact).unwrap().zone, Zone::Graveyard);
    assert_eq!(state.get_object(grudge).unwrap().zone, Zone::Graveyard,
        "cast from hand, so it goes to the graveyard — where its flashback \
         waits");
}

/// "Target **artifact**" says nothing about creatures, so an artifact creature
/// is a legal target. This is the mirror of
/// `bramblecrush_cannot_target_an_artifact_that_is_also_a_creature`: the same
/// permanent, refused there and taken here, which is what makes each test
/// about its own card's wording rather than about Manor Gargoyle.
#[test]
fn ancient_grudge_can_target_an_artifact_that_is_also_a_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let gargoyle = named_permanent(&mut state, &reg, "Manor Gargoyle", P1);
    assert!(state.is_creature(gargoyle, &reg), "test precondition");

    let grudge = castable_spell(&mut state, &reg, "Ancient Grudge", P0);
    assert!(offered_targets(&state, &reg, grudge).contains(&Target::Object(gargoyle)),
        "an artifact creature is an artifact");
}

/// "Flashback {G}", cast from the graveyard, and CR 702.33a's "then exile it"
/// — the whole second life of the card, end to end.
#[test]
fn ancient_grudge_can_be_flashed_back_and_is_then_exiled() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let artifact = named_permanent(&mut state, &reg, "Cobbled Wings", P1);

    let card_id = reg.get_id_by_name("Ancient Grudge").unwrap();
    let grudge = state.create_object(card_id, P0, Zone::Graveyard, None, None);
    state.get_object_mut(grudge).unwrap().name = "Ancient Grudge".into();
    // Exactly {G}: the flashback cost, and not the {1}{R} printed cost.
    state.get_player_mut(P0).mana_pool.add(ManaType::Green, 1);

    assert!(can_cast(&state, &reg, grudge),
        "flashback is offered for a card with flashback in your graveyard");
    let state = cast_and_resolve(&state, &reg, grudge, vec![Target::Object(artifact)]);

    assert_eq!(state.get_object(artifact).unwrap().zone, Zone::Graveyard,
        "the flashed-back Grudge still destroys what it targeted");
    assert_eq!(state.get_object(grudge).unwrap().zone, Zone::Exile,
        "and is exiled rather than returning to the graveyard, so it cannot be \
         flashed back a second time");
}

// -------------------------------------------------------------------------
// Bramblecrush
// -------------------------------------------------------------------------

/// "Target **noncreature** permanent" is about what the permanent is, not about
/// which of the other types it has. Manor Gargoyle is an artifact *creature*,
/// so it is not a legal target — an implementation asking "is it an artifact,
/// enchantment or land?" rather than "is it noncreature" would take it.
///
/// The table above uses a plain creature, which that implementation also
/// refuses; only a permanent that is a creature *and* something else separates
/// the two readings.
#[test]
fn bramblecrush_cannot_target_an_artifact_that_is_also_a_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let gargoyle = named_permanent(&mut state, &reg, "Manor Gargoyle", P1);
    assert!(state.is_creature(gargoyle, &reg), "test precondition");
    assert!(state.has_card_type(gargoyle, CardType::Artifact, &reg),
        "test precondition: it is an artifact as well");
    let plain_artifact = named_permanent(&mut state, &reg, "Cobbled Wings", P1);

    let crush = castable_spell(&mut state, &reg, "Bramblecrush", P0);
    let offered = offered_targets(&state, &reg, crush);

    assert!(!offered.contains(&Target::Object(gargoyle)),
        "an artifact creature is a creature; offered {offered:?}");
    assert!(offered.contains(&Target::Object(plain_artifact)),
        "a plain artifact is a legal target, so the refusal above is about \
         creature-ness; offered {offered:?}");
}

/// "Target noncreature permanent" carries no controller restriction, so your
/// own permanents are legal targets too. Every existing case points it at the
/// opponent.
#[test]
fn bramblecrush_may_target_your_own_permanents() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let mine = named_permanent(&mut state, &reg, "Forest", P0);
    let theirs = named_permanent(&mut state, &reg, "Forest", P1);

    let crush = castable_spell(&mut state, &reg, "Bramblecrush", P0);
    let offered = offered_targets(&state, &reg, crush);

    assert!(offered.contains(&Target::Object(mine)), "your own; offered {offered:?}");
    assert!(offered.contains(&Target::Object(theirs)), "and theirs; offered {offered:?}");

    let state = cast_and_resolve(&state, &reg, crush, vec![Target::Object(mine)]);
    assert_eq!(state.get_object(mine).unwrap().zone, Zone::Graveyard,
        "and it does destroy your own when you point it there");
}

/// Bramblecrush should use the destruction pipeline for non-creature permanents.
/// An indestructible enchantment should survive Bramblecrush.
#[test]
fn bramblecrush_respects_indestructible() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Create a non-creature permanent (enchantment) with indestructible.
    let enchantment = state.create_object(CardId(9999), P1, Zone::Battlefield, None, None);
    state.get_object_mut(enchantment).unwrap().name = "Indestructible Enchantment".into();
    state.get_object_mut(enchantment).unwrap().card_types = vec![CardType::Enchantment];
    state.until_end_of_turn.push(
        mtg_engine::state::TemporaryEffect::GrantKeyword {
            target: enchantment,
            keyword: Keyword::Indestructible,
        },
    );

    let crush = castable_spell(&mut state, &reg, "Bramblecrush", P0);
    state = cast_and_resolve(&state, &reg, crush, vec![Target::Object(enchantment)]);

    // Indestructible enchantment should survive.
    assert_eq!(state.get_object(enchantment).unwrap().zone, Zone::Battlefield,
        "Bramblecrush should respect indestructible on non-creature permanents");
}
