//! CR 702.33: a card can carry several instances of flashback at once, and
//! the player may pay any of them.
//!
//! The action generator picked a single winner — a granted flashback always
//! beat the printed one — and discarded the rest. That is not merely a missing
//! choice. With Bump in the Night ({B} mana cost, {5}{R} printed flashback) in
//! the graveyard and only red mana available, Past in Flames' granted {B} cost
//! was found unaffordable and the payable {5}{R} was never offered at all.
//!
//! Separately, CR 702.33a defines the granted cost as "equal to its mana
//! cost", so a card with NO mana cost gains no usable flashback. Three places
//! substituted a free cost instead, which made such a card castable for {0}.

mod common;
use common::*;
use mtg_engine::actions::{Action, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::ids::ObjectId;
use mtg_engine::state::{GameState, TemporaryEffect};
use mtg_engine::types::*;

/// Every distinct flashback cost offered for `card` this turn.
fn flashback_costs(state: &GameState, reg: &CardRegistry, card: ObjectId) -> Vec<ManaCost> {
    let mut costs: Vec<ManaCost> = mtg_engine::engine::legal_actions(state, reg).actions.iter()
        .filter_map(|a| match a {
            Action::CastSpell { object_id, alternative_cost: Some(c), .. } if *object_id == card => Some(c.clone()),
            _ => None,
        })
        .collect();
    costs.dedup();
    costs
}

fn card_in_graveyard(state: &mut GameState, reg: &CardRegistry, name: &str) -> ObjectId {
    named_card_in_graveyard(state, reg, name, P0)
}

/// A card with printed flashback that is also granted flashback must offer
/// both costs.
#[test]
fn both_granted_and_printed_flashback_costs_are_offered() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Devil's Play has a printed flashback cost distinct from its mana cost.
    let card = card_in_graveyard(&mut state, &reg, "Geistflame");
    let printed = reg.card_data(state.get_object(card).unwrap().card_id).unwrap()
        .flashback_cost.expect("Geistflame has printed flashback");
    let mana_cost = reg.card_data(state.get_object(card).unwrap().card_id).unwrap()
        .cost.expect("Geistflame has a mana cost");
    assert_ne!(printed, mana_cost, "test precondition: the two costs differ");

    // Grant flashback equal to the mana cost, the way Snapcaster does.
    state.until_end_of_turn.push(TemporaryEffect::GrantFlashback {
        target: card, cost: mana_cost.clone(),
    });
    // Plenty of mana so affordability isn't what's being tested.
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 8);
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 8);

    let costs = flashback_costs(&state, &reg, card);
    assert!(costs.contains(&mana_cost),
        "the granted flashback cost must be offered; got {costs:?}");
    assert!(costs.contains(&printed),
        "the printed flashback cost must ALSO be offered — CR 702.33 lets the \
         player choose any instance; got {costs:?}");
}

/// The affordable option must survive even when the other one isn't payable.
#[test]
fn an_unaffordable_granted_cost_does_not_hide_the_payable_printed_one() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let card = card_in_graveyard(&mut state, &reg, "Geistflame");
    let data = reg.card_data(state.get_object(card).unwrap().card_id).unwrap();
    let printed = data.flashback_cost.clone().unwrap();

    // Grant a cost the player cannot pay: five blue.
    state.until_end_of_turn.push(TemporaryEffect::GrantFlashback {
        target: card,
        cost: ManaCost::new(vec![ManaSymbol::Colored(Color::Blue); 5]),
    });
    // Enough for the printed {3}{R} flashback, but no blue at all.
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 4);
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 4);

    let costs = flashback_costs(&state, &reg, card);
    assert!(costs.contains(&printed),
        "the granted cost is unaffordable, so the payable printed flashback \
         must still be offered; got {costs:?}");
}

/// CR 702.33a: no mana cost means no flashback cost — not a free one.
#[test]
fn a_card_with_no_mana_cost_gains_no_flashback() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Snapcaster's grant reads the target's mana cost.
    let snapcaster = reg.get_id_by_name("Snapcaster Mage").unwrap();
    let behavior = reg.get(snapcaster).unwrap();

    // A graveyard card with no mana cost at all.
    let costless = state.create_object(mtg_engine::ids::CardId(0), P0, Zone::Graveyard, None, None);

    let snap_obj = named_creature(&mut state, &reg, "Snapcaster Mage", P0);
    let before = state.until_end_of_turn.len();
    behavior.on_enter_battlefield(&mut state, snap_obj,
        &[mtg_engine::actions::Target::Object(costless)], &reg);

    assert_eq!(state.until_end_of_turn.len(), before,
        "a card with no mana cost has no flashback cost, so no grant should be \
         made — substituting a free cost made it castable for {{0}}");
}

/// A card that already has a flashback grant is still a legal Snapcaster
/// target — CR 702.33 allows several instances at once, so refusing meant a
/// second Snapcaster's trigger was removed under CR 603.3c.
#[test]
fn snapcaster_can_target_a_card_that_already_has_flashback() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let card = card_in_graveyard(&mut state, &reg, "Geistflame");
    state.until_end_of_turn.push(TemporaryEffect::GrantFlashback {
        target: card,
        cost: ManaCost::new(vec![ManaSymbol::Colored(Color::Red)]),
    });

    let snapcaster = reg.get_id_by_name("Snapcaster Mage").unwrap();
    let behavior = reg.get(snapcaster).unwrap();
    assert!(behavior.is_valid_target(&state, P0, &mtg_engine::actions::Target::Object(card), &reg),
        "a card that already has flashback is still a legal target");
}

// -------------------------------------------------------------------------
// From the bug-audit files, re-filed by the rule each one exercises.
// -------------------------------------------------------------------------

/// Bug: Snapcaster Mage grants flashback to an instant or sorcery in
/// the graveyard, but incorrectly excludes cards that already have
/// innate flashback. The oracle says "target instant or sorcery card"
/// with no restriction on existing flashback.
#[test]
fn bug_snapcaster_excludes_innate_flashback_cards() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put Think Twice (has innate flashback) in P0's graveyard
    let think_twice = {
        let card_id = registry.get_id_by_name("Think Twice").unwrap();
        let id = state.create_object(card_id, P0, Zone::Graveyard, None, None);
        state.get_object_mut(id).unwrap().name = "Think Twice".into();
        id
    };

    // Cast Snapcaster Mage — should be able to target Think Twice
    let _snap = castable_spell(&mut state, &registry, "Snapcaster Mage", P0);

    // Check if Think Twice is a valid target
    let behavior = registry.get(
        registry.get_id_by_name("Snapcaster Mage").unwrap()
    ).unwrap();
    let is_valid = behavior.is_valid_target(
        &state, P0, &Target::Object(think_twice), &registry
    );

    // BUG: Think Twice excluded because it has innate flashback
    assert!(is_valid,
        "Snapcaster Mage should be able to target cards with innate flashback");
}
