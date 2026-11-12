//! The permanents whose abilities trigger on a creature dying, with a
//! condition on which creature (CR 603.2, 603.10a).
//!
//! - Abattoir Ghoul, 3/2 First strike: "Whenever a creature dealt damage by
//!   this creature this turn dies, you gain life equal to that creature's
//!   toughness."
//! - Falkenrath Noble, 2/2 Flying: "Whenever this creature or another
//!   creature dies, target player loses 1 life and you gain 1 life."
//! - Lumberknot, 1/1 Hexproof: "Whenever a creature dies, put a +1/+1
//!   counter on this creature."
//! - Selhoff Occultist, 2/3: "Whenever this creature or another creature
//!   dies, target player mills a card."
//! - Unruly Mob, 1/1: "Whenever another creature you control dies, put a
//!   +1/+1 counter on this creature."
//! - Village Cannibals, 2/2: "Whenever another Human creature dies, put a
//!   +1/+1 counter on this creature."
//!
//! Each proof pins the triggers the card declares and the condition on the
//! dead creature the engine reads as it dies (from that creature's last
//! known information, CR 603.10a, 608.2h), over every such creature: either
//! player's, damaged by the watcher or not, of any toughness, a token or
//! not, a Human or not. Then it runs the card's code for the trigger
//! resolving, the watcher on the battlefield or gone since, under either
//! player's control.

use super::{acted, asked_for, board, card_proof, triggers, Act, Snapshot};
use crate::actions::Target;
use crate::cards::{CardBehavior, CardRegistry, TargetRequirement as R, TriggerKind};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::{CounterType, Zone};

use crate::cards::isd;

/// The watcher, owned by player 0, on the battlefield or gone to their
/// graveyard, controlled (or last controlled) by `you`; and the board.
struct Board {
    registry: CardRegistry,
    state: GameState,
    me: ObjectId,
    you: PlayerId,
}

fn watcher(card: Box<dyn CardBehavior>) -> Board {
    let (registry, mut state, _, card) = board(card);
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
    Board { registry, state, me, you }
}

/// A creature that died, as the engine remembers it: its id, its
/// controller, whether `watcher` had damaged it this turn, its toughness,
/// whether it was a token, and whether it was a Human.
struct Dead {
    id: ObjectId,
    controller: PlayerId,
    damaged_by_watcher: bool,
    toughness: i32,
    token: bool,
    human: bool,
}

fn any_dead() -> Dead {
    let toughness: i8 = kani::any();
    kani::assume((-2..=6).contains(&toughness));
    Dead {
        id: ObjectId(100),
        controller: PlayerId(u8::from(kani::any::<bool>())),
        damaged_by_watcher: kani::any(),
        toughness: i32::from(toughness),
        token: kani::any(),
        human: kani::any(),
    }
}

impl Board {
    fn card(&self) -> crate::ids::CardId {
        self.state.get_object(self.me).expect("on the board").card_id
    }

    /// Whether the card's death-watch triggers for `dead`.
    fn triggers_for(&self, dead: &Dead) -> bool {
        let damaged_by = if dead.damaged_by_watcher { vec![self.me] } else { vec![] };
        let subtypes = if dead.human { vec!["Human".to_string()] } else { vec![] };
        let card = self.card();
        let fires = self.registry.get(card).expect("registered").should_trigger_on_creature_dies(
            &self.state, self.me, dead.id, dead.controller, &damaged_by, dead.toughness, dead.token, &subtypes, &self.registry);
        std::mem::forget((damaged_by, subtypes));
        fires
    }

    /// The death-watch trigger resolving for `dead`, with `targets`.
    fn another_dies(&mut self, dead: &Dead, targets: &[Target]) -> Snapshot {
        let before = Snapshot::of(&self.state);
        let damaged_by = if dead.damaged_by_watcher { vec![self.me] } else { vec![] };
        let card = self.card();
        self.registry.get(card).expect("registered").on_any_creature_dies(
            &mut self.state, self.me, dead.id, dead.controller, &damaged_by, dead.toughness, dead.token, targets, &self.registry);
        std::mem::forget(damaged_by);
        before
    }

    /// The watcher's own "when this creature dies" trigger resolving.
    fn dies(&mut self, targets: &[Target]) -> Snapshot {
        let before = Snapshot::of(&self.state);
        let card = self.card();
        self.registry.get(card).expect("registered").on_dies(&mut self.state, self.me, targets, &self.registry);
        before
    }
}

/// Either player, or a target gone illegal.
fn any_player_target() -> Target {
    match kani::any::<u8>() % 3 {
        0 => Target::Player(PlayerId(0)),
        1 => Target::Player(PlayerId(1)),
        _ => Target::Illegal,
    }
}

card_proof! {
    /// Abattoir Ghoul: triggers for a creature it dealt damage to this
    /// turn, and no other; its controller gains life equal to that
    /// creature's toughness as it last existed, none for 0 or less.
    fn abattoir_ghoul_gains_the_toughness_of_what_it_damaged() {
        assert!(triggers(&isd::abattoir_ghoul::AbattoirGhoul) == [(TriggerKind::AnyCreatureDies, None)]);
        let mut b = watcher(Box::new(isd::abattoir_ghoul::AbattoirGhoul));
        let dead = any_dead();
        assert!(b.triggers_for(&dead) == dead.damaged_by_watcher);
        let before = b.another_dies(&dead, &[]);
        if dead.toughness > 0 {
            assert!(asked_for(&[Act::Life(b.you, dead.toughness)]));
        } else {
            assert!(acted() == 0);
        }
        assert!(Snapshot::of(&b.state) == before);
        std::mem::forget(b);
    }
}

/// Falkenrath Noble's drain: its target player loses 1 life, then its
/// controller gains 1 (CR 119.3); with its target gone illegal, nothing.
fn drained(you: PlayerId, target: &Target) -> bool {
    match target {
        Target::Player(p) => asked_for(&[Act::Life(*p, -1), Act::Life(you, 1)]),
        _ => acted() == 0,
    }
}

card_proof! {
    /// Falkenrath Noble: triggers for every other creature's death, and
    /// its own, each targeting a player; and drains.
    fn falkenrath_noble_drains_when_any_creature_dies() {
        let noble = isd::falkenrath_noble::FalkenrathNoble;
        assert!(triggers(&noble) == [(TriggerKind::SelfDies, Some(R::PlayerOnly)), (TriggerKind::AnyCreatureDies, Some(R::PlayerOnly))]);
        let mut b = watcher(Box::new(noble));
        let dead = any_dead();
        assert!(b.triggers_for(&dead));
        let target = any_player_target();
        let before = if kani::any() { b.another_dies(&dead, &[target.clone()]) } else { b.dies(&[target.clone()]) };
        assert!(drained(b.you, &target) && Snapshot::of(&b.state) == before);
        std::mem::forget(b);
    }
}

card_proof! {
    /// Selhoff Occultist: triggers for every other creature's death, and
    /// its own, each targeting a player, who mills a card; with its target
    /// gone illegal, nothing.
    fn selhoff_occultist_mills_its_target_one_when_any_creature_dies() {
        let occultist = isd::selhoff_occultist::SelhoffOccultist;
        assert!(triggers(&occultist) == [(TriggerKind::SelfDies, Some(R::PlayerOnly)), (TriggerKind::AnyCreatureDies, Some(R::PlayerOnly))]);
        let mut b = watcher(Box::new(occultist));
        let dead = any_dead();
        assert!(b.triggers_for(&dead));
        let target = any_player_target();
        let before = if kani::any() { b.another_dies(&dead, &[target.clone()]) } else { b.dies(&[target.clone()]) };
        match target {
            Target::Player(p) => {
                assert!(asked_for(&[Act::Mill(p, 1)]));
            }
            _ => {
                assert!(acted() == 0);
            }
        }
        assert!(Snapshot::of(&b.state) == before);
        std::mem::forget(b);
    }
}

/// The watcher's trigger puts a +1/+1 counter on it (CR 122.1); one gone
/// from the battlefield takes none, which `add_counters` decides.
fn grows(b: &Board, before: &Snapshot) -> bool {
    asked_for(&[Act::Counters(b.me, CounterType::PlusOnePlusOne, 1)]) && Snapshot::of(&b.state) == *before
}

card_proof! {
    /// Lumberknot: triggers for every creature's death, its own included
    /// (where it then takes no counter, being gone); and grows.
    fn lumberknot_grows_when_any_creature_dies() {
        let knot = isd::lumberknot::Lumberknot;
        assert!(triggers(&knot) == [(TriggerKind::SelfDies, None), (TriggerKind::AnyCreatureDies, None)]);
        let mut b = watcher(Box::new(knot));
        let dead = any_dead();
        assert!(b.triggers_for(&dead));
        let before = if kani::any() { b.another_dies(&dead, &[]) } else { b.dies(&[]) };
        assert!(grows(&b, &before));
        std::mem::forget(b);
    }
}

card_proof! {
    /// Unruly Mob: triggers for a creature its controller controlled as it
    /// died, and no other; and grows.
    fn unruly_mob_grows_when_another_creature_you_control_dies() {
        assert!(triggers(&isd::unruly_mob::UnrulyMob) == [(TriggerKind::AnyCreatureDies, None)]);
        let mut b = watcher(Box::new(isd::unruly_mob::UnrulyMob));
        let dead = any_dead();
        assert!(b.triggers_for(&dead) == (dead.controller == b.you));
        let before = b.another_dies(&dead, &[]);
        assert!(grows(&b, &before));
        std::mem::forget(b);
    }
}

card_proof! {
    /// Village Cannibals: triggers for a creature that was a Human as it
    /// died, and no other; and grows.
    fn village_cannibals_grows_when_another_human_dies() {
        assert!(triggers(&isd::village_cannibals::VillageCannibals) == [(TriggerKind::AnyCreatureDies, None)]);
        let mut b = watcher(Box::new(isd::village_cannibals::VillageCannibals));
        let dead = any_dead();
        assert!(b.triggers_for(&dead) == dead.human);
        let before = b.another_dies(&dead, &[]);
        assert!(grows(&b, &before));
        std::mem::forget(b);
    }
}

card_proof! { adding_counters
    /// Gutter Grime: triggers for a nontoken creature its controller
    /// controlled as it died, and no other death; a slime counter on it
    /// (none if it has left, which `add_counters` decides), then a green
    /// Ooze creature token for its controller whose power and toughness are
    /// each the slime counters on Gutter Grime (CR 604.3), whatever that
    /// count is by then.
    fn gutter_grime_counts_slime_and_makes_an_ooze() {
        assert!(triggers(&isd::gutter_grime::GutterGrime) == [(TriggerKind::AnyCreatureDies, None)]);
        let mut b = watcher(Box::new(isd::gutter_grime::GutterGrime));
        let dead = any_dead();
        assert!(b.triggers_for(&dead) == (!dead.token && dead.controller == b.you));
        let slime: u32 = kani::any();
        kani::assume(slime <= 3);
        if slime > 0 {
            b.state.get_object_mut(b.me).expect("on the board").counters.insert(CounterType::Slime, slime);
        }
        let on = b.state.get_object(b.me).expect("on the board").zone == Zone::Battlefield;
        b.another_dies(&dead, &[]);
        assert!(asked_for(&[
            Act::Counters(b.me, CounterType::Slime, 1),
            Act::DefinedToken {
                defined_by: b.me, controller: b.you, colors: vec![crate::types::Color::Green],
                types: vec![crate::types::CardType::Creature], subtypes: vec!["Ooze".into()],
            },
        ]));
        let card = b.card();
        let grime = b.registry.get(card).expect("registered");
        let now = i32::try_from(slime + u32::from(on)).expect("small");
        assert!(grime.token_dynamic_pt(&b.state, b.me, ObjectId(100), &b.registry) == Some((now, now)));
        std::mem::forget(b);
    }
}
