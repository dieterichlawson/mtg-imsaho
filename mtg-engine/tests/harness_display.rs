//! What the player is shown, as opposed to what the game knows.
//!
//! An LLM player sees the `GameView` and the labels on the actions offered to
//! it, and can only reason about what is in them. Three ways that has gone
//! wrong: printed P/T shown for a creature whose P/T is a
//! characteristic-defining ability (CR 208.2 — a CDA works in every zone),
//! the front-face name shown for a transformed card, and internal object
//! handles rendered into a label with `{:?}`.

mod common;
use common::*;
use mtg_engine::triggers::{PendingTrigger, TriggerEvent, TriggerSource};
use mtg_engine::actions::Target;
use mtg_engine::types::*;

/// The view reports effective P/T wherever the card is.
///
/// Geist-Honored Monk's "power and toughness are each equal to the number of
/// creatures you control" is a CDA, so it has a real size in the graveyard and
/// in hand as well as on the battlefield — and that is the number the player
/// needs in order to decide whether reanimating it is worth anything.
#[test]
fn the_view_shows_effective_power_in_every_zone() {
    let reg = registry();

    for zone in [Zone::Battlefield, Zone::Graveyard, Zone::Hand] {
        let mut state = game_at_step(Step::PrecombatMain, P0);
        // Two other creatures out, so the Monk's count differs from its
        // printed 0/0 in every zone.
        named_permanent(&mut state, &reg, "Grizzly Bears", P0);
        named_permanent(&mut state, &reg, "Grizzly Bears", P0);

        let card_id = reg.get_id_by_name("Geist-Honored Monk").unwrap();
        let monk = state.create_object(card_id, P0, zone, Some(0), Some(0));
        state.get_object_mut(monk).unwrap().name = "Geist-Honored Monk".into();
        state.get_object_mut(monk).unwrap().summoning_sick = false;

        let expected = state.effective_power(monk, &reg).expect("a CDA has a value");
        assert!(expected >= 2,
            "test precondition: in {zone:?} the value is {expected}, not the printed 0");

        let view = mtg_engine::view::GameView::for_player(&state, P0, &reg);
        let shown = match zone {
            Zone::Battlefield => view.battlefield.iter()
                .find(|c| c.object_id == monk).and_then(|c| c.effective_power),
            Zone::Graveyard => view.graveyards.iter()
                .find(|(pid, _)| *pid == P0)
                .and_then(|(_, cards)| cards.iter().find(|c| c.object_id == monk))
                .and_then(|c| c.power),
            _ => view.your_hand.iter().find(|c| c.object_id == monk).and_then(|c| c.power),
        };

        assert_eq!(shown, Some(expected),
            "in {zone:?} the view must show the Monk's effective power, not the \
             printed 0 — a CDA works in every zone (CR 208.2)");
    }
}

/// A transformed card's trigger is labelled with the face that is showing.
/// The battlefield says "Rampaging Werewolf"; a stack entry saying "Tormented
/// Pariah" describes a permanent the player cannot see.
#[test]
fn a_transformed_cards_trigger_label_names_the_face_that_is_showing() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    let pariah = named_permanent(&mut state, &reg, "Tormented Pariah", P0);
    mtg_engine::cards::helpers::apply_transform(&mut state, pariah, &reg);
    let card_id = state.get_object(pariah).unwrap().card_id;

    let trigger = PendingTrigger {
        source: TriggerSource::new(pariah, card_id, P0, "transform back if 2+ spells cast"),
        event: TriggerEvent::Upkeep,
    };
    let label = trigger.display_name_with_state(&reg, Some(&state));

    assert!(label.contains("Rampaging Werewolf"),
        "the label names the face that is on the battlefield; label = {label:?}");
    assert!(!label.contains("Tormented Pariah"),
        "and not the front face, which names a permanent that is not there; \
         label = {label:?}");
}

/// No ability label anywhere in the set renders an internal handle.
///
/// `ObjectId(5)` means nothing to a player — there is no way to map it back to
/// a creature. Skirsdag High Priest's tap-pair labels used to be built with
/// `{:?}`; this checks every card rather than that one, over a board with
/// enough going on for the enumerating abilities to enumerate something.
#[test]
fn no_ability_label_renders_an_internal_object_id() {
    let reg = registry();
    let mut offenders = Vec::new();
    let mut checked = 0;

    let mut names: Vec<String> = reg.all_names().iter().map(|s| (*s).to_string()).collect();
    names.sort();
    for name in names {
        let card_id = reg.get_id_by_name(&name).expect("named card has an id");
        let Some(behavior) = reg.get(card_id) else { continue };

        let mut state = game_at_step(Step::PrecombatMain, P0);
        state.creature_died_this_turn = true; // unlock the morbid ones
        let id = named_permanent(&mut state, &reg, &name, P0);
        // Fodder for abilities that enumerate creatures or graveyard cards.
        for _ in 0..3 {
            ready_creature(&mut state, P0, 2, 2);
            named_card_in_graveyard(&mut state, &reg, "Grizzly Bears", P0);
        }
        ready_creature(&mut state, P1, 2, 2);

        for ability in behavior.activated_abilities(&state, id, &reg) {
            checked += 1;
            if ability.description.contains("ObjectId(") {
                offenders.push(format!("{name}: {:?}", ability.description));
            }
        }
    }

    assert!(checked >= 20,
        "expected to have looked at a good number of ability labels, got {checked}");
    assert!(offenders.is_empty(),
        "{} ability label(s) render an internal handle the player cannot map to \
         anything:\n  {}", offenders.len(), offenders.join("\n  "));
}

/// A loyalty ability is offered with the cost the card prints.
///
/// The view builds each label from the ability's loyalty change, adding the
/// sign only when the card's own text doesn't already carry it. Get the
/// sign test wrong in either direction and the player is shown a doubled
/// cost — "+1: +1: Each player discards a card", or "+0: 0: Deal 3 damage".
#[test]
fn a_loyalty_ability_is_labelled_with_the_cost_the_card_prints() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Liliana of the Veil prints a plus ability and two minus abilities;
    // Garruk Relentless prints two zero-cost ones.
    let liliana = named_permanent(&mut state, &reg, "Liliana of the Veil", P0);
    let garruk = named_permanent(&mut state, &reg, "Garruk Relentless", P0);
    set_loyalty(&mut state, liliana, 3);
    set_loyalty(&mut state, garruk, 3);

    let view = mtg_engine::view::GameView::for_player(&state, P0, &reg);

    for id in [liliana, garruk] {
        let shown = &view.battlefield.iter()
            .find(|p| p.object_id == id).expect("on the battlefield")
            .loyalty_abilities;
        let printed = reg.get(state.get_object(id).unwrap().card_id).unwrap()
            .loyalty_abilities(&state, id);
        assert_eq!(shown.len(), printed.len(), "every loyalty ability is offered");
        for (ab, (index, label)) in printed.iter().zip(shown) {
            assert_eq!(*index, ab.ability_index);
            assert_eq!(label, ab.description.trim(),
                "the label is the printed text, with its cost written once: \
                 loyalty change {} rendered as {label:?}", ab.loyalty_change);
        }
    }
}

/// The full log is the game's record, not the players' — it keeps the
/// Debug-level bookkeeping the display log hides, and still drops the
/// Private lines that are one player's hidden information (issue #119).
#[test]
fn the_full_log_keeps_debug_lines_and_drops_private_ones() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    state.log(mtg_engine::state::LogLevel::Private, "p0 looked at Delver's top card".to_string());
    state.log(mtg_engine::state::LogLevel::Debug, "p0 tapped Forest for G".to_string());
    state.log(mtg_engine::state::LogLevel::Info, "p0 played a land".to_string());

    let view = mtg_engine::view::GameView::for_player(&state, P0, &reg);

    assert!(view.full_log.iter().any(|l| l.contains("tapped Forest")),
        "the full log keeps Debug lines: {:?}", view.full_log);
    assert!(view.full_log.iter().any(|l| l.contains("played a land")),
        "and everything above them: {:?}", view.full_log);
    assert!(!view.full_log.iter().any(|l| l.contains("top card")),
        "but never a Private line, not even for the player it belongs to: {:?}",
        view.full_log);

    assert!(!view.display_log.iter().any(|l| l.contains("tapped Forest")),
        "the display log starts one level higher: {:?}", view.display_log);
}

/// Nevermore's chosen name is public information, so the view carries it.
/// Without it the player sees an enchantment with no indication of which
/// card it is turning off.
#[test]
fn the_view_reports_the_name_a_nevermore_chose() {
    use mtg_engine::types::ContinuousEffect;

    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let nevermore = named_permanent(&mut state, &reg, "Nevermore", P0);
    state.get_object_mut(nevermore).unwrap().instance_continuous_effects = Some(vec![
        ContinuousEffect::PreventCastingNamed { name: "Lightning Bolt".into() },
    ]);
    let bears = named_permanent(&mut state, &reg, "Grizzly Bears", P0);

    for seat in [P0, P1] {
        let view = mtg_engine::view::GameView::for_player(&state, seat, &reg);
        let row = view.battlefield.iter()
            .find(|p| p.object_id == nevermore).expect("on the battlefield");
        assert_eq!(row.named_card.as_deref(), Some("Lightning Bolt"),
            "p{} must be able to read the banned name off the Nevermore", seat.0);
        assert_eq!(view.battlefield.iter()
            .find(|p| p.object_id == bears).unwrap().named_card, None,
            "a permanent that named nothing carries no name");
    }
}

/// CR 510.4: the two combat damage steps are distinct, and the view has to
/// say which one the player is in — the first-strike half and the regular
/// half offer the same actions and would otherwise be indistinguishable
/// (issue #140).
#[test]
fn the_view_and_the_prompt_name_which_combat_damage_step_this_is() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);

    let attacker = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(attacker).unwrap().keywords.push(Keyword::FirstStrike);
    let blocker = ready_creature(&mut state, P1, 4, 4);
    mtg_engine::combat::declare_attackers(&mut state, &[(attacker, P1)], &[], &reg);
    mtg_engine::combat::declare_blockers(&mut state, &[(blocker, attacker)]);

    // First instance: first-strike damage, with a second step to come.
    mtg_engine::engine::advance_step(&mut state, &reg);
    assert_eq!(state.step, Step::CombatDamage);
    let view = mtg_engine::view::GameView::for_player(&state, P0, &reg);
    assert!(view.first_strike_damage_step,
        "the first of the two damage steps is flagged as the first-strike one");
    assert_eq!(mtg_engine::engine::legal_actions(&state, &reg).context.as_deref(),
        Some("FIRST-STRIKE COMBAT DAMAGE"), "and the prompt says so too");

    // Second instance: regular damage, nothing further pending.
    mtg_engine::engine::advance_step(&mut state, &reg);
    assert_eq!(state.step, Step::CombatDamage);
    let view = mtg_engine::view::GameView::for_player(&state, P0, &reg);
    assert!(!view.first_strike_damage_step,
        "the regular damage step is the same Step with the flag off — the view \
         must not report it as the first-strike half");
    assert_eq!(mtg_engine::engine::legal_actions(&state, &reg).context.as_deref(),
        Some("COMBAT DAMAGE"), "and the prompt names it the regular half");

    // A step that is not the combat damage step is never either of them.
    mtg_engine::engine::advance_step(&mut state, &reg);
    assert_eq!(state.step, Step::EndCombat);
    assert!(!mtg_engine::view::GameView::for_player(&state, P0, &reg).first_strike_damage_step);
}

/// The names a prompt asks about reach the player who has to answer.
///
/// A prompt can name cards in a hidden zone — a hand, a library, the cards
/// an effect looked at — and the object ids alone say nothing. `for_player`
/// resolves them into `revealed_names`, one match arm per prompt kind, and
/// an arm that goes missing leaves that prompt unreadable.
#[test]
fn a_prompt_that_names_hidden_cards_carries_their_names() {
    use mtg_engine::state::{AwaitingAction, PendingEffect, ResolutionChoiceKind as K};

    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let source = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let in_hand = spell_in_hand(&mut state, &reg, "Moment of Heroism", P1);
    let in_library = state.create_object(
        reg.get_id_by_name("Forest").unwrap(), P1, Zone::Library, None, None);
    state.get_object_mut(in_library).unwrap().name = "Forest".into();
    state.get_player_mut(P1).library_order.push(in_library);

    let cases = [
        ("a target picker", K::ChooseTarget {
            description: "d".into(), options: vec![Target::Object(in_hand)],
            optional: false,
            effect: PendingEffect::CardEffect { source_id: source, key: String::new() },
        }, in_hand),
        ("a hand prompt", K::ChooseCardFromHand {
            description: "d".into(), player: P1, cards: vec![in_hand],
            discard_immediately: true, remaining: 1,
        }, in_hand),
        ("a look at cards", K::ChooseFromLookedAt {
            description: "d".into(), looked_at: vec![in_library],
        }, in_library),
        ("a library search", K::ChooseFromLibrary {
            description: "d".into(), options: vec![in_library], searcher: P1,
            source_id: source, destination: Zone::Hand, tapped: false,
        }, in_library),
    ];

    for (what, choice, named) in cases {
        let mut s = state.clone();
        s.awaiting_action = Some(AwaitingAction::ResolutionChoice {
            player: P1, source, choice,
        });
        let view = mtg_engine::view::GameView::for_player(&s, P1, &reg);
        assert!(view.revealed_names.contains_key(&named),
            "{what} names #{} from a hidden zone, so the view carries its name; \
             revealed_names = {:?}", named.0, view.revealed_names);
    }

    // With no prompt up there is nothing to reveal.
    let view = mtg_engine::view::GameView::for_player(&state, P1, &reg);
    assert!(view.revealed_names.is_empty(),
        "nothing is revealed with no prompt up: {:?}", view.revealed_names);
}

/// The view is one seat's view: the opponents list is everyone else, and
/// the display log starts at Info.
#[test]
fn the_view_is_one_seats_view_of_the_game() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    state.log(mtg_engine::state::LogLevel::Debug, "p0 tapped Forest for G".to_string());
    state.log(mtg_engine::state::LogLevel::Info, "p0 played a land".to_string());

    let view = mtg_engine::view::GameView::for_player(&state, P0, &reg);
    assert_eq!(view.opponents.len(), 1, "a two-player game has one opponent");
    assert_eq!(view.opponents[0].id, P1, "and it is the other seat, not this one");

    assert!(view.display_log.iter().any(|l| l.contains("played a land")),
        "the display log starts at Info: {:?}", view.display_log);
    assert!(!view.display_log.iter().any(|l| l.contains("tapped Forest")),
        "and stops below it: {:?}", view.display_log);
}

/// CR 117.1/110.4: two predicates the whole engine reads through.
#[test]
fn the_step_and_card_type_predicates_say_what_they_mean() {
    // CR 502/514: no player receives priority in the untap or cleanup step.
    for step in [Step::Untap, Step::Cleanup] {
        assert!(!step.has_priority(), "{step:?} gives nobody priority");
    }
    for step in [Step::Upkeep, Step::Draw, Step::PrecombatMain, Step::BeginCombat,
                 Step::DeclareAttackers, Step::DeclareBlockers, Step::CombatDamage,
                 Step::EndCombat, Step::PostcombatMain, Step::EndStep] {
        assert!(step.has_priority(), "{step:?} is a step with priority");
    }

    // CR 110.4a: the five permanent types, and nothing else.
    for ty in [CardType::Land, CardType::Creature, CardType::Enchantment,
               CardType::Artifact, CardType::Planeswalker] {
        assert!(ty.is_permanent(), "{ty:?} is a permanent type");
    }
    for ty in [CardType::Instant, CardType::Sorcery] {
        assert!(!ty.is_permanent(), "{ty:?} is never a permanent");
    }
}

/// A prompt's own description is what the player is shown; the generic
/// header is the fallback for a prompt that did not write one.
///
/// Issue #87: the header was shown unconditionally, and it says what KIND of
/// decision this is without saying what the decision is about. Delver of
/// Secrets' "you may reveal the top card" names that card in its description
/// (CR 701.20a), and under the header the choice was blind — the player was
/// asked yes or no about a card they could not see.
///
/// Both halves are here because the clause is a `!is_empty()` guard on a
/// match arm: a version that always takes the description shows an empty
/// label for the prompts that have none, and a version that never takes it
/// is issue #87 again.
#[test]
fn a_prompt_shows_its_own_description_and_falls_back_to_a_header() {
    use mtg_engine::state::{AwaitingAction, ResolutionChoiceKind as K};

    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let source = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let looked = state.create_object(
        reg.get_id_by_name("Forest").unwrap(), P0, Zone::Library, None, None);
    state.get_object_mut(looked).unwrap().name = "Forest".into();
    state.get_player_mut(P0).library_order.push(looked);

    let described = |choice| {
        let mut s = state.clone();
        s.priority_player = None;
        s.awaiting_action = Some(AwaitingAction::ResolutionChoice {
            player: P0, source, choice,
        });
        mtg_engine::engine::legal_actions(&s, &reg).context.unwrap_or_default()
    };

    let yes_no = |description: &str| K::YesNo {
        description: description.into(), source_card: source,
    };
    assert_eq!(described(yes_no("Delver of Secrets: reveal Forest?")),
        "Delver of Secrets: reveal Forest?",
        "the description says what is being decided AND what it is about");
    let source_name = state.obj_name(source);
    assert_eq!(described(yes_no("")), format!("{source_name}: choose yes or no"),
        "and a prompt with nothing to add falls back to the header");

    let looked_at = |description: &str| K::ChooseFromLookedAt {
        description: description.into(), looked_at: vec![looked],
    };
    assert_eq!(described(looked_at("Forbidden Alchemy: choose a card to put into your hand")),
        "Forbidden Alchemy: choose a card to put into your hand");
    assert_eq!(described(looked_at("")), format!("{source_name}: choose a card"));
}

/// The header a player gets while something is on the stack names WHOSE
/// spell they are responding to, from their own seat: "your" for the one
/// they cast, and the other player by number.
///
/// Both seats are asked because the clause is one comparison, and read from
/// one seat a version that has it backwards says something plausible.
#[test]
fn the_respond_header_names_whose_spell_it_is_from_each_seat() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    let bolt = castable_spell(&mut state, &reg, "Geistflame", P0);
    let state = cast_onto_stack(&state, &reg, bolt, vec![Target::Object(bear)]);

    let header = |seat| {
        let mut s = state.clone();
        s.priority_player = Some(seat);
        mtg_engine::engine::legal_actions(&s, &reg).context.unwrap_or_default()
    };
    let spell_name = state.obj_name(bolt);
    assert_eq!(header(P0), format!("RESPOND TO your {spell_name}"),
        "p0 cast it, so p0 is told it is theirs");
    assert_eq!(header(P1), format!("RESPOND TO p0's {spell_name}"),
        "and p1 is told whose it is");
}

/// An offered ability carries its own description — the sentence the card
/// prints — so the entry a player picks says what it does.
///
/// The lookup finds the ability by index among the ones its source has, and
/// a lookup that matched the wrong one leaves the label empty on every card
/// with a single ability, which is most of them.
#[test]
fn an_offered_ability_carries_its_own_description() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let priest = named_permanent(&mut state, &reg, "Avacynian Priest", P0);
    state.get_object_mut(priest).unwrap().summoning_sick = false;
    named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    add_mana(&mut state, P0, &[(ManaType::White, 1)]);
    state.priority_player = Some(P0);

    let offered = mtg_engine::engine::legal_actions(&state, &reg).activatable_abilities;
    let entry = offered.iter().find(|a| a.object_id == priest)
        .expect("the Priest's tap ability is offered");
    let printed = reg.get(state.get_object(priest).unwrap().card_id)
        .expect("registered")
        .activated_abilities(&state, priest, &reg)
        .into_iter()
        .find(|a| a.ability_index == entry.ability_index)
        .expect("the ability it says it is")
        .description;
    assert!(!entry.description.is_empty(), "the offer is labelled");
    assert_eq!(entry.description, printed,
        "and the label is this ability's own sentence, not another's");
    // And it carries what the ability can be pointed at. The collapsed view
    // is how both clients pick a target, so an entry with an empty list is
    // an ability they cannot activate even though the flat action list
    // offers it.
    assert!(!entry.target_options.is_empty(),
        "the Priest taps a creature, so the entry offers one: {entry:?}");
}

/// CR 400.2/603.3d: the stack is public and a trigger's targets are chosen as
/// it goes on the stack, so both seats can see what it is pointed at. Issue
/// #134 — the panel never showed them.
///
/// The accessor that carries them is used by the stack view and by the log
/// line naming the target (issue #135), and by nothing else: stubbing it to
/// return no targets at all passed the whole suite. This is that surface,
/// which is where the two issues were filed about.
#[test]
fn the_stack_view_shows_what_a_trigger_is_pointed_at() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let hunter = named_permanent(&mut state, &reg, "Fiend Hunter", P0);
    let victim = named_permanent(&mut state, &reg, "Grizzly Bears", P1);

    let card_id = state.get_object(hunter).unwrap().card_id;
    let mut source = mtg_engine::triggers::TriggerSource::new(
        hunter, card_id, P0, "you may exile another target creature");
    source.chosen_targets = vec![Target::Object(victim)];
    state.stack.push(mtg_engine::state::StackEntry::Trigger(
        mtg_engine::triggers::PendingTrigger::new(
            source, mtg_engine::triggers::TriggerEvent::SelfEntered)));

    for seat in [P0, P1] {
        let view = mtg_engine::view::GameView::for_player(&state, seat, &reg);
        let item = view.stack.iter()
            .find(|s| s.controller == P0)
            .unwrap_or_else(|| panic!("p{}'s view shows the trigger", seat.0));
        assert_eq!(item.targets, vec![Target::Object(victim)],
            "and says what it is pointed at, from either seat");
    }
}
