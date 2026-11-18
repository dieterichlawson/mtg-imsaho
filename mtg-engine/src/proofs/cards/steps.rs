//! The permanents whose abilities trigger at the beginning of a step (CR
//! 503.1a, 513.1a): whose step, what they target, and what they do.
//!
//! Scope: proved here is what each card declares (its abilities, costs,
//! targets, triggers and static effects) and what its own code writes to
//! the game state (a question asked, an until-end-of-turn effect added).
//! What a resolution does through the engine, the moves, damage, draws
//! and destruction, is not proved here, and where the text below speaks
//! of one it is of that part alone: each card is played end to end with
//! the real engine by `tests/card_semantics.rs`.
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

use super::{any_object, board, card_proof, triggers, Form, Kinds};
use crate::actions::Target;
use crate::cards::{CardBehavior, CardRegistry, TargetFilter, TargetRequirement as R, TriggerKind, TriggerScope};
use crate::ids::{ObjectId, PlayerId};
use crate::state::{GameObject, GameState};
use crate::types::{CardType, Zone};

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

    fn get(&self, id: ObjectId) -> &GameObject {
        self.state.get_object(id).expect("on the board")
    }

    fn is(&self, id: ObjectId, subtype: &str) -> bool {
        self.get(id).subtypes.iter().any(|s| s == subtype)
    }
}

card_proof! {
    /// Reaper from the Abyss: triggers at each end step, if a creature died
    /// this turn (checked as it triggers and as it resolves, CR 603.4); its
    /// target a non-Demon creature on the battlefield. Resolving is the
    /// next proof's: the two in one harness ran out of memory at 7 GB.
    fn reaper_from_the_abyss_triggers_after_a_death_targeting_a_non_demon() {
        let reaper = isd::reaper_from_the_abyss::ReaperFromTheAbyss;
        // Matched as a pattern: a requirement holding a list of names,
        // compared with one built to compare, is two read from the heap
        // (`prints_effects` says why that is not done).
        let declared = std::mem::ManuallyDrop::new(triggers(&reaper));
        assert!(matches!(declared.as_slice(), [(TriggerKind::EndStep, Some(R::CreatureWithFilter(TargetFilter::NotSubtypes(names))))]
            if names.len() == 1 && names[0] == "Demon"));
        for back in [false, true] {
            assert!(reaper.step_trigger_scope(&TriggerKind::EndStep, back) == TriggerScope::Each);
        }
        let b = permanent(Box::new(isd::reaper_from_the_abyss::ReaperFromTheAbyss));
        let died = b.state.creature_died_this_turn;
        assert!(reaper.should_trigger(&b.state, b.me, &TriggerKind::EndStep, &b.registry) == died);
        let candidate = b.others[0];
        let o = b.get(candidate);
        let creature = o.zone == Zone::Battlefield && o.card_types.contains(&CardType::Creature);
        assert!(reaper.is_valid_target(&b.state, b.you, &Target::Object(candidate), &b.registry) == (creature && !b.is(candidate, "Demon")));
        std::mem::forget(b);
    }
}

