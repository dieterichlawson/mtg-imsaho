//! The permanents whose abilities trigger on another object's entering,
//! a spell being cast, combat damage, or blocks (CR 603.2), with the
//! condition each one's text puts on that event.
//!
//! Scope: proved here is what each card declares (its abilities, costs,
//! targets, triggers and static effects) and what its own code writes to
//! the game state (a question asked, an until-end-of-turn effect added).
//! What a resolution does through the engine, the moves, damage, draws
//! and destruction, is not proved here, and where the text below speaks
//! of one it is of that part alone: each card is played end to end with
//! the real engine by `tests/card_semantics.rs`.
//!
//! - Champion of the Parish, 1/1: "Whenever another Human you control
//!   enters, put a +1/+1 counter on this creature."
//! - Mentor of the Meek, 2/2: "Whenever another creature you control with
//!   power 2 or less enters, you may pay {1}. If you do, draw a card."
//! - Burning Vengeance, Enchantment: "Whenever you cast a spell from your
//!   graveyard, this enchantment deals 2 damage to any target."
//! - Charmbreaker Devils, 4/4: "Whenever you cast an instant or sorcery
//!   spell, this creature gets +4/+0 until end of turn." (Its upkeep
//!   trigger is `proofs::cards::steps`'.)
//! - Rakish Heir, 2/2: "Whenever a Vampire you control deals combat damage
//!   to a player, put a +1/+1 counter on it."
//! - Ashmouth Hound, 2/1: "Whenever this creature blocks or becomes blocked
//!   by a creature, this creature deals 1 damage to that creature."
//! - Hamlet Captain, 2/2: "Whenever this creature attacks or blocks, other
//!   Humans you control get +1/+1 until end of turn."
//!
//! Each proof pins the triggers the card declares and the condition the
//! engine asks it about the event, over every such event on a board of a
//! card and a token of any characteristics; then runs the card's code for
//! the trigger resolving, the permanent on the battlefield or gone since,
//! under either player's control. That the watcher is never told about its
//! own entering ("another") is the engine's (`triggers::collect`), proved
//! in `proofs::actions::triggering`.

use super::{acted, any_object, any_object_of, board, card_proof, case_by_case, each_is, kept, triggers, Form, Kinds, Snapshot};
use crate::cards::{AttackInfo, CardBehavior, CardRegistry, TriggerKind};
use crate::ids::{CardId, ObjectId, PlayerId};
use crate::state::{GameObject, GameState, TemporaryEffect as T};
use crate::types::{CardType, Zone};

use crate::cards::isd;

const PEOPLE: Kinds = Kinds { types: &[CardType::Creature], subtypes: &["Human", "Vampire"], keywords: &[] };

/// The board: the card's permanent, owned by player 0, on the battlefield
/// or gone to their graveyard, controlled (or last controlled) by `you`;
/// a card and a token beside it.
struct Board {
    registry: CardRegistry,
    state: GameState,
    me: ObjectId,
    others: [ObjectId; 2],
    you: PlayerId,
}

fn watching(card: Box<dyn CardBehavior>, can_be_gone: bool) -> Board {
    watching_of(card, can_be_gone, &PEOPLE)
}

/// [`watching`]'s board, the other two objects of `kinds`.
fn watching_of(card: Box<dyn CardBehavior>, can_be_gone: bool, kinds: &Kinds) -> Board {
    watching_on(card, can_be_gone, |state, blank| {
        [any_object(state, blank, kinds, Form::Card), any_object(state, blank, kinds, Form::Token)]
    })
}

/// [`watching_of`]'s board, the card of exactly `subtypes[0]` and the
/// token of `subtypes[1]`.
fn watching_with(card: Box<dyn CardBehavior>, can_be_gone: bool, kinds: &Kinds, subtypes: [&[&str]; 2]) -> Board {
    watching_on(card, can_be_gone, |state, blank| {
        [any_object_of(state, blank, kinds, Form::Card, subtypes[0]), any_object_of(state, blank, kinds, Form::Token, subtypes[1])]
    })
}

/// The board with the other two objects made by `others`.
fn watching_on(card: Box<dyn CardBehavior>, can_be_gone: bool, others: impl FnOnce(&mut GameState, CardId) -> [ObjectId; 2]) -> Board {
    let (registry, mut state, blank, card) = board(card);
    let you = PlayerId(u8::from(kani::any::<bool>()));
    let gone = can_be_gone && kani::any();
    let me = state.create_object(card, PlayerId(0), if gone { Zone::Graveyard } else { Zone::Battlefield }, None, None);
    let o = state.get_object_mut(me).expect("just created");
    if gone {
        o.last_controller = Some(you);
        state.players[0].graveyard_order.push(me);
    } else {
        o.controller = you;
    }
    let others = others(&mut state, blank);
    Board { registry, state, me, others, you }
}

impl Board {
    fn card(&self) -> CardId {
        self.state.get_object(self.me).expect("on the board").card_id
    }

    fn get(&self, id: ObjectId) -> &GameObject {
        self.state.get_object(id).expect("on the board")
    }

    fn behavior(&self) -> &dyn CardBehavior {
        self.registry.get(self.card()).expect("registered")
    }

    fn is(&self, id: ObjectId, subtype: &str) -> bool {
        self.get(id).subtypes.iter().any(|s| s == subtype)
    }
}

/// Mentor of the Meek's board: its creature and two others, of no
/// subtype, which nothing here reads.
const CREATURES: Kinds = Kinds { types: &[CardType::Creature], subtypes: &[], keywords: &[] };

/// Mentor of the Meek: triggers for another creature with power 2 or less
/// entering under its controller's control, and nothing else entering.
/// Its effect is the proof below. For the creature that entered `which`
/// (the Mentor itself 0, the card 1, the token 2), a constant in each
/// harness: its power is read through the layers, and with the creature
/// the solver's, every object's was, and the proof ran out of memory at
/// four and a half gigabytes in symbolic execution.
fn mentor_of_the_meek_triggers_for_another_small_creature_of_yours(which: u8) {
    assert!(triggers(&isd::mentor_of_the_meek::MentorOfTheMeek) == [(TriggerKind::AnyCreatureEnters, None)]);
    let b = watching_of(Box::new(isd::mentor_of_the_meek::MentorOfTheMeek), false, &CREATURES);
    let entered = match which { 0 => b.me, 1 => b.others[0], _ => b.others[1] };
    let under = PlayerId(u8::from(kani::any::<bool>()));
    let power = b.state.effective_power(entered, &b.registry);
    let fires = b.behavior().should_trigger_on_creature_enters(&b.state, b.me, entered, under, &b.registry);
    assert!(fires == (entered != b.me && under == b.you && power.is_some_and(|p| p <= 2)));
    std::mem::forget(b);
}

case_by_case! { mentor_of_the_meek_triggers_for_another_small_creature_of_yours;
    mentor_of_the_meek_does_not_trigger_for_itself: 0,
    mentor_of_the_meek_triggers_for_a_small_creature_card: 1,
    mentor_of_the_meek_triggers_for_a_small_creature_token: 2,
}

card_proof! {
    /// Charmbreaker Devils: triggers for an instant or sorcery spell its
    /// controller casts, and no other spell (CR 603.2: both are the
    /// trigger's condition, asked once, as it triggers); resolving, +4/+0
    /// until end of turn on itself, whoever controls it by then (CR
    /// 113.7a), and nothing else.
    fn charmbreaker_devils_gets_four_when_you_cast_an_instant_or_sorcery() {
        let devils = isd::charmbreaker_devils::CharmbreakerDevils;
        assert!(triggers(&devils) == [(TriggerKind::Upkeep, None), (TriggerKind::SpellCast, None)]);
        let mut b = watching(Box::new(devils), false);
        let caster = PlayerId(u8::from(kani::any::<bool>()));
        let spell = b.state.create_object(CardId(1), caster, Zone::Stack, None, None);
        let kind = match kani::any::<u8>() % 3 { 0 => CardType::Instant, 1 => CardType::Sorcery, _ => CardType::Creature };
        b.state.get_object_mut(spell).expect("just created").card_types = vec![kind];
        let fires = b.behavior().should_trigger_on_spell_cast(&b.state, b.me, caster, spell, &b.registry);
        assert!(fires == (caster == b.you && kind != CardType::Creature));
        let before = Snapshot::of(&b.state);
        let card = b.card();
        b.registry.get(card).expect("registered").on_spell_cast(&mut b.state, b.me, caster, spell, &[], &b.registry);
        assert!(b.state.until_end_of_turn[before.until_end_of_turn()..] == [T::ModifyPT { target: b.me, power_mod: 4, toughness_mod: 0 }]);
        assert!(acted() == 0 && Snapshot::of(&b.state).without_until_end_of_turn() == before.without_until_end_of_turn());
        std::mem::forget(b);
    }
}

/// The subtypes [`hamlet_captain_pumps_your_other_humans`] gives each of
/// the other two objects: none, Human, or Vampire.
const HAMLET_SUBTYPES: [&[&str]; 3] = [&[], &["Human"], &["Vampire"]];

/// Hamlet Captain: triggers as it attacks and as it blocks; +1/+1 until
/// end of turn on each other Human creature its controller controls. With
/// the other two objects' subtypes those of `HAMLET_SUBTYPES` at `card`
/// and `token`, and the trigger its attack's or (if `blocks`) its
/// block's, each a constant in each harness: over subtypes the solver
/// chose, each object's compared with "Human" byte by byte, the proof ran
/// out of memory at four and a half gigabytes, and with the subtypes
/// constants and both triggers in one harness, at four.
fn hamlet_captain_pumps_your_other_humans((card, token, blocks): (usize, usize, bool)) {
    assert!(triggers(&isd::hamlet_captain::HamletCaptain) == [(TriggerKind::Attacks, None), (TriggerKind::Blocks, None)]);
    let mut b = watching_with(Box::new(isd::hamlet_captain::HamletCaptain), true, &PEOPLE, [HAMLET_SUBTYPES[card], HAMLET_SUBTYPES[token]]);
    let humans = kept(&b.others, |id| {
        let o = b.get(id);
        o.zone == Zone::Battlefield && o.controller == b.you && o.card_types.contains(&CardType::Creature) && b.is(id, "Human")
    });
    let before = Snapshot::of(&b.state);
    let card = b.card();
    let captain = b.registry.get(card).expect("registered");
    if blocks {
        captain.on_blocks(&mut b.state, b.me, b.others[0], &b.registry);
    } else {
        captain.on_attacks(&mut b.state, b.me, AttackInfo::new(b.me, PlayerId(1 - b.you.0)), &[], &b.registry);
    }
    let added = &b.state.until_end_of_turn[before.until_end_of_turn()..];
    assert!(each_is(added, humans.len(), |i| T::ModifyPT { target: humans[i], power_mod: 1, toughness_mod: 1 }) && acted() == 0);
    assert!(Snapshot::of(&b.state).without_until_end_of_turn() == before.without_until_end_of_turn());
    std::mem::forget(b);
}

case_by_case! { hamlet_captain_pumps_your_other_humans;
    hamlet_captain_attacking_with_neither_other_object_of_a_subtype: (0, 0, false),
    hamlet_captain_attacking_with_the_token_a_human: (0, 1, false),
    hamlet_captain_attacking_with_the_token_a_vampire: (0, 2, false),
    hamlet_captain_attacking_with_the_card_a_human: (1, 0, false),
    hamlet_captain_attacking_with_both_humans: (1, 1, false),
    hamlet_captain_attacking_with_the_card_a_human_and_the_token_a_vampire: (1, 2, false),
    hamlet_captain_attacking_with_the_card_a_vampire: (2, 0, false),
    hamlet_captain_attacking_with_the_card_a_vampire_and_the_token_a_human: (2, 1, false),
    hamlet_captain_attacking_with_both_vampires: (2, 2, false),
    hamlet_captain_blocking_with_neither_other_object_of_a_subtype: (0, 0, true),
    hamlet_captain_blocking_with_the_token_a_human: (0, 1, true),
    hamlet_captain_blocking_with_the_token_a_vampire: (0, 2, true),
    hamlet_captain_blocking_with_the_card_a_human: (1, 0, true),
    hamlet_captain_blocking_with_both_humans: (1, 1, true),
    hamlet_captain_blocking_with_the_card_a_human_and_the_token_a_vampire: (1, 2, true),
    hamlet_captain_blocking_with_the_card_a_vampire: (2, 0, true),
    hamlet_captain_blocking_with_the_card_a_vampire_and_the_token_a_human: (2, 1, true),
    hamlet_captain_blocking_with_both_vampires: (2, 2, true),
}
