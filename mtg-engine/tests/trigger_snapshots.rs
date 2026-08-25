//! A trigger is about what was true when it went on the stack.
//!
//! It resolves later, and by then the board has moved: the creature that
//! attacked may be dead, the Equipment may be on something else, and the
//! permanent whose ability it is may itself be in the graveyard. Re-reading
//! the board at resolution answers a different question than the one the
//! trigger asked.
//!
//! Also here: the werewolf transform condition, which twelve cards each
//! carried a private copy of, and every copy had invented the same clause.

mod common;

use common::*;
use mtg_engine::actions::Target;
use mtg_engine::cards::{AttackInfo, CardRegistry};
use mtg_engine::types::*;
// ---------------------------------------------------------------------------
// Trepanation Blade: "Whenever equipped creature attacks, defending player
// reveals cards from the top of their library until they reveal a land card.
// That player puts those cards into their graveyard. Equipped creature gets
// +1/+0 until end of turn for each card put into that player's graveyard this
// way."
// ---------------------------------------------------------------------------

/// Stack a library for the defending player: three non-lands then a land, so
/// a full reveal mills four.
fn stack_library(state: &mut mtg_engine::state::GameState, reg: &CardRegistry, player: mtg_engine::ids::PlayerId) {
    for name in ["Chapel Geist", "Walking Corpse", "Avacyn's Pilgrim", "Forest"] {
        let id = state.create_object(reg.get_id_by_name(name).unwrap(), player, Zone::Library, None, None);
        state.get_player_mut(player).library_order.push(id);
    }
}

/// Killing the equipped creature in response cancelled the mill too — the
/// handler read `attached_to`, found None, and returned before doing anything.
/// Only the buff has nowhere to go.
#[test]
fn mill_occurs_when_equipped_creature_dies_before_trigger_resolves() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let attacker = named_creature(&mut state, &reg, "Walking Corpse", P0);
    let blade = named_equipment(&mut state, &reg, "Trepanation Blade", P0);
    state.get_object_mut(blade).unwrap().attached_to = Some(attacker);
    stack_library(&mut state, &reg, P1);

    // The attack happened; then the creature died in response to the trigger.
    let attack = AttackInfo::new(attacker, P1);
    mtg_engine::destruction::try_destroy(&mut state, attacker, &reg);
    mtg_engine::sba::check_state_based_actions(&mut state, &reg);
    assert_eq!(state.get_object(blade).unwrap().attached_to, None,
        "test precondition: SBA unattached the Blade");

    reg.get(state.get_object(blade).unwrap().card_id).unwrap()
        .on_attacks(&mut state, blade, attack, &[], &reg);

    assert_eq!(state.get_player(P1).library_order.len(), 0,
        "the defending player still mills to the first land — the trigger was \
         already on the stack and resolves whatever happened to the creature");
}

/// Re-equipping between declaration and resolution must not move the buff to
/// a creature that never attacked.
#[test]
fn the_buff_goes_to_the_creature_that_attacked_not_the_current_host() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let attacker = named_creature(&mut state, &reg, "Walking Corpse", P0);
    let bystander = named_creature(&mut state, &reg, "Chapel Geist", P0);
    let blade = named_equipment(&mut state, &reg, "Trepanation Blade", P0);
    state.get_object_mut(blade).unwrap().attached_to = Some(attacker);
    stack_library(&mut state, &reg, P1);

    let attack = AttackInfo::new(attacker, P1);
    let attacker_power = state.effective_power(attacker, &reg).unwrap();
    let bystander_power = state.effective_power(bystander, &reg).unwrap();

    // Equip moves to the bystander before the trigger resolves.
    state.get_object_mut(blade).unwrap().attached_to = Some(bystander);

    reg.get(state.get_object(blade).unwrap().card_id).unwrap()
        .on_attacks(&mut state, blade, attack, &[], &reg);

    assert!(state.effective_power(attacker, &reg).unwrap() > attacker_power,
        "the creature that attacked gets the +1/+0");
    assert_eq!(state.effective_power(bystander, &reg).unwrap(), bystander_power,
        "a creature that never attacked gets nothing, even though the Blade is \
         on it now");
}

/// The defending player is the one that was attacked, not "whoever the
/// opponent is" — which is the same in two-player, so assert the ordinary
/// case works end to end.
#[test]
fn trepanation_blade_mills_the_defending_player() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let attacker = named_creature(&mut state, &reg, "Walking Corpse", P0);
    let blade = named_equipment(&mut state, &reg, "Trepanation Blade", P0);
    state.get_object_mut(blade).unwrap().attached_to = Some(attacker);
    stack_library(&mut state, &reg, P1);
    let before = state.effective_power(attacker, &reg).unwrap();

    reg.get(state.get_object(blade).unwrap().card_id).unwrap()
        .on_attacks(&mut state, blade, AttackInfo::new(attacker, P1), &[], &reg);

    assert_eq!(state.get_player(P1).library_order.len(), 0, "milled to the land");
    assert_eq!(state.effective_power(attacker, &reg).unwrap(), before + 4,
        "+1/+0 for each of the four cards put into the graveyard");
}

// ---------------------------------------------------------------------------
// Death triggers whose effect happens somewhere else.
// ---------------------------------------------------------------------------

/// Selhoff Occultist's "whenever another creature dies, target player mills a
/// card" fires even when the Occultist died in the same event. Requiring it
/// to still be on the battlefield made it a no-op in exactly the board-wipe
/// case the trigger exists for.
#[test]
fn selhoff_occultist_mills_even_when_it_died_in_the_same_wipe() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let occultist = named_creature(&mut state, &reg, "Selhoff Occultist", P0);
    let other = named_creature(&mut state, &reg, "Walking Corpse", P0);
    for name in ["Chapel Geist", "Forest"] {
        let id = state.create_object(reg.get_id_by_name(name).unwrap(), P1, Zone::Library, None, None);
        state.get_player_mut(P1).library_order.push(id);
    }

    mtg_engine::destruction::try_destroy_all(&mut state, &[occultist, other], &reg);
    assert_eq!(state.get_object(occultist).unwrap().zone, Zone::Graveyard,
        "test precondition: the Occultist died too");

    reg.get(state.get_object(occultist).unwrap().card_id).unwrap()
        .on_any_creature_dies(&mut state, occultist, other, P0, &[], 2, false,
            &[Target::Player(P1)], &reg);

    assert_eq!(state.get_player(P1).library_order.len(), 1,
        "the mill happens — the trigger was created while the Occultist was on \
         the battlefield and resolves regardless of where it is now");
}

/// Rage Thrower's damage likewise. CR 608.2h: a source that has left the
/// battlefield still deals its damage, from last known information.
#[test]
fn rage_thrower_deals_damage_even_when_it_died_in_the_same_wipe() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let thrower = named_creature(&mut state, &reg, "Rage Thrower", P0);
    let other = named_creature(&mut state, &reg, "Walking Corpse", P0);
    let life_before = state.get_player(P1).life;

    mtg_engine::destruction::try_destroy_all(&mut state, &[thrower, other], &reg);

    reg.get(state.get_object(thrower).unwrap().card_id).unwrap()
        .on_any_creature_dies(&mut state, thrower, other, P0, &[], 2, false,
            &[Target::Player(P1)], &reg);

    assert_eq!(state.get_player(P1).life, life_before - 2,
        "2 damage is dealt even though the Thrower died in the same event");
}

/// The other side of the rule: a trigger that puts a counter on ITS OWN
/// permanent does nothing when that permanent is gone (CR 121.1). Lumberknot
/// is right to check.
#[test]
fn a_trigger_that_counters_itself_does_nothing_once_it_has_left() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let knot = named_creature(&mut state, &reg, "Lumberknot", P0);
    let other = named_creature(&mut state, &reg, "Walking Corpse", P0);

    mtg_engine::destruction::try_destroy_all(&mut state, &[knot, other], &reg);

    reg.get(state.get_object(knot).unwrap().card_id).unwrap()
        .on_any_creature_dies(&mut state, knot, other, P0, &[], 2, false, &[], &reg);

    assert_eq!(counters_of(&state, knot, CounterType::PlusOnePlusOne), 0,
        "a permanent in the graveyard cannot receive counters");
}

// ---------------------------------------------------------------------------
// The werewolf transform condition, once.
// ---------------------------------------------------------------------------

/// "If no spells were cast last turn" is satisfied when there was no last
/// turn. Twelve cards each had a private copy of this condition and every one
/// of them had added `&& !state.is_first_turn`, which is nowhere in the
/// oracle text.
#[test]
fn every_werewolf_uses_the_same_transform_condition() {
    let reg = registry();
    let werewolves = [
        "Reckless Waif", "Gatstaf Shepherd", "Village Ironsmith",
        "Villagers of Estwald", "Hanweir Watchkeep", "Grizzled Outcasts",
        "Tormented Pariah", "Mayor of Avabruck", "Ulvenwald Mystics",
        "Kruin Outlaw", "Daybreak Ranger", "Instigator Gang",
    ];

    for name in werewolves {
        // No spells last turn, and it is the game's first turn.
        let mut state = game_at_step(Step::Upkeep, P0);
        state.is_first_turn = true;
        let id = named_creature(&mut state, &reg, name, P0);
        assert!(reg.get(state.get_object(id).unwrap().card_id).unwrap()
            .should_transform(&state, id, &reg),
            "{name}: no spells were cast last turn, so it transforms — being the \
             game's first turn is not a condition the card has");

        // One spell last turn: stays on the front face.
        let mut state = game_at_step(Step::Upkeep, P0);
        state.num_spells_cast_last_turn.insert(P0, 1);
        let id = named_creature(&mut state, &reg, name, P0);
        assert!(!reg.get(state.get_object(id).unwrap().card_id).unwrap()
            .should_transform(&state, id, &reg),
            "{name}: a spell was cast last turn");

        // Transformed, two spells last turn: turns back.
        let mut state = game_at_step(Step::Upkeep, P0);
        state.num_spells_cast_last_turn.insert(P0, 2);
        let id = named_creature(&mut state, &reg, name, P0);
        state.get_object_mut(id).unwrap().is_transformed = true;
        assert!(reg.get(state.get_object(id).unwrap().card_id).unwrap()
            .should_transform(&state, id, &reg),
            "{name}: a player cast two or more spells last turn, so it turns back");
    }
}

// ---------------------------------------------------------------------------
// Back from the Brink: "Exile a creature card from your graveyard and pay its
// mana cost: Create a token that's a copy of that card."
// ---------------------------------------------------------------------------

/// CR 107.3e: X is 0 in a mana cost paid other than by casting the spell, and
/// there is no announcement. The ability's cost carried the raw {X}, so the
/// engine put the player through the X-funding prompt for a value that can
/// only be zero.
#[test]
fn x_cost_creature_activation_costs_only_non_x_portion() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let brink = named_creature(&mut state, &reg, "Back from the Brink", P0);

    // Mikaeus, the Lunarch is {X}{W} — the set's one creature card with {X}.
    let mikaeus = named_card_in_graveyard(&mut state, &reg, "Mikaeus, the Lunarch", P0);
    assert!(reg.card_data(state.get_object(mikaeus).unwrap().card_id)
        .and_then(|d| d.cost).is_some_and(|c| c.has_x()),
        "test precondition: Mikaeus' printed cost contains {{X}}");

    let abilities = reg.get(state.get_object(brink).unwrap().card_id).unwrap()
        .activated_abilities(&state, brink, &reg);
    let ability = abilities.iter()
        .find(|a| a.ability_index == usize::try_from(mikaeus.0).unwrap())
        .expect("Back from the Brink should offer Mikaeus");
    assert!(!ability.cost.has_x(),
        "X is 0 when a mana cost is paid other than by casting the spell \
         (CR 107.3e), so the ability must not carry {{X}} into the engine's \
         X-funding prompt; cost was {:?}", ability.cost);
    assert_eq!(ability.cost.symbols, vec![ManaSymbol::Colored(Color::White)],
        "only the {{W}} remains");

    // The rule itself, independent of what the set happens to contain.
    let with_x = ManaCost::new(vec![ManaSymbol::X, ManaSymbol::Colored(Color::Green)]);
    assert!(with_x.has_x());
    assert!(!with_x.without_x().has_x());
    assert_eq!(with_x.without_x().symbols, vec![ManaSymbol::Colored(Color::Green)]);
}
