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
    let mut costs: Vec<ManaCost> = Vec::new();
    for action in &mtg_engine::engine::legal_actions(state, reg).actions {
        if let Action::CastSpell { object_id, alternative_cost: Some(cost), .. } = action {
            // `ManaCost` is not ordered, so this is the dedup rather than
            // `sort` + `dedup` — which would drop only *adjacent* repeats and
            // quietly let "both costs offered" pass on one cost listed twice.
            if *object_id == card && !costs.contains(cost) {
                costs.push(cost.clone());
            }
        }
    }
    costs
}

/// A card with printed flashback that is also granted flashback must offer
/// both costs.
#[test]
fn both_granted_and_printed_flashback_costs_are_offered() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Geistflame has a printed flashback cost distinct from its mana cost.
    let card = named_card_in_graveyard(&mut state, &reg, "Geistflame", P0);
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

    let card = named_card_in_graveyard(&mut state, &reg, "Geistflame", P0);
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

    let snap_obj = named_permanent(&mut state, &reg, "Snapcaster Mage", P0);
    let before = state.until_end_of_turn.len();
    behavior.on_enter_battlefield(&mut state, snap_obj,
        &[mtg_engine::actions::Target::Object(costless)], &reg);

    assert_eq!(state.until_end_of_turn.len(), before,
        "a card with no mana cost has no flashback cost, so no grant should be \
         made — substituting a free cost made it castable for {{0}}");
}

/// CR 702.33 allows several instances of flashback at once, so nothing about
/// a card's *existing* flashback makes it an illegal Snapcaster target —
/// whether that flashback is printed on the card or was granted earlier this
/// turn. Refusing either removed a second Snapcaster's trigger under
/// CR 603.3c instead of stacking a second instance.
///
/// The control row is a plain card with no flashback of any kind: without it,
/// an `is_valid_target` that said yes to everything would pass this test.
#[test]
fn a_cards_existing_flashback_never_makes_it_an_illegal_snapcaster_target() {
    let reg = registry();
    let behavior = reg.get(reg.get_id_by_name("Snapcaster Mage").unwrap()).unwrap();

    // (what the card already has, the card, whether a grant is added on top)
    let cases: [(&str, &str, bool); 3] = [
        ("no flashback at all", "Geistflame", false),
        ("flashback printed on the card", "Think Twice", false),
        ("flashback granted earlier this turn", "Geistflame", true),
    ];

    for (what, name, grant) in cases {
        let mut state = game_at_step(Step::PrecombatMain, P0);
        let card = named_card_in_graveyard(&mut state, &reg, name, P0);
        if grant {
            state.until_end_of_turn.push(TemporaryEffect::GrantFlashback {
                target: card,
                cost: ManaCost::new(vec![ManaSymbol::Colored(Color::Red)]),
            });
        }
        assert!(behavior.is_valid_target(&state, P0, &Target::Object(card), &reg),
            "{name} ({what}) is an instant or sorcery card in a graveyard, so it \
             is a legal Snapcaster target (CR 702.33)");
    }

    // And the requirement Snapcaster does have is still enforced: the target
    // has to be an instant or sorcery card in a graveyard.
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let creature = named_card_in_graveyard(&mut state, &reg, "Walking Corpse", P0);
    assert!(!behavior.is_valid_target(&state, P0, &Target::Object(creature), &reg),
        "a creature card in the graveyard is not a legal Snapcaster target");
}
