//! Two independent gaps this file pins.
//!
//! A spell whose second target slot is "up to N" produced no cast action at
//! all — `valid_targets_for_req` had no `UpToTargets` branch, so the Cartesian
//! product with the first slot was always empty and Memory's Journey could
//! never be cast. And "from THEIR graveyard" was not enforced at announcement:
//! every graveyard was offered, so a player could declare targets from a third
//! party's and have them silently discarded at resolution (CR 601.2c).
//!
//! Separately, `CreatureCardMilled` is what Undead Alchemist watches, and only
//! `mill_cards` emitted it — so cards that moved library cards to the graveyard
//! by hand were invisible to it.

mod common;

use common::*;
use mtg_engine::actions::{Action, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::events::GameEvent;
use mtg_engine::ids::ObjectId;
use mtg_engine::state::GameState;
use mtg_engine::types::*;
/// Put a card into a player's library and return its id.
fn card_in_library(state: &mut GameState, reg: &CardRegistry, name: &str, owner: mtg_engine::ids::PlayerId) -> ObjectId {
    let card_id = reg.get_id_by_name(name).unwrap_or_else(|| panic!("unknown {name}"));
    let data = reg.card_data(card_id).unwrap();
    let id = state.create_object(card_id, owner, Zone::Library, data.power, data.toughness);
    state.get_object_mut(id).unwrap().name = name.into();
    state.get_player_mut(owner).library_order.push(id);
    id
}

/// Undead Alchemist's tokens are nameless — a 2/2 black Zombie — so they are
/// counted by what they are rather than by `name`.
fn zombie_tokens(state: &GameState, reg: &CardRegistry) -> usize {
    state.objects.values()
        .filter(|o| o.is_token && o.zone == Zone::Battlefield
            && state.has_subtype(o.id, "Zombie", reg))
        .count()
}

fn cast_actions_for(state: &GameState, reg: &CardRegistry, spell: ObjectId) -> Vec<Vec<Target>> {
    mtg_engine::engine::legal_actions(state, reg).actions.iter()
        .filter_map(|a| match a {
            Action::CastSpell { object_id, targets, .. } if *object_id == spell => Some(targets.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn memorys_journey_is_castable() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    named_card_in_graveyard(&mut state, &reg, "Walking Corpse", P0);
    let spell = castable_spell(&mut state, &reg, "Memory's Journey", P0);

    let actions = cast_actions_for(&state, &reg, spell);
    assert!(!actions.is_empty(),
        "Memory's Journey has an 'up to three' second target slot; with no \
         UpToTargets branch the Cartesian product was empty and the card could \
         never be cast at all");
}

/// "Up to three" includes zero — the player alone is a legal cast.
///
/// The announcement names only the player: the card slot is asked for
/// separately, so this checks the whole way through — the prompt appears, an
/// empty answer is accepted, and the spell reaches the stack with one target.
#[test]
fn memorys_journey_can_be_cast_with_no_card_targets() {
    use mtg_engine::actions::ResolvedChoice;
    use mtg_engine::state::{AwaitingAction, ResolutionChoiceKind};

    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    named_card_in_graveyard(&mut state, &reg, "Walking Corpse", P0);
    let spell = castable_spell(&mut state, &reg, "Memory's Journey", P0);

    let actions = cast_actions_for(&state, &reg, spell);
    assert!(actions.iter().any(|t| t.len() == 1 && matches!(t[0], Target::Player(_))),
        "the player slot is announced on its own; got {actions:?}");

    let asked = cast_onto_stack(&state, &reg, spell, vec![Target::Player(P0)]);
    assert!(matches!(&asked.awaiting_action,
        Some(AwaitingAction::ResolutionChoice {
            choice: ResolutionChoiceKind::ChooseTargetSet { min: 0, .. }, .. })),
        "the card slot is asked for, and none is an answer: {:?}", asked.awaiting_action);

    let none = Action::ResolveChoice { choice: ResolvedChoice::ChosenTargetSet(vec![]) };
    let cast = mtg_engine::engine::submit_action(&asked, &none, &reg);
    let on_stack = cast.get_object(spell).expect("the spell exists");
    assert_eq!(on_stack.zone, Zone::Stack, "zero cards is still a cast");
    assert_eq!(on_stack.targets, vec![Target::Player(P0)],
        "the player and nothing else: {:?}", on_stack.targets);
}

/// "from THEIR graveyard": the cards offered for the second slot are the
/// named player's, never a third party's.
///
/// The slot is one prompt raised after the player is named, so this is a
/// property of the prompt's options — the place where the constraint has to
/// hold now that there is no enumeration to inspect.
#[test]
fn memorys_journey_only_offers_the_targeted_players_graveyard() {
    use mtg_engine::state::{AwaitingAction, ResolutionChoiceKind};

    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let mine = named_card_in_graveyard(&mut state, &reg, "Walking Corpse", P0);
    let theirs = named_card_in_graveyard(&mut state, &reg, "Avacyn's Pilgrim", P1);
    let spell = castable_spell(&mut state, &reg, "Memory's Journey", P0);

    // Both players are announceable, so both branches below are reachable.
    let actions = cast_actions_for(&state, &reg, spell);
    for p in [P0, P1] {
        assert!(actions.iter().any(|t| t.first() == Some(&Target::Player(p))),
            "p{} is a legal first target; got {actions:?}", p.0);
    }

    for (player, ours, theirs) in [(P0, mine, theirs), (P1, theirs, mine)] {
        let asked = cast_onto_stack(&state, &reg, spell, vec![Target::Player(player)]);
        let Some(AwaitingAction::ResolutionChoice {
            choice: ResolutionChoiceKind::ChooseTargetSet { options, .. }, .. })
            = &asked.awaiting_action else {
            panic!("expected a card-slot prompt for p{}, got {:?}",
                player.0, asked.awaiting_action);
        };
        assert!(options.contains(&Target::Object(ours)),
            "p{}'s own graveyard card is offered: {options:?}", player.0);
        assert!(!options.contains(&Target::Object(theirs)),
            "targeting p{} must not offer a card from the other player's \
             graveyard; got {options:?}", player.0);
    }
}

// ── CreatureCardMilled from bespoke mill paths ───────────────────

fn milled_creature_events(state: &GameState) -> usize {
    state.events.iter()
        .filter(|e| matches!(e, GameEvent::CreatureCardMilled { .. }))
        .count()
}

/// Mulch puts the non-lands it reveals into the graveyard from the library —
/// that is a mill, and a creature among them must be visible to watchers.
#[test]
fn mulch_emits_creature_card_milled() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    for _ in 0..4 {
        card_in_library(&mut state, &reg, "Walking Corpse", P0);
    }
    let spell = castable_spell(&mut state, &reg, "Mulch", P0);

    state.events.clear();
    let state = cast_and_resolve(&state, &reg, spell, vec![]);

    assert!(milled_creature_events(&state) > 0,
        "Mulch put creature cards into the graveyard from the library, so \
         Undead Alchemist's watcher must see it");
}

/// Cellar Door mills from the BOTTOM, so it cannot use `mill_cards` — but it
/// is still a mill, and the event has to say so. That it takes the bottom card
/// rather than the top is `cards_complex_creatures.rs`'s business; one card in
/// the library is enough here, where the question is only whether the mill was
/// announced.
#[test]
fn cellar_door_emits_creature_card_milled() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let door = named_permanent(&mut state, &reg, "Cellar Door", P0);
    card_in_library(&mut state, &reg, "Walking Corpse", P1);
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 3);

    state.events.clear();
    activate_via_hooks(&mut state, &reg, door, 0, &[Target::Player(P1)]);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert!(milled_creature_events(&state) > 0,
        "Cellar Door milled a creature card from the bottom of a library; \
         milling from the bottom is still milling");
}

/// "…then mill three cards." The cards leave *your* library for *your*
/// graveyard, and an opponent's Undead Alchemist is exactly the watcher that
/// cares — whether a watcher cares is the collector's decision (it skips
/// watchers controlled by the milled player), not the miller's.
#[test]
fn heretics_punishment_emits_creature_card_milled() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let punishment = named_permanent(&mut state, &reg, "Heretic's Punishment", P0);
    for _ in 0..3 {
        card_in_library(&mut state, &reg, "Walking Corpse", P0);
    }
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 4);

    state.events.clear();
    activate_via_hooks(&mut state, &reg, punishment, 0, &[Target::Player(P1)]);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert_eq!(milled_creature_events(&state), 3,
        "three creature cards went from library to graveyard, so three \
         CreatureCardMilled events");
}

/// "Look at the top four cards of your library. Put one of them into your hand
/// and the rest into your graveyard." The rest are a library-to-graveyard move
/// too, and the shared `ChooseFromRevealed` handler moved them by hand.
#[test]
fn forbidden_alchemy_emits_creature_card_milled_for_the_rest() {
    use mtg_engine::actions::ResolvedChoice;

    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let library: Vec<ObjectId> = (0..4)
        .map(|_| card_in_library(&mut state, &reg, "Walking Corpse", P0))
        .collect();
    let spell = castable_spell(&mut state, &reg, "Forbidden Alchemy", P0);

    let state = cast_and_resolve(&state, &reg, spell, vec![]);
    let mut state = state;
    state.events.clear();
    let state = mtg_engine::engine::submit_action(&state, &Action::ResolveChoice {
        choice: ResolvedChoice::ChosenCard(library[0]),
    }, &reg);

    assert_eq!(state.get_object(library[0]).unwrap().zone, Zone::Hand,
        "test premise: the chosen card went to hand");
    assert_eq!(milled_creature_events(&state), 3,
        "the other three creature cards were put into the graveyard from the \
         library, so an opponent's Undead Alchemist must see them");
}

/// "Whenever equipped creature attacks, defending player reveals cards from the
/// top of their library until they reveal a land card. ... That player puts the
/// revealed cards into their graveyard."
///
/// The one mill in the set that hits an *opponent's* library by default, which
/// is exactly whose graveyard Undead Alchemist watches. It moved the cards by
/// hand and the Alchemist saw nothing.
#[test]
fn trepanation_blade_emits_creature_card_milled() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let blade = named_permanent(&mut state, &reg, "Trepanation Blade", P0);
    let attacker = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(blade).unwrap().attached_to = Some(attacker);
    // Two creature cards on top, then a land to stop on.
    card_in_library(&mut state, &reg, "Walking Corpse", P1);
    card_in_library(&mut state, &reg, "Walking Corpse", P1);
    card_in_library(&mut state, &reg, "Forest", P1);

    state.events.clear();
    reg.get(state.get_object(blade).unwrap().card_id).unwrap().on_attacks(
        &mut state, blade,
        mtg_engine::cards::AttackInfo { attacker, defending_player: P1 },
        &[], &reg);

    assert_eq!(milled_creature_events(&state), 2,
        "two creature cards went from the defending player's library to their \
         graveyard, so two CreatureCardMilled events");
}

/// The event belongs to the zone change, not to any one helper: anything that
/// moves a card from a library to a graveyard is a mill (CR 701.13a).
#[test]
fn move_object_emits_creature_card_milled_for_any_library_to_graveyard_move() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let card = card_in_library(&mut state, &reg, "Walking Corpse", P1);

    state.events.clear();
    state.move_object(card, Zone::Graveyard, &reg);

    assert_eq!(milled_creature_events(&state), 1,
        "a bare move_object from library to graveyard is still a mill — four \
         cards did exactly this by hand and lost the event");
}

/// CR 614.1c/302.6: a library search that puts the card onto the
/// battlefield lands it there ready to use, and tapped when the card says
/// so — and a search that puts it into hand touches neither.
///
/// `finish_library_search` decides on the destination alone, and nothing
/// pinned the split: a search to hand that ran the battlefield half would
/// have untapped-and-unsickened a card sitting in a hand.
#[test]
fn a_library_search_only_touches_the_card_it_puts_onto_the_battlefield() {
    let reg = registry();

    let searched = |destination: Zone, tapped: bool| {
        let mut state = game_at_step(Step::PrecombatMain, P0);
        let source = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
        let found = stock_library(&mut state, &reg, P0, 1)[0];
        state.get_object_mut(found).unwrap().name = "Forest".into();
        state.get_object_mut(found).unwrap().summoning_sick = true;
        mtg_engine::cards::helpers::finish_library_search(
            &mut state, P0, found, destination, tapped, &reg);
        let _ = source;
        (state, found)
    };

    // Onto the battlefield: ready to use, and untapped unless asked.
    let (state, found) = searched(Zone::Battlefield, false);
    let o = state.get_object(found).unwrap();
    assert_eq!(o.zone, Zone::Battlefield);
    assert!(!o.summoning_sick, "a land fetched onto the battlefield is usable");
    assert!(!o.tapped, "and arrives untapped when the card does not say otherwise");

    // "...tapped" arrives tapped (CR 614.1c).
    let (state, found) = searched(Zone::Battlefield, true);
    assert!(state.get_object(found).unwrap().tapped,
        "a search that says 'tapped' puts it onto the battlefield tapped");

    // Into hand: the battlefield half never runs.
    let (state, found) = searched(Zone::Hand, true);
    let o = state.get_object(found).unwrap();
    assert_eq!(o.zone, Zone::Hand);
    assert!(!o.tapped, "a card in hand is not tapped");
    assert!(o.summoning_sick, "and nothing about the battlefield was done to it");

    // Either way the library is shuffled (CR 701.20).
    assert!(state.events.iter().any(|e|
        matches!(e, GameEvent::LibraryShuffled { player } if *player == P0)),
        "a search shuffles the library it read");
}

/// "Whenever a creature card is put into an **opponent's** graveyard from
/// their library" — so an Alchemist watching its own controller's mill does
/// nothing at all.
///
/// Whether a watcher cares is the collector's decision, not the miller's: it
/// skips watchers controlled by the milled player. Both halves are here
/// because the skip is a single comparison, and a test that only shows the
/// trigger firing on an opponent passes just as well when it fires on
/// everyone.
#[test]
fn undead_alchemist_does_not_watch_its_own_controllers_mill() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    named_permanent(&mut state, &reg, "Undead Alchemist", P0);
    let mine = card_in_library(&mut state, &reg, "Walking Corpse", P0);
    let theirs = card_in_library(&mut state, &reg, "Walking Corpse", P1);

    // P0, who controls the Alchemist, mills their own creature card.
    state.events.clear();
    state.trigger_event_index = 0;
    state.move_object(mine, Zone::Graveyard, &reg);
    mtg_engine::triggers::process_triggers(&mut state, &reg);

    assert_eq!(state.get_object(mine).unwrap().zone, Zone::Graveyard,
        "the milled player controls the Alchemist, so the card was not exiled");
    assert_eq!(zombie_tokens(&state, &reg), 0, "and no Zombie was made");

    // The same mill, one library over, is the one it watches.
    state.events.clear();
    state.trigger_event_index = 0;
    state.move_object(theirs, Zone::Graveyard, &reg);
    mtg_engine::triggers::process_triggers(&mut state, &reg);

    assert_eq!(state.get_object(theirs).unwrap().zone, Zone::Exile,
        "an opponent milled, so the Alchemist exiled the card");
    assert_eq!(zombie_tokens(&state, &reg), 1, "and made a Zombie for it");
}
