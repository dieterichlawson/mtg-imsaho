//! One tap, one cost.
//!
//! A `{T}` symbol in a cost has three requirements that are identical for
//! every permanent in the game — on the battlefield, untapped, and (for a
//! creature) past summoning sickness unless it has haste, CR 302.6 — and a
//! fourth that follows from CR 602.2h: the tap that pays a `{T}` cost cannot
//! also be the tap that produces mana for the same activation.
//!
//! The first three used to be re-derived by hand in each card's
//! `mana_abilities` / `activated_abilities`. Two of twenty-odd cards spelled
//! out the summoning-sickness half and *both* of those forgot haste; the
//! other nineteen forgot summoning sickness altogether, so a Llanowar-style
//! mana creature could be tapped for mana the turn it arrived. They now all
//! defer to `GameState::can_pay_tap_cost`, applied centrally by
//! `engine::available_mana_abilities` and by `legal_actions`.

mod common;

use common::*;
use mtg_engine::actions::Action;
use mtg_engine::cards::CardRegistry;
use mtg_engine::types::*;
/// Put a named permanent onto the battlefield *this turn*, so it still has
/// summoning sickness. `named_permanent` deliberately clears the flag.
fn sick_named(
    state: &mut mtg_engine::state::GameState,
    reg: &CardRegistry,
    name: &str,
    owner: PlayerId,
) -> ObjectId {
    let id = named_permanent(state, reg, name, owner);
    state.get_object_mut(id).unwrap().summoning_sick = true;
    id
}

/// Grant haste the way an effect does — `obj.keywords` is the *printed* set
/// for objects with no registry face, not a place effects write to.
fn grant_haste(state: &mut mtg_engine::state::GameState, id: ObjectId) {
    state.until_end_of_turn.push(mtg_engine::state::TemporaryEffect::GrantKeyword {
        target: id,
        keyword: Keyword::Haste,
    });
}

fn mana_ability_actions(
    state: &mtg_engine::state::GameState,
    reg: &CardRegistry,
    object_id: ObjectId,
) -> usize {
    mtg_engine::engine::legal_actions(state, reg)
        .actions
        .iter()
        .filter(|a| matches!(a, Action::ActivateManaAbility { object_id: o, .. } if *o == object_id))
        .count()
}

// ---------------------------------------------------------------------------
// CR 302.6: summoning sickness gates {T} costs on creatures.
// ---------------------------------------------------------------------------

/// Avacyn's Pilgrim is "{T}: Add {W}". A creature that arrived this turn
/// cannot pay a {T} cost, so the ability must not be offered — and, more
/// importantly, must not silently fund a spell through the auto-tap planner.
#[test]
fn a_mana_creature_cannot_be_tapped_for_mana_the_turn_it_arrives() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let pilgrim = sick_named(&mut state, &reg, "Avacyn's Pilgrim", P0);

    assert_eq!(mana_ability_actions(&state, &reg, pilgrim), 0,
        "a summoning-sick creature cannot pay {{T}} (CR 302.6)");
    assert!(mtg_engine::engine::available_mana_abilities(&state, pilgrim, &reg).is_empty(),
        "the auto-tap planner must not see a summoning-sick creature as a mana source");
}

/// The same Pilgrim, no longer summoning sick, is a mana source.
#[test]
fn a_settled_mana_creature_is_a_mana_source() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let pilgrim = named_permanent(&mut state, &reg, "Avacyn's Pilgrim", P0);

    assert_eq!(mana_ability_actions(&state, &reg, pilgrim), 1);
    assert_eq!(mtg_engine::engine::available_mana_abilities(&state, pilgrim, &reg).len(), 1);
}

/// Haste is the exception (CR 302.6). Every hand-rolled summoning-sickness
/// check in card code had dropped it.
#[test]
fn haste_lets_a_mana_creature_tap_the_turn_it_arrives() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let pilgrim = sick_named(&mut state, &reg, "Avacyn's Pilgrim", P0);
    grant_haste(&mut state, pilgrim);

    assert!(state.can_pay_tap_cost(pilgrim, &reg),
        "a hasty creature can pay {{T}} the turn it arrives");
    assert_eq!(mana_ability_actions(&state, &reg, pilgrim), 1,
        "haste must re-enable the {{T}} mana ability (CR 302.6)");
}

/// Deranged Assistant's "{T}: Mill a card, add {C}" has a condition of its
/// own on top of the tap cost — there has to be a card left to mill. That
/// part stays in the card; the tap part does not.
#[test]
fn a_mana_ability_keeps_its_own_conditions() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let assistant = named_permanent(&mut state, &reg, "Deranged Assistant", P0);

    state.get_player_mut(P0).library_order.clear();
    assert!(mtg_engine::engine::available_mana_abilities(&state, assistant, &reg).is_empty(),
        "no card left to mill means the ability can't be activated");

    let card = state.create_object(
        reg.get_id_by_name("Walking Corpse").unwrap(), P0, Zone::Library, None, None);
    state.get_player_mut(P0).library_order.push(card);
    assert_eq!(mtg_engine::engine::available_mana_abilities(&state, assistant, &reg).len(), 1,
        "with a card to mill the ability is available again");
}

/// Summoning sickness is set on non-creature permanents too, but never
/// restricts them (CR 302.6 is about creatures). A land that entered this
/// turn taps for mana normally.
#[test]
fn a_land_that_entered_this_turn_still_taps_for_mana() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let forest = sick_named(&mut state, &reg, "Forest", P0);

    assert!(state.can_pay_tap_cost(forest, &reg),
        "summoning sickness does not apply to a land (CR 302.6)");
    assert_eq!(mana_ability_actions(&state, &reg, forest), 1);
}

/// A tapped permanent can't pay {T} again, and neither can one that has left
/// the battlefield.
#[test]
fn tapped_and_off_battlefield_permanents_cannot_pay_a_tap_cost() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let tapped = named_permanent(&mut state, &reg, "Forest", P0);
    state.get_object_mut(tapped).unwrap().tapped = true;
    assert!(!state.can_pay_tap_cost(tapped, &reg));
    assert!(mtg_engine::engine::available_mana_abilities(&state, tapped, &reg).is_empty());

    let gone = named_permanent(&mut state, &reg, "Forest", P0);
    state.move_object(gone, Zone::Graveyard, &reg);
    assert!(!state.can_pay_tap_cost(gone, &reg));
    assert!(mtg_engine::engine::available_mana_abilities(&state, gone, &reg).is_empty(),
        "a land in the graveyard is not a mana source");
}

// ---------------------------------------------------------------------------
// Skirsdag High Priest: the same rule, on an activated ability.
// ---------------------------------------------------------------------------

/// Skirsdag High Priest is "Morbid — {T}, Tap two untapped creatures you
/// control: ...". Its card-level guard checked `summoning_sick` directly and
/// returned an empty ability list, so the engine's correct haste-aware check
/// never got a chance to run.
#[test]
fn skirsdag_high_priest_with_haste_can_activate_while_summoning_sick() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let priest = sick_named(&mut state, &reg, "Skirsdag High Priest", P0);
    named_permanent(&mut state, &reg, "Walking Corpse", P0);
    named_permanent(&mut state, &reg, "Walking Corpse", P0);
    state.creature_died_this_turn = true;

    assert!(!offers_ability_of(&state, &reg, priest),
        "without haste a summoning-sick Priest cannot pay {{T}}");

    grant_haste(&mut state, priest);
    assert!(offers_ability_of(&state, &reg, priest),
        "with haste the Priest can pay {{T}} the turn it arrives (CR 302.6)");
}

// ---------------------------------------------------------------------------
// CR 602.2h: one tap pays one cost.
// ---------------------------------------------------------------------------

/// Gavony Township is "{2}{G}{W}, {T}: ..." on top of "{T}: Add {C}". Paying
/// the {T} in the activation cost means the Township is tapped, so it cannot
/// also produce {C} toward the {2}. With three other lands the ability needs
/// a fourth and must not be offered.
#[test]
fn a_tap_ability_cannot_fund_itself_from_its_own_mana_ability() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let township = named_permanent(&mut state, &reg, "Gavony Township", P0);
    named_permanent(&mut state, &reg, "Forest", P0);
    named_permanent(&mut state, &reg, "Forest", P0);
    named_permanent(&mut state, &reg, "Plains", P0);

    assert!(!offers_ability_of(&state, &reg, township),
        "{{2}}{{G}}{{W}},{{T}} needs four other mana sources — the Township's own \
         {{T}}: Add {{C}} is unavailable because that tap pays the ability's {{T}} (CR 602.2h)");

    // A fourth land makes the cost genuinely payable.
    named_permanent(&mut state, &reg, "Plains", P0);
    assert!(offers_ability_of(&state, &reg, township),
        "with four other lands the ability is payable");
}

/// The same rule for the other four ISD utility lands, which share the
/// "{cost}, {T}:" over "{T}: Add {C}" shape.
///
/// Each row is run twice — one land short, then with that land added — because
/// "the ability is not offered" is also what you get from the wrong *colours*
/// of mana. Only the second half shows that the count was the limit and that
/// the land's own {T} was the mana that did not close the gap.
#[test]
fn the_isd_utility_lands_do_not_fund_their_own_tap_abilities() {
    let reg = registry();
    // (land, the other lands that exactly pay its activation cost)
    let lands: &[(&str, &[&str])] = &[
        ("Kessig Wolf Run", &["Mountain", "Forest"]),                 // {X}{R}{G}, {T} at X=0
        ("Nephalia Drownyard", &["Island", "Swamp", "Forest"]),       // {1}{U}{B}, {T}
        ("Gavony Township", &["Forest", "Plains", "Island", "Swamp"]), // {2}{G}{W}, {T}
        ("Stensia Bloodhall", &["Swamp", "Mountain", "Forest", "Island", "Plains"]), // {3}{B}{R}, {T}
    ];

    for &(name, exact) in lands {
        // One short: every land but the last of the set that would pay for it.
        let mut state = game_at_step(Step::PrecombatMain, P0);
        let land = named_permanent(&mut state, &reg, name, P0);
        // Kessig Wolf Run targets a creature and Stensia Bloodhall a player, so
        // give every row a creature — without one, "not offered" would be about
        // the target rather than about the mana.
        ready_creature(&mut state, P0, 2, 2);
        for basic in &exact[..exact.len() - 1] {
            named_permanent(&mut state, &reg, basic, P0);
        }
        assert!(!offers_ability_of(&state, &reg, land),
            "{name}: one mana short, and its own {{T}}: Add {{C}} must not close \
             the gap — that tap is already paying the ability's {{T}} (CR 602.2h)");

        // The missing land, and only that, makes it payable.
        named_permanent(&mut state, &reg, exact[exact.len() - 1], P0);
        assert!(offers_ability_of(&state, &reg, land),
            "{name}: with all of {exact:?} untapped the cost is payable, so the \
             assertion above was about the count and not about colours");
    }
}
