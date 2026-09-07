//! Self-tests for the object-shape invariants (`mtg_engine::invariants`'s
//! `objects`, `permanents` and `effects` families): what a game object may
//! look like in each zone, what may be attached to what, and what an
//! effect record may say.
//!
//! Same contract as the other invariant self-tests — the checker is the
//! fuzzer's only pair of eyes, so every clause needs an object that
//! violates it.

mod common;
use common::*;
use mtg_engine::cards::CardRegistry;
use mtg_engine::ids::{CardId, ObjectId};
use mtg_engine::invariants::{check_core, check_settled};
use mtg_engine::actions::Target;
use mtg_engine::types::*;

fn base() -> (GameState, CardRegistry) {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.turn_number = 3;
    (state, reg)
}

#[track_caller]
fn flags(state: &GameState, reg: &CardRegistry, needle: &str) {
    let v = check_core(state, reg);
    assert!(v.iter().any(|m| m.contains(needle)),
        "expected a violation containing {needle:?}, got: {v:?}");
}

#[track_caller]
fn flags_settled(state: &GameState, reg: &CardRegistry, needle: &str) {
    let v = check_settled(state, reg);
    assert!(v.iter().any(|m| m.contains(needle)),
        "expected a settled violation containing {needle:?}, got: {v:?}");
}

/// CR 102.1: every player id stored on an object names a real player. Five
/// fields carry one, and each is its own way to hand the checker a seat
/// that is not there.
#[test]
fn every_player_id_on_an_object_is_range_checked() {
    let (mut state, reg) = base();
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let ghost = PlayerId(u8::try_from(state.players.len()).unwrap());

    let cases: [(&str, fn(&mut mtg_engine::state::GameObject, PlayerId)); 5] = [
        ("owner", |o, p| o.owner = p),
        ("controller", |o, p| o.controller = p),
        ("last_controller", |o, p| o.last_controller = Some(p)),
        ("attached_to_player", |o, p| o.attached_to_player = Some(p)),
        ("last_attached_to_player", |o, p| o.last_attached_to_player = Some(p)),
    ];

    for (what, set) in cases {
        let mut s = state.clone();
        set(s.get_object_mut(bear).unwrap(), ghost);
        flags(&s, &reg, &format!("{what} p2 is not a player (of 2)"));
    }
}

/// CR 200.1/205.2c/110.4/305.9: what a card is, everywhere. A face the
/// registry knows, at least one type, and a type the zone allows.
#[test]
fn an_objects_types_and_face_are_checked_in_every_zone() {
    let (mut state, reg) = base();
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let card = spell_in_hand(&mut state, &reg, "Moment of Heroism", P0);

    // A token copy of a card the registry does not know.
    let mut s = state.clone();
    {
        let o = s.get_object_mut(bear).unwrap();
        o.is_token = true;
        o.card_id = CardId(424_242);
    }
    flags(&s, &reg, "token copy of unregistered card 424242");

    // Nothing at all for a type.
    let mut s = state.clone();
    // A runtime P/T would make it a creature (CR 205.1b), so it has none.
    let blank = s.create_object(CardId(0), P0, Zone::Battlefield, None, None);
    {
        let o = s.get_object_mut(blank).unwrap();
        o.is_token = true;
        o.card_types = vec![];
        o.name = "Wolf".into();
        o.subtypes = vec!["Wolf".into()];
    }
    flags(&s, &reg, "has no card type");

    // CR 304.4/307.4: an instant or sorcery never stays on the battlefield.
    let mut s = state.clone();
    s.get_object_mut(card).unwrap().zone = Zone::Battlefield;
    flags(&s, &reg, "is an instant/sorcery on the battlefield (CR 304.4/307.4)");

    // CR 110.4: and what is on the battlefield is a permanent type.
    let mut s = state.clone();
    {
        let o = s.get_object_mut(bear).unwrap();
        o.is_token = true;
        o.card_id = CardId(0);
        o.name = "Token".into();
        o.subtypes = vec![];
        o.card_types = vec![CardType::Instant];
        // A runtime P/T would make it a creature (CR 205.1b), which is a
        // permanent type — the clause under test is about having none.
        o.power = None;
        o.toughness = None;
    }
    flags(&s, &reg, "on the battlefield has no permanent type (CR 110.4)");

    // CR 305.9: a land is played, never put on the stack.
    let mut s = state.clone();
    let forest = spell_in_hand(&mut s, &reg, "Forest", P0);
    s.get_object_mut(forest).unwrap().zone = Zone::Stack;
    flags(&s, &reg, "is a land on the stack (CR 305.9)");
}

/// CR 107.3g/702.34a: X and the flashback mark belong to specific cards in
/// specific zones.
#[test]
fn x_and_the_flashback_mark_belong_where_they_are_written() {
    let (mut state, reg) = base();
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let card = spell_in_hand(&mut state, &reg, "Moment of Heroism", P0);

    // CR 107.3g: X on a permanent whose cost has no X.
    let mut s = state.clone();
    s.get_object_mut(bear).unwrap().x_value = Some(2);
    flags(&s, &reg, "carries x_value but its cost has no X");

    // CR 702.34a: only a card can be cast with flashback...
    let mut s = state.clone();
    {
        let o = s.get_object_mut(bear).unwrap();
        o.is_token = true;
        o.card_id = CardId(0);
        o.name = "Bear".into();
        o.subtypes = vec!["Bear".into()];
        o.cast_with_flashback = true;
        o.cast_from_zone = Some(Zone::Graveyard);
        o.zone = Zone::Stack;
    }
    flags(&s, &reg, "is a token cast with flashback");

    // ...and only an instant or a sorcery.
    let mut s = state.clone();
    let creature = spell_in_hand(&mut s, &reg, "Grizzly Bears", P0);
    s.get_object_mut(creature).unwrap().zone = Zone::Stack;
    s.get_object_mut(creature).unwrap().cast_with_flashback = true;
    s.get_object_mut(creature).unwrap().cast_from_zone = Some(Zone::Graveyard);
    flags(&s, &reg, "cast with flashback is neither instant nor sorcery");
    let _ = card;
}

/// CR 400.7: a card off the battlefield is its printed self. Nothing it
/// picked up there comes with it.
#[test]
fn a_card_off_the_battlefield_keeps_nothing_it_picked_up_there() {
    let (mut state, reg) = base();
    let card = spell_in_hand(&mut state, &reg, "Moment of Heroism", P0);

    let cases: [(&str, fn(&mut mtg_engine::state::GameObject)); 5] = [
        ("keeps instance effects/text (CR 400.7)",
            |o| o.instance_oracle_text = Some("gains flying".into())),
        ("is summoning sick", |o| o.summoning_sick = true),
        ("remembers attacking", |o| o.attacked_on_turn = Some(3)),
        ("keeps a damage record", |o| o.dealt_deathtouch_damage = true),
        ("remembers activations this turn (CR 400.7)",
            |o| { o.abilities_activated_this_turn.insert(0); }),
    ];
    for (needle, corrupt) in cases {
        let mut s = state.clone();
        corrupt(s.get_object_mut(card).unwrap());
        flags(&s, &reg, needle);
    }

    // CR 701.19: a regeneration shield is a battlefield thing.
    let mut s = state.clone();
    s.get_object_mut(card).unwrap().regeneration_shields = 1;
    flags(&s, &reg, "keeps a regeneration shield");

    // CR 205.4b: the legendary cache never claims more than the face.
    let mut s = state.clone();
    s.get_object_mut(card).unwrap().is_legendary = true;
    flags(&s, &reg, "is flagged legendary but its face is not");
}

/// CR 120.3/120.3c: marked damage lives on a battlefield creature, with a
/// record of what dealt it, and never on a planeswalker.
#[test]
fn marked_damage_lives_on_a_battlefield_creature() {
    let (mut state, reg) = base();
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let land = named_permanent(&mut state, &reg, "Forest", P0);
    let lili = named_permanent(&mut state, &reg, "Liliana of the Veil", P0);
    set_loyalty(&mut state, lili, 3);

    let mut s = state.clone();
    {
        let o = s.get_object_mut(land).unwrap();
        o.damage_marked = 1;
        o.damaged_by.push(bear);
    }
    flags(&s, &reg, "damage marked but is no battlefield creature (CR 120.3)");

    let mut s = state.clone();
    s.get_object_mut(bear).unwrap().damage_marked = 1;
    flags(&s, &reg, "damage marked but no record of what dealt it");

    let mut s = state.clone();
    s.get_object_mut(bear).unwrap().dealt_deathtouch_damage = true;
    flags(&s, &reg, "was dealt deathtouch damage but has none marked");

    // CR 120.3c: damage to a planeswalker is loyalty, not marked damage.
    let mut s = state.clone();
    {
        let o = s.get_object_mut(lili).unwrap();
        o.damage_marked = 1;
        o.damaged_by.push(bear);
    }
    flags(&s, &reg, "is a planeswalker with damage marked (CR 120.3c)");

    // CR 606.3: the loyalty sentinel is only ever on a planeswalker.
    let mut s = state.clone();
    s.get_object_mut(bear).unwrap().abilities_activated_this_turn.insert(999);
    flags(&s, &reg, "used a loyalty ability but is no planeswalker");
}

/// CR 303.4/701.3a: attachment is a battlefield fact, and only an Aura
/// whose enchant ability names players is ever attached to one.
#[test]
fn attachment_is_a_battlefield_fact_about_the_right_kind_of_thing() {
    let (mut state, reg) = base();
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let card = spell_in_hand(&mut state, &reg, "Pacifism", P0);

    // Attached while not on the battlefield.
    let mut s = state.clone();
    s.get_object_mut(card).unwrap().attached_to = Some(bear);
    flags(&s, &reg, "is attached to something");

    // A player enchanted by something that is no Aura.
    let mut s = state.clone();
    s.get_object_mut(bear).unwrap().attached_to_player = Some(P1);
    flags(&s, &reg, "is attached to a player but is no Aura (CR 303.4)");

    // An Aura that enchants creatures, attached to a player.
    let mut s = state.clone();
    let aura = named_permanent(&mut s, &reg, "Pacifism", P0);
    s.get_object_mut(aura).unwrap().attached_to = None;
    s.get_object_mut(aura).unwrap().attached_to_player = Some(P1);
    flags(&s, &reg, "enchants a player but its enchant ability does not allow one");
    flags_settled(&s, &reg, "enchants creatures but is attached to a player");

    // The shadow of a past attachment is not kept on the battlefield.
    let mut s = state.clone();
    s.get_object_mut(bear).unwrap().last_attached_to_player = Some(P1);
    flags(&s, &reg, "on the battlefield keeps a last-attached-to-player shadow");

    // CR 303.4d/301.5c: an attached Aura or Equipment is not a creature.
    let mut s = state.clone();
    let host = named_permanent(&mut s, &reg, "Grizzly Bears", P0);
    let other = named_permanent(&mut s, &reg, "Grizzly Bears", P0);
    s.get_object_mut(other).unwrap().attached_to = Some(host);
    s.get_object_mut(other).unwrap().subtypes.push("Aura".into());
    s.get_object_mut(other).unwrap().card_types.push(CardType::Enchantment);
    flags_settled(&s, &reg, "is a creature attached to something (CR 303.4d/301.5c)");
}

/// CR 111.4/205.3/707.8: a token is named after its subtypes, a subtype
/// belongs to its card type, and the name cache agrees with the face.
#[test]
fn a_tokens_name_and_types_describe_what_it_is() {
    let (mut state, reg) = base();
    let wolf = state.create_token_with_subtypes("", P0, 2, 2, vec![Color::Green],
        vec![CardType::Creature], vec![], vec!["Wolf".into()], &reg)[0];
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);

    // CR 111.4: the name is the subtype(s), and nothing else. A different
    // word is wrong, and so is the right word with " Token" welded on —
    // that is a name no card can share, so "creatures with the same name"
    // could never match the token (issues #331, #334).
    let mut s = state.clone();
    s.get_object_mut(wolf).unwrap().name = "Bear".into();
    flags(&s, &reg, "is not its subtypes");
    let mut s = state.clone();
    s.get_object_mut(wolf).unwrap().name = "Wolf Token".into();
    flags(&s, &reg, "is not its subtypes");

    // A token with subtypes and no creature type.
    let mut s = state.clone();
    s.get_object_mut(wolf).unwrap().card_types = vec![CardType::Artifact];
    flags(&s, &reg, "is a token with subtypes");

    // CR 205.3: an Aura is an Enchantment, an Equipment an Artifact.
    let mut s = state.clone();
    s.get_object_mut(bear).unwrap().subtypes.push("Equipment".into());
    flags(&s, &reg, "has subtype Equipment without type Artifact (CR 205.3)");

    // CR 707.8: the cached name is the face that is up.
    let mut s = state.clone();
    s.get_object_mut(bear).unwrap().name = "Something Else".into();
    flags(&s, &reg, "name cache says");

    // CR 208.1: power and toughness come as a pair.
    let mut s = state.clone();
    s.get_object_mut(bear).unwrap().toughness = None;
    flags(&s, &reg, "(CR 208.1)");

    // CR 111.8: a token on the battlefield has never changed zones.
    let mut s = state.clone();
    s.get_object_mut(wolf).unwrap().zone_change_count = 1;
    flags(&s, &reg, "is a token that changed zones");

    // Loyalty counters on a non-planeswalker.
    let mut s = state.clone();
    s.add_counters(bear, CounterType::Loyalty, 1);
    flags(&s, &reg, "holds loyalty counters but is no planeswalker");

    // The unused day/night designation stays unused.
    let mut s = state.clone();
    s.day_night = Some(mtg_engine::state::DayNight::Day);
    flags(&s, &reg, "day/night designation set but nothing in this pool uses it");
}

/// CR 700.2: a modal spell on the stack has exactly one of its own modes
/// chosen, and a spell that is not modal has none.
#[test]
fn a_modal_spell_on_the_stack_chose_one_of_its_modes() {
    let (mut state, reg) = base();
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let modal = castable_spell(&mut state, &reg, "Brimstone Volley", P0);
    let state = cast_onto_stack(&state, &reg, modal, vec![Target::Object(bear)]);

    // A spell with no modes carrying a chosen one is checked elsewhere; the
    // gap here is a modal spell whose mode is missing or out of range.
    let modal_card = state.get_object(modal).unwrap().card_id;
    if matches!(reg.get(modal_card).map(|b| b.target_requirement()),
                Some(mtg_engine::cards::TargetRequirement::ModalChoice(_))) {
        let mut s = state.clone();
        s.get_object_mut(modal).unwrap().chosen_mode = None;
        flags(&s, &reg, "is a modal spell on the stack with no mode chosen (CR 700.2)");
        let mut s = state.clone();
        s.get_object_mut(modal).unwrap().chosen_mode = Some(99);
        flags(&s, &reg, "(CR 700.2)");
    }
}

/// CR 614.12b: a permanent still waiting on its enters-as-a-copy choice has
/// not entered the battlefield.
#[test]
fn a_permanent_waiting_on_its_copy_choice_is_not_on_the_battlefield() {
    let (mut state, reg) = base();
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    state.pending_entry_choices.push(bear);
    flags(&state, &reg, "is on the battlefield while still queued for its enters-as-copy choice (CR 614.12b)");
}

/// CR 400.7/702.34a/611.2b: an effect record points at something that can
/// carry it, for as long as the rules let it.
#[test]
fn every_effect_record_points_at_something_that_can_carry_it() {
    let (mut state, reg) = base();
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let land = named_permanent(&mut state, &reg, "Forest", P0);
    let buried = named_card_in_graveyard(&mut state, &reg, "Moment of Heroism", P0);
    let creature_card = named_card_in_graveyard(&mut state, &reg, "Grizzly Bears", P0);
    let cost = state.face_data(buried, &reg).unwrap().cost.unwrap();

    // CR 702.34a: flashback is granted to an instant or a sorcery card.
    let mut s = state.clone();
    s.until_end_of_turn.push(mtg_engine::state::TemporaryEffect::GrantFlashback {
        target: creature_card, cost: cost.clone() });
    flags_settled(&s, &reg, "which is no instant or sorcery");

    let mut s = state.clone();
    s.until_end_of_turn.push(mtg_engine::state::TemporaryEffect::GrantFlashback {
        target: ObjectId(4242), cost: cost.clone() });
    flags_settled(&s, &reg, "flashback granted to");

    let mut s = state.clone();
    let token = s.create_token_with_subtypes("", P0, 2, 2, vec![Color::Green],
        vec![CardType::Creature], vec![], vec!["Wolf".into()], &reg)[0];
    s.until_end_of_turn.push(mtg_engine::state::TemporaryEffect::GrantFlashback {
        target: token, cost });
    flags_settled(&s, &reg, "flashback granted to token #");

    // A control effect over something that is not a creature.
    let mut s = state.clone();
    s.control_effects.push(mtg_engine::state::ControlEffect {
        object: land, controller: P1, original_controller: P0,
        source: bear, source_controller: P0, timestamp: 1 });
    flags_settled(&s, &reg, "control effect over #");
    let _ = buried;
}

/// CR 704.5m/301.5/702.16c/603.8: what an attachment on the battlefield may
/// be attached to once state-based actions have settled.
#[test]
fn an_attachment_on_the_battlefield_sits_where_the_rules_allow() {
    let (mut state, reg) = base();
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let land = named_permanent(&mut state, &reg, "Forest", P0);
    let aura = named_permanent(&mut state, &reg, "Pacifism", P0);
    state.get_object_mut(aura).unwrap().attached_to = Some(bear);
    assert_eq!(check_settled(&state, &reg), Vec::<String>::new());

    // CR 704.5m: an Aura that enchants creatures, on a land. That is an
    // Aura problem, reported by the enchant-ability clause — the Equipment
    // clause is about Equipment and has nothing to say here.
    let mut s = state.clone();
    s.get_object_mut(aura).unwrap().attached_to = Some(land);
    flags_settled(&s, &reg, "enchants creatures but is attached to non-creature #");
    assert!(!check_settled(&s, &reg).iter().any(|m| m.contains("Equipment")),
        "an Aura on a land is not an Equipment on a non-creature: {:?}",
        check_settled(&s, &reg));

    // The Equipment clause does have something to say about an Equipment.
    let mut s = state.clone();
    let blade = named_permanent(&mut s, &reg, "Butcher's Cleaver", P0);
    s.get_object_mut(blade).unwrap().attached_to = Some(land);
    flags_settled(&s, &reg, "attached to non-creature");

    // CR 702.16c: nothing is attached to what has protection from it.
    let mut s = state.clone();
    s.get_object_mut(bear).unwrap().instance_continuous_effects = Some(vec![
        ContinuousEffect::ProtectionFromSubtype {
            subtype: "Aura".into(), scope: EffectScope::OnSelf },
    ]);
    flags_settled(&s, &reg, "which has protection from it (CR 702.16c)");

    // CR 301.5: Equipment equips creatures, never players.
    let mut s = state.clone();
    let blade = named_permanent(&mut s, &reg, "Butcher's Cleaver", P0);
    s.get_object_mut(blade).unwrap().attached_to = None;
    s.get_object_mut(blade).unwrap().attached_to_player = Some(P1);
    flags_settled(&s, &reg, "is Equipment attached to a player (CR 301.5)");
}

/// CR 603.8: at a fixed point no unflagged permanent's state-trigger
/// condition is true — the state-based-action loop would have fired it.
#[test]
fn a_state_trigger_whose_condition_holds_has_fired() {
    let (mut state, reg) = base();
    let garruk = named_permanent(&mut state, &reg, "Garruk Relentless", P0);
    set_loyalty(&mut state, garruk, 3);
    assert!(!check_settled(&state, &reg).iter()
        .any(|m| m.contains("state trigger condition holds")),
        "setup: at three loyalty the condition does not hold");

    // "When Garruk Relentless has two or fewer loyalty counters on him,
    // transform him" — with the trigger neither on the stack nor flagged.
    let mut s = state.clone();
    set_loyalty(&mut s, garruk, 2);
    s.get_object_mut(garruk).unwrap().state_trigger_on_stack = false;
    flags_settled(&s, &reg, "state trigger condition holds but the trigger has not fired (CR 603.8)");
}

/// CR 611.2b/400.7: a control effect names the player who has to keep
/// controlling its source, and dies with the source or the object.
#[test]
fn a_control_effect_outlives_neither_its_source_nor_its_object() {
    let (mut state, reg) = base();
    let olivia = named_permanent(&mut state, &reg, "Olivia Voldaren", P0);
    let vampire = named_permanent(&mut state, &reg, "Markov Patrician", P1);
    state.gain_control_while_source_controlled(vampire, olivia, &reg);
    assert_eq!(check_settled(&state, &reg), Vec::<String>::new());

    // The effect gives the permanent to whoever controls the source.
    let mut s = state.clone();
    s.control_effects[0].source_controller = P1;
    flags_settled(&s, &reg, "'s source from p");
}

/// Every non-token object names a card in the registry — with one exemption,
/// and the exemption is exactly as wide as the thing it exists for.
///
/// A permanent that copied a TOKEN carries the token's characteristics on the
/// object and `CardId(0)` where a printed card would be, remembering the real
/// card in `copy_grantor` for the zone-change revert. Both halves of that
/// shape are the exemption: `CardId(0)` alone is a corrupt object, and a
/// `copy_grantor` alone does not excuse a card id nobody has heard of.
#[test]
fn an_unregistered_card_id_is_excused_only_by_a_copied_token() {
    let (mut state, reg) = base();
    let twin = named_permanent(&mut state, &reg, "Evil Twin", P0);
    let printed = state.get_object(twin).unwrap().card_id;

    // The shape the exemption is for: copied a token, so no printed card.
    let mut s = state.clone();
    {
        let o = s.get_object_mut(twin).unwrap();
        o.card_id = CardId(0);
        o.copy_grantor = Some(printed);
        o.name = "Wolf".into();
    }
    assert_eq!(check_core(&s, &reg), Vec::<String>::new(),
        "a permanent that copied a token has no printed card to name");

    // CardId(0) with nothing remembering a real card is just corrupt.
    let mut s = state.clone();
    s.get_object_mut(twin).unwrap().card_id = CardId(0);
    flags(&s, &reg, "is not in the registry");

    // And a grantor does not excuse an id that is not the copied-token one.
    let mut s = state.clone();
    {
        let o = s.get_object_mut(twin).unwrap();
        o.card_id = CardId(424_242);
        o.copy_grantor = Some(printed);
    }
    flags(&s, &reg, "is not in the registry");
}

/// CR 400.7: a non-token card off the battlefield is its printed self. Four
/// runtime fields say otherwise, and the clause is one disjunction over all
/// four — so each is set on its own, or three of them ride along on the
/// fourth and none of them is really checked.
///
/// The gate in front of them matters too, and in both directions: a TOKEN's
/// object-level characteristics ARE what it is (it has no printed card to
/// fall back on), and a permanent still on the battlefield is allowed
/// everything it picked up there.
#[test]
fn only_a_card_off_the_battlefield_has_to_be_its_printed_self() {
    let (mut state, reg) = base();
    let card = spell_in_hand(&mut state, &reg, "Moment of Heroism", P0);

    let cases: [(&str, fn(&mut mtg_engine::state::GameObject)); 4] = [
        ("subtypes", |o| o.subtypes = vec!["Zombie".into()]),
        ("colors", |o| o.colors = vec![Color::Black]),
        ("card types", |o| o.card_types = vec![CardType::Creature]),
        ("keywords", |o| o.keywords = vec![Keyword::Flying]),
    ];
    for (what, corrupt) in cases {
        let mut s = state.clone();
        corrupt(s.get_object_mut(card).unwrap());
        let v = check_core(&s, &reg);
        assert!(v.iter().any(|m| m.contains("keeps runtime characteristics")),
            "{what} alone is a runtime characteristic a card in hand may not keep, got: {v:?}");
    }

    // A token in a graveyard is its object-level fields — there is no printed
    // card underneath to disagree with (CR 111.4).
    let mut s = state.clone();
    let token = s.create_token_with_subtypes(
        "Spirit", P0, 1, 1, vec![Color::White], vec![CardType::Creature],
        vec![Keyword::Flying], vec!["Spirit".into()], &reg)[0];
    s.move_object(token, Zone::Graveyard, &reg);
    let v = check_core(&s, &reg);
    assert!(!v.iter().any(|m| m.contains("keeps runtime characteristics")),
        "a token has nothing else to be: {v:?}");

    // And a permanent on the battlefield keeps what it picked up there.
    let mut s = state.clone();
    let bear = named_permanent(&mut s, &reg, "Grizzly Bears", P0);
    s.get_object_mut(bear).unwrap().keywords = vec![Keyword::Flying];
    let v = check_core(&s, &reg);
    assert!(!v.iter().any(|m| m.contains("keeps runtime characteristics")),
        "a granted keyword on the battlefield is not a violation: {v:?}");
}

/// CR 707.2: `copy_grantor` remembers the card a copy is a copy of, and for a
/// non-token that card is a real one. A TOKEN copy is the exception — its
/// grantor is the printed card of whatever it was made from, and a token made
/// by an effect rather than a card has none to name.
///
/// CR 702.34a: a card cast with flashback is an instant or a sorcery, and the
/// clause says so with a conjunction — with only one of the two card types
/// ever tried, either half could be dropped.
#[test]
fn the_copy_and_flashback_marks_name_the_kinds_of_card_that_can_carry_them() {
    let (mut state, reg) = base();
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);

    let mut s = state.clone();
    s.get_object_mut(bear).unwrap().copy_grantor = Some(CardId(424_242));
    flags(&s, &reg, "is not in the registry");

    // A token's grantor is not looked up: it has no printed card of its own.
    let mut s = state.clone();
    {
        let o = s.get_object_mut(bear).unwrap();
        o.is_token = true;
        o.copy_grantor = Some(CardId(424_242));
    }
    let v = check_core(&s, &reg);
    assert!(!v.iter().any(|m| m.contains("copy_grantor")),
        "a token copy names no printed card: {v:?}");

    // Flashback: an instant is fine, a creature is not.
    let mut s = state.clone();
    let bolt = spell_in_hand(&mut s, &reg, "Geistflame", P0);
    s.move_object(bolt, Zone::Exile, &reg);
    s.get_object_mut(bolt).unwrap().cast_with_flashback = true;
    s.get_object_mut(bolt).unwrap().cast_from_zone = Some(Zone::Graveyard);
    let v = check_core(&s, &reg);
    assert!(!v.iter().any(|m| m.contains("neither instant nor sorcery")),
        "Geistflame has flashback and is an instant: {v:?}");

    let mut s = state.clone();
    let creature = spell_in_hand(&mut s, &reg, "Grizzly Bears", P0);
    s.move_object(creature, Zone::Exile, &reg);
    s.get_object_mut(creature).unwrap().cast_with_flashback = true;
    s.get_object_mut(creature).unwrap().cast_from_zone = Some(Zone::Graveyard);
    flags(&s, &reg, "neither instant nor sorcery");

    // CR 702.34a: flashback casts the card from your graveyard, so the two
    // marks a cast leaves behind have to agree. Different abilities read
    // different ones — the exile on resolution reads the flashback flag,
    // "whenever you cast a spell from your graveyard" reads the zone — and a
    // spell carrying only one of them is a cast half the game can see
    // (issue #330).
    let mut s = state.clone();
    let bolt = spell_in_hand(&mut s, &reg, "Geistflame", P0);
    s.move_object(bolt, Zone::Exile, &reg);
    s.get_object_mut(bolt).unwrap().cast_with_flashback = true;
    flags(&s, &reg, "cast with flashback was cast from None");

    let mut s = state.clone();
    let bolt = spell_in_hand(&mut s, &reg, "Geistflame", P0);
    s.move_object(bolt, Zone::Exile, &reg);
    {
        let o = s.get_object_mut(bolt).unwrap();
        o.cast_with_flashback = true;
        o.cast_from_zone = Some(Zone::Hand);
    }
    flags(&s, &reg, "cast with flashback was cast from Some(Hand)");
}
