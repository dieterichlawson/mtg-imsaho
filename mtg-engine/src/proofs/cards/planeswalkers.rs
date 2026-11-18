//! The planeswalkers (CR 306): the loyalty abilities each has (CR 606), and
//! what each does as it resolves.
//!
//! Scope: proved here is what each card declares (its abilities, costs,
//! targets, triggers and static effects) and what its own code writes to
//! the game state (a question asked, an until-end-of-turn effect added).
//! What a resolution does through the engine, the moves, damage, draws
//! and destruction, is not proved here, and where the text below speaks
//! of one it is of that part alone: each card is played end to end with
//! the real engine by `tests/card_semantics.rs`.
//!
//! - Garruk Relentless, loyalty 3: "When Garruk Relentless has two or fewer
//!   loyalty counters on him, transform him. 0: Garruk Relentless deals 3
//!   damage to target creature. That creature deals damage equal to its
//!   power to him. 0: Create a 2/2 green Wolf creature token." Transformed,
//!   Garruk, the Veil-Cursed: "+1: Create a 1/1 black Wolf creature token
//!   with deathtouch. −1: Sacrifice a creature. If you do, search your
//!   library for a creature card, reveal it, put it into your hand, then
//!   shuffle. −3: Creatures you control gain trample and get +X/+X until
//!   end of turn, where X is the number of creature cards in your
//!   graveyard."
//! - Liliana of the Veil, loyalty 3: "+1: Each player discards a card. −2:
//!   Target player sacrifices a creature. −6: Separate all permanents
//!   target player controls into two piles. That player sacrifices all
//!   permanents in the pile of their choice."
//!
//! A loyalty ability's cost, its loyalty change, is paid as it is
//! activated (CR 606.4), by the engine; it may leave the planeswalker with
//! no loyalty, and dead before the ability resolves, which it does all
//! the same (CR 113.7a). Each resolution here is over the planeswalker on
//! the battlefield or gone, under either player's control, activated by
//! either player ("you" is the activator, CR 602.2a), beside a card and a
//! token of any characteristics.

use super::{acted, any_object, any_target, board, card_proof, case_by_case, each_matches, Form, Kinds, Snapshot};
use crate::actions::{ResolvedChoice, Target};
use crate::cards::{CardBehavior, CardRegistry, LoyaltyAbilityDef, TargetRequirement};
use crate::ids::{CardId, ObjectId, PlayerId};
use crate::state::{AwaitingAction, GameObject, GameState, ResolutionChoiceKind, TemporaryEffect as T};
use crate::types::{CardType, Keyword, Zone};

use crate::cards::isd;

const ANY_CREATURE: Kinds = Kinds { types: &[CardType::Creature, CardType::Enchantment], subtypes: &[], keywords: &[] };

/// The board: the planeswalker, owned by player 0, on the battlefield or
/// gone to their graveyard, controlled (or last controlled) by either
/// player, its ability activated by `you`, either player; a card and a
/// token beside it.
struct Board {
    registry: CardRegistry,
    state: GameState,
    me: ObjectId,
    others: [ObjectId; 2],
    you: PlayerId,
}

fn planeswalker(card: Box<dyn CardBehavior>) -> Board {
    let (registry, mut state, blank, card) = board(card);
    let controller = PlayerId(u8::from(kani::any::<bool>()));
    let gone: bool = kani::any();
    let me = state.create_object(card, PlayerId(0), if gone { Zone::Graveyard } else { Zone::Battlefield }, None, None);
    let o = state.get_object_mut(me).expect("just created");
    if gone {
        o.last_controller = Some(controller);
        state.players[0].graveyard_order.push(me);
    } else {
        o.controller = controller;
    }
    let others = [any_object(&mut state, blank, &ANY_CREATURE, Form::Card), any_object(&mut state, blank, &ANY_CREATURE, Form::Token)];
    let you = PlayerId(u8::from(kani::any::<bool>()));
    state.resolving_ability_activator = Some(you);
    Board { registry, state, me, others, you }
}

impl Board {
    fn card(&self) -> CardId {
        self.state.get_object(self.me).expect("on the board").card_id
    }

    fn get(&self, id: ObjectId) -> &GameObject {
        self.state.get_object(id).expect("on the board")
    }

    /// Of the card and the token, the creatures on the battlefield `p`
    /// controls, in id order.
    fn creatures_of(&self, p: PlayerId) -> Vec<ObjectId> {
        let mut ids = Vec::with_capacity(2);
        for id in self.others {
            let o = self.get(id);
            if o.zone == Zone::Battlefield && o.controller == p && o.card_types.contains(&CardType::Creature) {
                ids.push(id);
            }
        }
        ids
    }

    /// Ability `index` resolving with `targets`; the board before.
    fn resolve(&mut self, index: usize, targets: &[Target]) -> Snapshot {
        let before = Snapshot::of(&self.state);
        let card = self.card();
        self.registry.get(card).expect("registered").on_loyalty_ability(&mut self.state, self.me, index, targets, &self.registry);
        before
    }
}

/// A loyalty ability as the engine reads it to offer it and pay for it:
/// its index, its loyalty change, and its target. The target is borrowed,
/// not cloned: a clone of a target requirement read from the heap went
/// into every variant of every filter it might hold, and Garruk
/// Relentless's proof of his abilities ran out of memory doing it.
fn ability(def: &LoyaltyAbilityDef) -> (usize, i32, Option<&TargetRequirement>) {
    (def.ability_index, def.loyalty_change, def.target_requirement.as_ref())
}

card_proof! {
    /// Garruk Relentless: three loyalty (CR 306.5b); face up, his two 0
    /// abilities, the first targeting a creature; transformed, Garruk, the
    /// Veil-Cursed's +1, −1 and −3, none targeting, at indices of their own
    /// (a face's abilities are its own, CR 712.8d).
    fn garruk_relentless_has_his_abilities() {
        let garruk = isd::garruk_relentless::GarrukRelentless;
        assert!(garruk.starting_loyalty() == Some(3));
        let (registry, mut state, _, card) = board(Box::new(isd::garruk_relentless::GarrukRelentless));
        let me = state.create_object(card, PlayerId(0), Zone::Battlefield, None, None);
        let transformed: bool = kani::any();
        state.get_object_mut(me).expect("just created").is_transformed = transformed;
        let defs = std::mem::ManuallyDrop::new(garruk.loyalty_abilities(&state, me));
        if transformed {
            assert!(defs.len() == 3 && ability(&defs[0]) == (10, 1, None) && ability(&defs[1]) == (11, -1, None) && ability(&defs[2]) == (12, -3, None));
        } else {
            assert!(defs.len() == 2 && ability(&defs[0]) == (0, 0, Some(&TargetRequirement::Creature)) && ability(&defs[1]) == (1, 0, None));
        }
        std::mem::forget((registry, state));
    }
}

card_proof! {
    /// Garruk, the Veil-Cursed's −3: each creature the player who
    /// activated it controls as it resolves (CR 611.2c) gets +X/+X and
    /// trample until end of turn, X the number of creature cards (no
    /// token, CR 109.1) in that player's graveyard then. Nothing else.
    fn garruk_the_veil_cursed_overruns() {
        let mut b = planeswalker(Box::new(isd::garruk_relentless::GarrukRelentless));
        let you = b.you;
        let mut x = 0;
        for id in [b.me, b.others[0], b.others[1]] {
            let o = b.get(id);
            if o.zone == Zone::Graveyard && o.owner == you && !o.is_token && o.card_types.contains(&CardType::Creature) {
                x += 1;
            }
        }
        let creatures = b.creatures_of(you);
        let before = b.resolve(12, &[]);
        let added = &b.state.until_end_of_turn[before.until_end_of_turn()..];
        // Matched by kind, not compared whole: an effect pushed where the
        // solver leaves open how many came before is, to symbolic
        // execution, of any kind, and `==` walked every filter a temporary
        // effect could hold (`each_matches` says more).
        assert!(each_matches(added, 2 * creatures.len(), |i, t| if i % 2 == 0 {
            matches!(t, T::ModifyPT { target, power_mod, toughness_mod } if *target == creatures[i / 2] && *power_mod == x && *toughness_mod == x)
        } else {
            matches!(t, T::GrantKeyword { target, keyword: Keyword::Trample } if *target == creatures[i / 2])
        }));
        assert!(acted() == 0 && Snapshot::of(&b.state).without_until_end_of_turn() == before.without_until_end_of_turn());
        std::mem::forget((b, creatures));
    }
}

card_proof! {
    /// Liliana of the Veil: three loyalty; +1 untargeted, −2 and −6 each
    /// targeting a player.
    fn liliana_of_the_veil_has_her_abilities() {
        let liliana = isd::liliana_of_the_veil::LilianaOfTheVeil;
        assert!(liliana.starting_loyalty() == Some(3));
        let (registry, mut state, _, card) = board(Box::new(isd::liliana_of_the_veil::LilianaOfTheVeil));
        let me = state.create_object(card, PlayerId(0), Zone::Battlefield, None, None);
        let defs = std::mem::ManuallyDrop::new(liliana.loyalty_abilities(&state, me));
        assert!(defs.len() == 3 && ability(&defs[0]) == (0, 1, None)
            && ability(&defs[1]) == (1, -2, Some(&TargetRequirement::PlayerOnly))
            && ability(&defs[2]) == (2, -6, Some(&TargetRequirement::PlayerOnly)));
        std::mem::forget((registry, state));
    }
}

card_proof! {
    /// Liliana of the Veil's −6: the player who activated her is asked to
    /// divide every permanent her target player controls into two piles,
    /// with those permanents; with none, nothing; her target gone,
    /// nothing. What the engine's handler does with the answer is
    /// [`liliana_divides`]'s.
    fn liliana_of_the_veil_asks_for_piles_of_her_targets_permanents() {
        let mut b = planeswalker(Box::new(isd::liliana_of_the_veil::LilianaOfTheVeil));
        let target = any_target(b.others);
        let mut permanents = Vec::with_capacity(3);
        if let Target::Player(p) = target {
            for id in [b.me, b.others[0], b.others[1]] {
                let o = b.get(id);
                if o.zone == Zone::Battlefield && o.controller == p {
                    permanents.push(id);
                }
            }
        }
        let before = b.resolve(2, &[target.clone()]);
        let Target::Player(p) = target else {
            assert!(acted() == 0 && Snapshot::of(&b.state) == before);
            std::mem::forget(b);
            return;
        };
        if permanents.is_empty() {
            assert!(acted() == 0 && Snapshot::of(&b.state) == before);
            std::mem::forget((b, permanents));
            return;
        }
        let asked = matches!(&b.state.awaiting_action, Some(AwaitingAction::ResolutionChoice {
            player, source, choice: ResolutionChoiceKind::DividePermanentsIntoPiles { permanents: offered, target_player, source_id, .. },
        }) if *player == b.you && *source == b.me && *source_id == b.me && *target_player == p && *offered == permanents);
        assert!(asked && acted() == 0 && Snapshot::of(&b.state) == Snapshot { asking: true, ..before.clone() });
        std::mem::forget((b, permanents));
    }
}

/// The board of Liliana of the Veil's −6 answered: the registry, the
/// game, Liliana (on the battlefield under player 0, who activated her),
/// her target `p`'s permanents in id order (two of `p`'s beside her, and
/// Liliana herself if `p` is player 0), and, if `stray`, a card in exile.
fn liliana_board(p: PlayerId, stray: bool) -> (CardRegistry, GameState, ObjectId, Vec<ObjectId>, Option<ObjectId>) {
    let (registry, mut state, blank, card) = board(Box::new(isd::liliana_of_the_veil::LilianaOfTheVeil));
    let you = PlayerId(0);
    let me = state.create_object(card, you, Zone::Battlefield, None, None);
    let others = [state.create_object(blank, p, Zone::Battlefield, None, None), state.create_object(blank, p, Zone::Battlefield, None, None)];
    let exiled = stray.then(|| state.create_object(blank, you, Zone::Exile, None, None));
    state.resolving_ability_activator = Some(you);
    let permanents = if p == you { vec![me, others[0], others[1]] } else { vec![others[0], others[1]] };
    (registry, state, me, permanents, exiled)
}

/// `permanents` as `split` divides them (its `k`th digit, 1 for the first
/// pile, for the `k`th of them): the first pile and the second.
fn piles(permanents: &[ObjectId], split: &str) -> (Vec<ObjectId>, Vec<ObjectId>) {
    assert!(split.len() == permanents.len());
    let mut pile = Vec::with_capacity(4);
    let mut rest = Vec::with_capacity(3);
    for (id, digit) in permanents.iter().zip(split.bytes()) {
        if digit == b'1' { pile.push(*id) } else { rest.push(*id) }
    }
    (pile, rest)
}

/// Liliana of the Veil's −6 asking for piles, over [`liliana_board`],
/// answered with the division `split` says ([`piles`]), and the exiled
/// card put in the first pile too if `stray`; the engine's handler for
/// the answer run with the card. A division naming anything not among
/// the permanents is refused and the question stands; otherwise the
/// target is asked which pile to sacrifice, the one named and the rest,
/// and nothing else. What the handler does with that answer is
/// [`liliana_sacrifices_a_pile`]'s. Each a constant in its harness: with
/// the board, the division and the choice the solver's, the proof of the
/// whole ran out of memory in symbolic execution before it reached the
/// handler, and answering both questions in one harness, with the piles
/// the first answer left read back for the second, did too.
fn liliana_divides((p, split, stray): (u8, &str, bool)) {
    let p = PlayerId(p);
    let (registry, mut state, me, permanents, exiled) = liliana_board(p, stray);
    let before = Snapshot::of(&state);
    let card = state.get_object(me).expect("on the board").card_id;
    registry.get(card).expect("registered").on_loyalty_ability(&mut state, me, 2, &[Target::Player(p)], &registry);
    let asked = matches!(&state.awaiting_action, Some(AwaitingAction::ResolutionChoice {
        player, source, choice: ResolutionChoiceKind::DividePermanentsIntoPiles { permanents: offered, target_player, .. },
    }) if *player == PlayerId(0) && *source == me && *target_player == p && *offered == permanents);
    assert!(asked && acted() == 0);
    let (mut pile, rest) = piles(&permanents, split);
    if let Some(exiled) = exiled {
        pile.push(exiled);
    }
    let _ = crate::engine::actions::choices::resolve_choice(&mut state, &ResolvedChoice::ChosenSubset(pile.clone()), &registry);
    let asks = if stray {
        matches!(&state.awaiting_action, Some(AwaitingAction::ResolutionChoice {
            choice: ResolutionChoiceKind::DividePermanentsIntoPiles { .. }, ..
        }))
    } else {
        matches!(&state.awaiting_action, Some(AwaitingAction::ResolutionChoice {
            player, source, choice: ResolutionChoiceKind::ChoosePile { pile_1, pile_2, .. },
        }) if *player == p && *source == me && *pile_1 == pile && *pile_2 == rest)
    };
    assert!(asks && acted() == 0 && Snapshot::of(&state) == Snapshot { asking: true, ..before.clone() });
    std::mem::forget((registry, state, permanents, pile, rest));
}

case_by_case! { liliana_divides;
    liliana_of_the_veil_against_her_controller_refuses_a_division_naming_another_card: (0, "101", true),
    liliana_of_the_veil_against_the_other_player_refuses_a_division_naming_another_card: (1, "10", true),
    liliana_of_the_veil_against_her_controller_divided_000_asks_which_pile: (0, "000", false),
    liliana_of_the_veil_against_her_controller_divided_001_asks_which_pile: (0, "001", false),
    liliana_of_the_veil_against_her_controller_divided_010_asks_which_pile: (0, "010", false),
    liliana_of_the_veil_against_her_controller_divided_011_asks_which_pile: (0, "011", false),
    liliana_of_the_veil_against_her_controller_divided_100_asks_which_pile: (0, "100", false),
    liliana_of_the_veil_against_her_controller_divided_101_asks_which_pile: (0, "101", false),
    liliana_of_the_veil_against_her_controller_divided_110_asks_which_pile: (0, "110", false),
    liliana_of_the_veil_against_her_controller_divided_111_asks_which_pile: (0, "111", false),
    liliana_of_the_veil_against_the_other_player_divided_00_asks_which_pile: (1, "00", false),
    liliana_of_the_veil_against_the_other_player_divided_01_asks_which_pile: (1, "01", false),
    liliana_of_the_veil_against_the_other_player_divided_10_asks_which_pile: (1, "10", false),
    liliana_of_the_veil_against_the_other_player_divided_11_asks_which_pile: (1, "11", false),
}

