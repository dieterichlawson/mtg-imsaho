//! The permanents whose abilities trigger at the beginning of a step (CR
//! 503.1a, 513.1a): whose step, what they target, and what they do.
//!
//! - Endless Ranks of the Dead, Enchantment: "At the beginning of your
//!   upkeep, create X 2/2 black Zombie creature tokens, where X is half the
//!   number of Zombies you control, rounded down."
//! - Bloodgift Demon, 5/4 Flying: "At the beginning of your upkeep, target
//!   player draws a card and loses 1 life."
//! - Angel of Flight Alabaster, 4/3 Flying: "At the beginning of your
//!   upkeep, return target Spirit card from your graveyard to your hand."
//! - Charmbreaker Devils, 4/4: "At the beginning of your upkeep, return an
//!   instant or sorcery card at random from your graveyard to your hand."
//! - Mayor of Avabruck's back face, Howlpack Alpha, 3/3: "At the beginning
//!   of your end step, create a 2/2 green Wolf creature token."
//! - Reaper from the Abyss, 6/6 Flying: "Morbid — At the beginning of each
//!   end step, if a creature died this turn, destroy target non-Demon
//!   creature."
//!
//! "Your upkeep" is the upkeep of the permanent's controller, and "each end
//! step" every player's: the scope the engine reads, pinned here, and the
//! engine's dispatch on it proved in `proofs::actions`. Each resolution is
//! over the permanent on the battlefield or gone by then, under either
//! player's control, and a card and a token of any characteristics.

use super::{acted, any_object, any_target, asked_for, board, card_proof, triggers, Act, Form, Kinds, Snapshot};
use crate::actions::Target;
use crate::cards::{CardBehavior, CardRegistry, TargetFilter, TargetRequirement as R, TriggerKind, TriggerScope};
use crate::ids::{CardId, ObjectId, PlayerId};
use crate::state::{GameObject, GameState, PendingEffect};
use crate::types::{CardType, Color, Zone};

use crate::cards::isd;

const KINDS: Kinds = Kinds { types: &[CardType::Creature], subtypes: &["Zombie", "Spirit", "Demon"], keywords: &[] };

struct Board {
    registry: CardRegistry,
    state: GameState,
    me: ObjectId,
    others: [ObjectId; 2],
    you: PlayerId,
}

fn permanent(card: Box<dyn CardBehavior>) -> Board {
    let (registry, mut state, blank, card) = board(card);
    let you = PlayerId(u8::from(kani::any::<bool>()));
    let gone: bool = kani::any();
    let me = state.create_object(card, PlayerId(0), if gone { Zone::Graveyard } else { Zone::Battlefield }, None, None);
    let o = state.get_object_mut(me).expect("just created");
    if gone {
        o.last_controller = Some(you);
        state.players[0].graveyard_order.push(me);
    } else {
        o.controller = you;
    }
    state.creature_died_this_turn = kani::any();
    let others = [any_object(&mut state, blank, &KINDS, Form::Card), any_object(&mut state, blank, &KINDS, Form::Token)];
    Board { registry, state, me, others, you }
}

impl Board {
    fn card(&self) -> CardId {
        self.state.get_object(self.me).expect("on the board").card_id
    }

    fn get(&self, id: ObjectId) -> &GameObject {
        self.state.get_object(id).expect("on the board")
    }

    fn is(&self, id: ObjectId, subtype: &str) -> bool {
        self.get(id).subtypes.iter().any(|s| s == subtype)
    }

    fn upkeep(&mut self, targets: &[Target]) -> Snapshot {
        let before = Snapshot::of(&self.state);
        let card = self.card();
        self.registry.get(card).expect("registered").on_upkeep(&mut self.state, self.me, targets, &self.registry);
        before
    }
}

/// `card`'s one triggered ability is "at the beginning of your upkeep",
/// targeting as `target` says.
fn your_upkeep(card: &dyn CardBehavior, target: Option<R>) {
    assert!(triggers(card) == [(TriggerKind::Upkeep, target)]);
    for back in [false, true] {
        assert!(card.step_trigger_scope(&TriggerKind::Upkeep, back) == TriggerScope::Your);
    }
}

card_proof! {
    /// Endless Ranks of the Dead: X 2/2 black Zombie creature tokens for
    /// its controller, X half the Zombie creatures they control as it
    /// resolves, rounded down; and nothing else.
    fn endless_ranks_of_the_dead_makes_half_your_zombies_again() {
        your_upkeep(&isd::endless_ranks_of_the_dead::EndlessRanksOfTheDead, None);
        let mut b = permanent(Box::new(isd::endless_ranks_of_the_dead::EndlessRanksOfTheDead));
        let zombies = b.others.iter().filter(|id| {
            let o = b.get(**id);
            o.zone == Zone::Battlefield && o.controller == b.you && o.card_types.contains(&CardType::Creature) && b.is(**id, "Zombie")
        }).count();
        let before = b.upkeep(&[]);
        assert!(asked_for(&[Act::Tokens {
            n: u32::try_from(zombies / 2).expect("small"), controller: b.you, power: 2, toughness: 2,
            colors: vec![Color::Black], types: vec![CardType::Creature], keywords: vec![], subtypes: vec!["Zombie".into()],
        }]));
        assert!(Snapshot::of(&b.state) == before);
        std::mem::forget(b);
    }
}

card_proof! {
    /// Bloodgift Demon: its target player draws a card and then loses 1
    /// life; with its target gone illegal, nothing.
    fn bloodgift_demon_makes_its_target_draw_and_lose_one() {
        your_upkeep(&isd::bloodgift_demon::BloodgiftDemon, Some(R::PlayerOnly));
        let mut b = permanent(Box::new(isd::bloodgift_demon::BloodgiftDemon));
        let target = any_target(b.others);
        let before = b.upkeep(&[target.clone()]);
        match target {
            Target::Player(p) => {
                assert!(asked_for(&[Act::Draw(p, 1), Act::Life(p, -1)]));
            }
            _ => {
                assert!(acted() == 0);
            }
        }
        assert!(Snapshot::of(&b.state) == before);
        std::mem::forget(b);
    }
}

card_proof! {
    /// Angel of Flight Alabaster: its target a Spirit card in its
    /// controller's graveyard (the engine asks the requirement, then the
    /// card), returned to its owner's hand.
    fn angel_of_flight_alabaster_returns_a_spirit_card() {
        your_upkeep(&isd::angel_of_flight_alabaster::AngelOfFlightAlabaster, Some(R::GraveyardCardOwnedByCaster));
        let mut b = permanent(Box::new(isd::angel_of_flight_alabaster::AngelOfFlightAlabaster));
        let candidate = b.others[0];
        let valid = isd::angel_of_flight_alabaster::AngelOfFlightAlabaster
            .is_valid_target(&b.state, b.you, &Target::Object(candidate), &b.registry);
        assert!(valid == (b.get(candidate).zone == Zone::Graveyard && b.is(candidate, "Spirit")));
        let target = any_target(b.others);
        let before = b.upkeep(&[target.clone()]);
        assert!(asked_for(&[Act::Effect(target, PendingEffect::ReturnToHand { source_name: "Angel of Flight Alabaster".into() })]));
        assert!(Snapshot::of(&b.state) == before);
        std::mem::forget(b);
    }
}

card_proof! {
    /// Reaper from the Abyss: triggers at each end step, if a creature died
    /// this turn (checked as it triggers and as it resolves, CR 603.4); its
    /// target a non-Demon creature on the battlefield; destroyed.
    fn reaper_from_the_abyss_destroys_a_non_demon_after_a_death() {
        let reaper = isd::reaper_from_the_abyss::ReaperFromTheAbyss;
        assert!(triggers(&reaper) == [(TriggerKind::EndStep, Some(R::CreatureWithFilter(TargetFilter::NotSubtypes(vec!["Demon".into()]))))]);
        for back in [false, true] {
            assert!(reaper.step_trigger_scope(&TriggerKind::EndStep, back) == TriggerScope::Each);
        }
        let mut b = permanent(Box::new(isd::reaper_from_the_abyss::ReaperFromTheAbyss));
        let died = b.state.creature_died_this_turn;
        assert!(reaper.should_trigger(&b.state, b.me, &TriggerKind::EndStep, &b.registry) == died);
        let candidate = b.others[0];
        let o = b.get(candidate);
        let creature = o.zone == Zone::Battlefield && o.card_types.contains(&CardType::Creature);
        assert!(reaper.is_valid_target(&b.state, b.you, &Target::Object(candidate), &b.registry) == (creature && !b.is(candidate, "Demon")));
        let target = any_target(b.others);
        let before = Snapshot::of(&b.state);
        let card = b.card();
        b.registry.get(card).expect("registered").on_end_step(&mut b.state, b.me, &[target.clone()], &b.registry);
        if died {
            assert!(asked_for(&[Act::Effect(target, PendingEffect::DestroyCreature { source_name: "Reaper from the Abyss".into() })]));
        } else {
            assert!(acted() == 0);
        }
        assert!(Snapshot::of(&b.state) == before);
        std::mem::forget(b);
    }
}

card_proof! {
    /// Charmbreaker Devils: at the beginning of its controller's upkeep, an
    /// instant or sorcery card (no token, CR 109.1) chosen at random from
    /// that player's graveyard returned to their hand; with none there,
    /// nothing. Over the Devils on the battlefield or gone, and two cards
    /// of any card type of an instant, a sorcery or a creature in either
    /// player's graveyard, or not.
    fn charmbreaker_devils_returns_an_instant_or_sorcery_at_random() {
        let devils = isd::charmbreaker_devils::CharmbreakerDevils;
        for back in [false, true] {
            assert!(devils.step_trigger_scope(&TriggerKind::Upkeep, back) == TriggerScope::Your);
        }
        let (registry, mut state, blank, card) = board(Box::new(devils));
        let you = PlayerId(u8::from(kani::any::<bool>()));
        let gone: bool = kani::any();
        let me = state.create_object(card, PlayerId(0), if gone { Zone::Graveyard } else { Zone::Battlefield }, None, None);
        if gone {
            state.get_object_mut(me).expect("just created").last_controller = Some(you);
            state.players[0].graveyard_order.push(me);
        } else {
            state.get_object_mut(me).expect("just created").controller = you;
        }
        let mut cards = [me; 2];
        let mut returnable = Vec::with_capacity(2);
        for slot in &mut cards {
            let owner = PlayerId(u8::from(kani::any::<bool>()));
            let in_graveyard: bool = kani::any();
            let id = state.create_object(blank, owner, if in_graveyard { Zone::Graveyard } else { Zone::Hand }, None, None);
            let kind = match kani::any::<u8>() % 3 { 0 => CardType::Instant, 1 => CardType::Sorcery, _ => CardType::Creature };
            state.get_object_mut(id).expect("just created").card_types = vec![kind];
            if in_graveyard {
                if owner == PlayerId(0) {
                    state.players[0].graveyard_order.push(id);
                } else {
                    state.players[1].graveyard_order.push(id);
                }
                if owner == you && kind != CardType::Creature {
                    returnable.push(id);
                }
            }
            *slot = id;
        }
        let before = Snapshot::of(&state);
        registry.get(card).expect("registered").on_upkeep(&mut state, me, &[], &registry);
        if returnable.is_empty() {
            assert!(acted() == 0);
        } else {
            assert!(acted() == 2 && super::act(0) == Some(Act::Random(returnable.clone(), 1)));
            let Some(Act::Move(id, Zone::Hand)) = super::act(1) else { panic!("returns it to the hand") };
            assert!(returnable.contains(&id));
        }
        assert!(Snapshot::of(&state) == before);
        std::mem::forget((registry, state, returnable));
    }
}

card_proof! {
    /// Howlpack Alpha (Mayor of Avabruck transformed): its end step trigger
    /// is its controller's end step's alone, the front face having none;
    /// resolving, a 2/2 green Wolf creature token for whoever controlled it
    /// (CR 608.2h, last known information if it has gone). Nothing else.
    fn howlpack_alpha_makes_a_wolf_at_your_end_step() {
        let mayor = isd::mayor_of_avabruck::MayorOfAvabruck;
        assert!(mayor.step_trigger_scope(&TriggerKind::EndStep, true) == TriggerScope::Your);
        let front = std::mem::ManuallyDrop::new(mayor.card_data());
        assert!(!front.triggered_abilities.iter().any(|t| t.kind == TriggerKind::EndStep));
        let mut b = permanent(Box::new(isd::mayor_of_avabruck::MayorOfAvabruck));
        let before = Snapshot::of(&b.state);
        let card = b.card();
        b.registry.get(card).expect("registered").on_end_step(&mut b.state, b.me, &[], &b.registry);
        assert!(asked_for(&[Act::Tokens {
            n: 1, controller: b.you, power: 2, toughness: 2, colors: vec![Color::Green],
            types: vec![CardType::Creature], keywords: vec![], subtypes: vec!["Wolf".into()],
        }]));
        assert!(Snapshot::of(&b.state) == before);
        std::mem::forget(b);
    }
}
