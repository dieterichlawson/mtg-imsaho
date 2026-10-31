//! The planeswalkers (CR 306): the loyalty abilities each has (CR 606), and
//! what each does as it resolves.
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

use super::{act, acted, any_object, any_target, asked_each, asked_for, board, card_proof, case_by_case, each_matches, no_ids_kept, Act, Form, Kinds, Snapshot};
use crate::actions::{ResolvedChoice, Target};
use crate::cards::{CardBehavior, CardRegistry, LoyaltyAbilityDef, TargetRequirement};
use crate::ids::{CardId, ObjectId, PlayerId};
use crate::state::{AwaitingAction, GameObject, GameState, PendingEffect, ResolutionChoiceKind, TemporaryEffect as T};
use crate::types::{CardType, Color, CounterType, Keyword, Zone};

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
    /// Garruk Relentless's state trigger (CR 603.8): it triggers while he
    /// is face up with two or fewer loyalty counters, and resolving, he
    /// transforms, unless he has already (the counters untouched: the
    /// card's ruling). Over loyalty from 0 to 4, either face up.
    fn garruk_relentless_transforms_at_two_or_fewer_loyalty() {
        let (registry, mut state, _, card) = board(Box::new(isd::garruk_relentless::GarrukRelentless));
        let me = state.create_object(card, PlayerId(0), Zone::Battlefield, None, None);
        let loyalty: u32 = kani::any();
        kani::assume(loyalty <= 4);
        let transformed: bool = kani::any();
        let o = state.get_object_mut(me).expect("just created");
        o.is_transformed = transformed;
        if loyalty > 0 {
            o.counters.insert(CounterType::Loyalty, loyalty);
        }
        let garruk = registry.get(card).expect("registered");
        assert!(garruk.state_trigger_condition(&state, me, &registry) == (!transformed && loyalty <= 2));
        let before = Snapshot::of(&state);
        garruk.on_state_trigger(&mut state, me, &registry);
        if transformed {
            assert!(acted() == 0);
        } else {
            assert!(asked_for(&[Act::Transform(me)]));
        }
        assert!(Snapshot::of(&state) == before);
        std::mem::forget((registry, state));
    }
}

card_proof! {
    /// Garruk Relentless's first 0: 3 damage from Garruk to his target
    /// creature, then damage from that creature to Garruk equal to its
    /// power (none for a power of 0 or less, CR 120.8), its power as it was
    /// before Garruk's damage, which changes no power; with his target
    /// gone, nothing (CR 608.2b). Nothing else.
    fn garruk_relentless_fights_his_target() {
        let mut b = planeswalker(Box::new(isd::garruk_relentless::GarrukRelentless));
        let target = any_target(b.others);
        let power = match target {
            Target::Object(id) => b.state.effective_power(id, &b.registry).unwrap_or(0),
            _ => 0,
        };
        let before = b.resolve(0, &[target.clone()]);
        match target {
            Target::Object(id) => {
                let deals = Act::Effect(Target::Object(id), PendingEffect::DealDamage { amount: 3, source_id: b.me });
                if power > 0 {
                    let back = Act::Effect(Target::Object(b.me), PendingEffect::DealDamage { amount: power.unsigned_abs(), source_id: id });
                    assert!(asked_for(&[deals, back]));
                } else {
                    assert!(asked_for(&[deals]));
                }
            }
            _ => {
                assert!(acted() == 0);
            }
        }
        assert!(Snapshot::of(&b.state) == before);
        std::mem::forget(b);
    }
}

/// Garruk Relentless's second 0, a 2/2 green Wolf creature token for the
/// player who activated it; Garruk, the Veil-Cursed's +1, a 1/1 black Wolf
/// creature token with deathtouch. Nothing else. Which ability, a constant
/// in each harness: the two in one ran out of memory.
fn garruk_makes_wolves(veil_cursed: bool) {
    let mut b = planeswalker(Box::new(isd::garruk_relentless::GarrukRelentless));
    let before = b.resolve(if veil_cursed { 10 } else { 1 }, &[]);
    let wolf = if veil_cursed {
        Act::Tokens {
            n: 1, controller: b.you, power: 1, toughness: 1, colors: vec![Color::Black],
            types: vec![CardType::Creature], keywords: vec![Keyword::Deathtouch], subtypes: vec!["Wolf".into()],
        }
    } else {
        Act::Tokens {
            n: 1, controller: b.you, power: 2, toughness: 2, colors: vec![Color::Green],
            types: vec![CardType::Creature], keywords: vec![], subtypes: vec!["Wolf".into()],
        }
    };
    assert!(asked_for(&[wolf]) && Snapshot::of(&b.state) == before);
    std::mem::forget(b);
}

case_by_case! { garruk_makes_wolves;
    garruk_relentless_makes_a_green_wolf: false,
    garruk_the_veil_cursed_makes_a_deathtouch_wolf: true,
}

card_proof! {
    /// Garruk, the Veil-Cursed's −1: the player who activated it
    /// sacrifices a creature they control (CR 701.21a: the one there is,
    /// or one they choose from theirs; it does not target, the card's
    /// ruling), and having done so, searches their library for a creature
    /// card to put into their hand, the search and its shuffle the
    /// engine's (`search_library`, every creature card in that library
    /// offered, in order, finding none allowed, CR 701.23b); with no
    /// creature to sacrifice, nothing. Over that player's library of a
    /// creature card and a land card, in either order, and the other's of a
    /// creature card.
    fn garruk_the_veil_cursed_sacrifices_then_searches() {
        let mut b = planeswalker(Box::new(isd::garruk_relentless::GarrukRelentless));
        let you = b.you;
        let mut library = [b.me; 3];
        for (i, slot) in library.iter_mut().enumerate() {
            let owner = if i < 2 { you } else { PlayerId(1 - you.0) };
            let id = b.state.create_object(CardId(1), owner, Zone::Library, None, None);
            b.state.get_object_mut(id).expect("just created").card_types = vec![if i == 1 { CardType::Land } else { CardType::Creature }];
            *slot = id;
        }
        let order = if kani::any() { [library[0], library[1]] } else { [library[1], library[0]] };
        if you == PlayerId(0) {
            b.state.players[0].library_order.extend_from_slice(&order);
            b.state.players[1].library_order.push(library[2]);
        } else {
            b.state.players[1].library_order.extend_from_slice(&order);
            b.state.players[0].library_order.push(library[2]);
        }
        let creatures = b.creatures_of(you);
        let before = b.resolve(11, &[]);
        let sacrificed = match creatures.len() {
            0 => {
                assert!(acted() == 0 && Snapshot::of(&b.state) == before);
                std::mem::forget(b);
                return;
            }
            1 => creatures[0],
            _ => {
                let asked = matches!(&b.state.awaiting_action, Some(AwaitingAction::ResolutionChoice {
                    player, source, choice: ResolutionChoiceKind::ChooseTarget {
                        options, optional: false, effect: PendingEffect::CardEffect { source_id, .. }, ..
                    },
                }) if *player == you && *source == b.me && *source_id == b.me
                    && options.len() == 2 && options[0] == Target::Object(creatures[0]) && options[1] == Target::Object(creatures[1]));
                assert!(asked && acted() == 0);
                crate::proofs::cards::clear_prompt(&mut b.state);
                let chosen = creatures[usize::from(kani::any::<bool>())];
                let card = b.card();
                b.registry.get(card).expect("registered").resolve_card_effect(&mut b.state, b.me, "", &Target::Object(chosen), &b.registry);
                chosen
            }
        };
        assert!(asked_for(&[
            Act::Sacrifice(sacrificed),
            Act::Search { source: b.me, player: you, candidates: vec![library[0]], to: Zone::Hand, tapped: false, optional: false },
        ]));
        assert!(Snapshot::of(&b.state) == before);
        std::mem::forget((b, creatures));
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

/// Liliana of the Veil's +1: each player still in the game discards a
/// card, the active player choosing first and then the other (CR
/// 101.4): a player with one card in hand discards it, a player with
/// more is asked which, and one with none discards nothing; and the
/// cards are discarded together once every player has chosen, in the
/// order chosen, and nothing is left kept for the next time. With
/// player `active` active, player `p` holding `sizes[p]` cards and lost
/// if `lost[p]`, each a constant in its harness (which card a player
/// asked chooses is the solver's): with the hands and who had lost the
/// solver's too, the queue of players kept between the questions had a
/// length it left open, and the proof ran out of memory in symbolic
/// execution.
fn liliana_makes_each_player_discard((active, sizes, lost): (u8, [usize; 2], [bool; 2])) {
    let (registry, mut state, blank, card) = board(Box::new(isd::liliana_of_the_veil::LilianaOfTheVeil));
    let me = state.create_object(card, PlayerId(0), Zone::Battlefield, None, None);
    state.active_player = PlayerId(active);
    let mut hands: [Vec<ObjectId>; 2] = [Vec::with_capacity(2), Vec::with_capacity(2)];
    for (p, hand) in hands.iter_mut().enumerate() {
        for _ in 0..sizes[p] {
            hand.push(state.create_object(blank, PlayerId(p as u8), Zone::Hand, None, None));
        }
        state.players[p].lost = lost[p];
    }
    let liliana = registry.get(card).expect("registered");
    let before = Snapshot::of(&state);
    liliana.on_loyalty_ability(&mut state, me, 0, &[], &registry);
    let mut chosen = Vec::with_capacity(2);
    for p in [PlayerId(active), PlayerId(1 - active)] {
        let hand = &hands[usize::from(p.0)];
        if lost[usize::from(p.0)] || hand.is_empty() {
            continue;
        }
        if hand.len() == 1 {
            chosen.push(hand[0]);
            continue;
        }
        let asked = matches!(&state.awaiting_action, Some(AwaitingAction::ResolutionChoice {
            player, source, choice: ResolutionChoiceKind::ChooseCardFromHand {
                player: from, cards, discard_immediately: false, remaining: 1, ..
            },
        }) if *player == p && *from == p && *source == me && cards == hand);
        assert!(asked && acted() == 0);
        crate::proofs::cards::clear_prompt(&mut state);
        let card = hand[usize::from(kani::any::<bool>())];
        liliana.on_discard_choice(&mut state, me, card, &registry);
        chosen.push(card);
    }
    assert!(state.awaiting_action.is_none());
    assert!(asked_each(chosen.len(), |i| Act::Discard(chosen[i])) && Snapshot::of(&state) == before);
    assert!(state.get_object(me).expect("on the board").card_state.is_empty() && no_ids_kept());
    std::mem::forget((registry, state, chosen, hands));
}

case_by_case! { liliana_makes_each_player_discard;
    liliana_of_the_veil_makes_each_player_discard_with_the_first_player_active_over_hands_of_no_and_no_neither_lost: (0, [0, 0], [false, false]),
    liliana_of_the_veil_makes_each_player_discard_with_the_first_player_active_over_hands_of_no_and_no_the_first_lost: (0, [0, 0], [true, false]),
    liliana_of_the_veil_makes_each_player_discard_with_the_first_player_active_over_hands_of_no_and_no_the_second_lost: (0, [0, 0], [false, true]),
    liliana_of_the_veil_makes_each_player_discard_with_the_first_player_active_over_hands_of_no_and_no_both_lost: (0, [0, 0], [true, true]),
    liliana_of_the_veil_makes_each_player_discard_with_the_first_player_active_over_hands_of_no_and_one_neither_lost: (0, [0, 1], [false, false]),
    liliana_of_the_veil_makes_each_player_discard_with_the_first_player_active_over_hands_of_no_and_one_the_first_lost: (0, [0, 1], [true, false]),
    liliana_of_the_veil_makes_each_player_discard_with_the_first_player_active_over_hands_of_no_and_one_the_second_lost: (0, [0, 1], [false, true]),
    liliana_of_the_veil_makes_each_player_discard_with_the_first_player_active_over_hands_of_no_and_one_both_lost: (0, [0, 1], [true, true]),
    liliana_of_the_veil_makes_each_player_discard_with_the_first_player_active_over_hands_of_no_and_two_neither_lost: (0, [0, 2], [false, false]),
    liliana_of_the_veil_makes_each_player_discard_with_the_first_player_active_over_hands_of_no_and_two_the_first_lost: (0, [0, 2], [true, false]),
    liliana_of_the_veil_makes_each_player_discard_with_the_first_player_active_over_hands_of_no_and_two_the_second_lost: (0, [0, 2], [false, true]),
    liliana_of_the_veil_makes_each_player_discard_with_the_first_player_active_over_hands_of_no_and_two_both_lost: (0, [0, 2], [true, true]),
    liliana_of_the_veil_makes_each_player_discard_with_the_first_player_active_over_hands_of_one_and_no_neither_lost: (0, [1, 0], [false, false]),
    liliana_of_the_veil_makes_each_player_discard_with_the_first_player_active_over_hands_of_one_and_no_the_first_lost: (0, [1, 0], [true, false]),
    liliana_of_the_veil_makes_each_player_discard_with_the_first_player_active_over_hands_of_one_and_no_the_second_lost: (0, [1, 0], [false, true]),
    liliana_of_the_veil_makes_each_player_discard_with_the_first_player_active_over_hands_of_one_and_no_both_lost: (0, [1, 0], [true, true]),
    liliana_of_the_veil_makes_each_player_discard_with_the_first_player_active_over_hands_of_one_and_one_neither_lost: (0, [1, 1], [false, false]),
    liliana_of_the_veil_makes_each_player_discard_with_the_first_player_active_over_hands_of_one_and_one_the_first_lost: (0, [1, 1], [true, false]),
    liliana_of_the_veil_makes_each_player_discard_with_the_first_player_active_over_hands_of_one_and_one_the_second_lost: (0, [1, 1], [false, true]),
    liliana_of_the_veil_makes_each_player_discard_with_the_first_player_active_over_hands_of_one_and_one_both_lost: (0, [1, 1], [true, true]),
    liliana_of_the_veil_makes_each_player_discard_with_the_first_player_active_over_hands_of_one_and_two_neither_lost: (0, [1, 2], [false, false]),
    liliana_of_the_veil_makes_each_player_discard_with_the_first_player_active_over_hands_of_one_and_two_the_first_lost: (0, [1, 2], [true, false]),
    liliana_of_the_veil_makes_each_player_discard_with_the_first_player_active_over_hands_of_one_and_two_the_second_lost: (0, [1, 2], [false, true]),
    liliana_of_the_veil_makes_each_player_discard_with_the_first_player_active_over_hands_of_one_and_two_both_lost: (0, [1, 2], [true, true]),
    liliana_of_the_veil_makes_each_player_discard_with_the_first_player_active_over_hands_of_two_and_no_neither_lost: (0, [2, 0], [false, false]),
    liliana_of_the_veil_makes_each_player_discard_with_the_first_player_active_over_hands_of_two_and_no_the_first_lost: (0, [2, 0], [true, false]),
    liliana_of_the_veil_makes_each_player_discard_with_the_first_player_active_over_hands_of_two_and_no_the_second_lost: (0, [2, 0], [false, true]),
    liliana_of_the_veil_makes_each_player_discard_with_the_first_player_active_over_hands_of_two_and_no_both_lost: (0, [2, 0], [true, true]),
    liliana_of_the_veil_makes_each_player_discard_with_the_first_player_active_over_hands_of_two_and_one_neither_lost: (0, [2, 1], [false, false]),
    liliana_of_the_veil_makes_each_player_discard_with_the_first_player_active_over_hands_of_two_and_one_the_first_lost: (0, [2, 1], [true, false]),
    liliana_of_the_veil_makes_each_player_discard_with_the_first_player_active_over_hands_of_two_and_one_the_second_lost: (0, [2, 1], [false, true]),
    liliana_of_the_veil_makes_each_player_discard_with_the_first_player_active_over_hands_of_two_and_one_both_lost: (0, [2, 1], [true, true]),
    liliana_of_the_veil_makes_each_player_discard_with_the_first_player_active_over_hands_of_two_and_two_neither_lost: (0, [2, 2], [false, false]),
    liliana_of_the_veil_makes_each_player_discard_with_the_first_player_active_over_hands_of_two_and_two_the_first_lost: (0, [2, 2], [true, false]),
    liliana_of_the_veil_makes_each_player_discard_with_the_first_player_active_over_hands_of_two_and_two_the_second_lost: (0, [2, 2], [false, true]),
    liliana_of_the_veil_makes_each_player_discard_with_the_first_player_active_over_hands_of_two_and_two_both_lost: (0, [2, 2], [true, true]),
    liliana_of_the_veil_makes_each_player_discard_with_the_second_player_active_over_hands_of_no_and_no_neither_lost: (1, [0, 0], [false, false]),
    liliana_of_the_veil_makes_each_player_discard_with_the_second_player_active_over_hands_of_no_and_no_the_first_lost: (1, [0, 0], [true, false]),
    liliana_of_the_veil_makes_each_player_discard_with_the_second_player_active_over_hands_of_no_and_no_the_second_lost: (1, [0, 0], [false, true]),
    liliana_of_the_veil_makes_each_player_discard_with_the_second_player_active_over_hands_of_no_and_no_both_lost: (1, [0, 0], [true, true]),
    liliana_of_the_veil_makes_each_player_discard_with_the_second_player_active_over_hands_of_no_and_one_neither_lost: (1, [0, 1], [false, false]),
    liliana_of_the_veil_makes_each_player_discard_with_the_second_player_active_over_hands_of_no_and_one_the_first_lost: (1, [0, 1], [true, false]),
    liliana_of_the_veil_makes_each_player_discard_with_the_second_player_active_over_hands_of_no_and_one_the_second_lost: (1, [0, 1], [false, true]),
    liliana_of_the_veil_makes_each_player_discard_with_the_second_player_active_over_hands_of_no_and_one_both_lost: (1, [0, 1], [true, true]),
    liliana_of_the_veil_makes_each_player_discard_with_the_second_player_active_over_hands_of_no_and_two_neither_lost: (1, [0, 2], [false, false]),
    liliana_of_the_veil_makes_each_player_discard_with_the_second_player_active_over_hands_of_no_and_two_the_first_lost: (1, [0, 2], [true, false]),
    liliana_of_the_veil_makes_each_player_discard_with_the_second_player_active_over_hands_of_no_and_two_the_second_lost: (1, [0, 2], [false, true]),
    liliana_of_the_veil_makes_each_player_discard_with_the_second_player_active_over_hands_of_no_and_two_both_lost: (1, [0, 2], [true, true]),
    liliana_of_the_veil_makes_each_player_discard_with_the_second_player_active_over_hands_of_one_and_no_neither_lost: (1, [1, 0], [false, false]),
    liliana_of_the_veil_makes_each_player_discard_with_the_second_player_active_over_hands_of_one_and_no_the_first_lost: (1, [1, 0], [true, false]),
    liliana_of_the_veil_makes_each_player_discard_with_the_second_player_active_over_hands_of_one_and_no_the_second_lost: (1, [1, 0], [false, true]),
    liliana_of_the_veil_makes_each_player_discard_with_the_second_player_active_over_hands_of_one_and_no_both_lost: (1, [1, 0], [true, true]),
    liliana_of_the_veil_makes_each_player_discard_with_the_second_player_active_over_hands_of_one_and_one_neither_lost: (1, [1, 1], [false, false]),
    liliana_of_the_veil_makes_each_player_discard_with_the_second_player_active_over_hands_of_one_and_one_the_first_lost: (1, [1, 1], [true, false]),
    liliana_of_the_veil_makes_each_player_discard_with_the_second_player_active_over_hands_of_one_and_one_the_second_lost: (1, [1, 1], [false, true]),
    liliana_of_the_veil_makes_each_player_discard_with_the_second_player_active_over_hands_of_one_and_one_both_lost: (1, [1, 1], [true, true]),
    liliana_of_the_veil_makes_each_player_discard_with_the_second_player_active_over_hands_of_one_and_two_neither_lost: (1, [1, 2], [false, false]),
    liliana_of_the_veil_makes_each_player_discard_with_the_second_player_active_over_hands_of_one_and_two_the_first_lost: (1, [1, 2], [true, false]),
    liliana_of_the_veil_makes_each_player_discard_with_the_second_player_active_over_hands_of_one_and_two_the_second_lost: (1, [1, 2], [false, true]),
    liliana_of_the_veil_makes_each_player_discard_with_the_second_player_active_over_hands_of_one_and_two_both_lost: (1, [1, 2], [true, true]),
    liliana_of_the_veil_makes_each_player_discard_with_the_second_player_active_over_hands_of_two_and_no_neither_lost: (1, [2, 0], [false, false]),
    liliana_of_the_veil_makes_each_player_discard_with_the_second_player_active_over_hands_of_two_and_no_the_first_lost: (1, [2, 0], [true, false]),
    liliana_of_the_veil_makes_each_player_discard_with_the_second_player_active_over_hands_of_two_and_no_the_second_lost: (1, [2, 0], [false, true]),
    liliana_of_the_veil_makes_each_player_discard_with_the_second_player_active_over_hands_of_two_and_no_both_lost: (1, [2, 0], [true, true]),
    liliana_of_the_veil_makes_each_player_discard_with_the_second_player_active_over_hands_of_two_and_one_neither_lost: (1, [2, 1], [false, false]),
    liliana_of_the_veil_makes_each_player_discard_with_the_second_player_active_over_hands_of_two_and_one_the_first_lost: (1, [2, 1], [true, false]),
    liliana_of_the_veil_makes_each_player_discard_with_the_second_player_active_over_hands_of_two_and_one_the_second_lost: (1, [2, 1], [false, true]),
    liliana_of_the_veil_makes_each_player_discard_with_the_second_player_active_over_hands_of_two_and_one_both_lost: (1, [2, 1], [true, true]),
    liliana_of_the_veil_makes_each_player_discard_with_the_second_player_active_over_hands_of_two_and_two_neither_lost: (1, [2, 2], [false, false]),
    liliana_of_the_veil_makes_each_player_discard_with_the_second_player_active_over_hands_of_two_and_two_the_first_lost: (1, [2, 2], [true, false]),
    liliana_of_the_veil_makes_each_player_discard_with_the_second_player_active_over_hands_of_two_and_two_the_second_lost: (1, [2, 2], [false, true]),
    liliana_of_the_veil_makes_each_player_discard_with_the_second_player_active_over_hands_of_two_and_two_both_lost: (1, [2, 2], [true, true]),
}

card_proof! {
    /// Liliana of the Veil's −2: her target player sacrifices a creature
    /// they control, chosen by them from every creature they control, and
    /// must (CR 701.21a); with none, nothing; her target gone, nothing.
    fn liliana_of_the_veil_makes_her_target_sacrifice_a_creature() {
        let mut b = planeswalker(Box::new(isd::liliana_of_the_veil::LilianaOfTheVeil));
        let target = any_target(b.others);
        let creatures = match target {
            Target::Player(p) => b.creatures_of(p),
            _ => Vec::new(),
        };
        let before = b.resolve(1, &[target.clone()]);
        match target {
            Target::Player(p) if !creatures.is_empty() => {
                let Some(Act::Choose { source, player, options, effect: PendingEffect::SacrificeCreature { source_name }, optional: false }) = act(0) else {
                    panic!("asks which creature")
                };
                assert!(acted() == 1 && *source == b.me && *player == p && source_name == "Liliana of the Veil");
                assert!(options.len() == creatures.len());
                for (o, c) in options.iter().zip(&creatures) {
                    assert!(*o == Target::Object(*c));
                }
            }
            _ => {
                assert!(acted() == 0);
            }
        }
        assert!(Snapshot::of(&b.state) == before);
        std::mem::forget((b, creatures));
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

/// The question [`liliana_divides`] leaves, which pile to sacrifice, over
/// the same board, asked of the target `p` by Liliana with the piles
/// `split` makes, the named pile first; answered with the first if
/// `first`, the engine's handler run. The target sacrifices every
/// permanent of that pile, and nothing else. The question is written
/// here as that proof finds it: the handler reads of it the player
/// asked, the source, and the two piles, which that proof checks, and
/// nothing else of it.
fn liliana_sacrifices_a_pile((p, split, first): (u8, &str, bool)) {
    let p = PlayerId(p);
    let (registry, mut state, me, permanents, _) = liliana_board(p, false);
    let (pile, rest) = piles(&permanents, split);
    state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
        player: p,
        source: me,
        choice: ResolutionChoiceKind::ChoosePile { description: String::new(), pile_1: pile.clone(), pile_2: rest.clone(), source_id: me },
    });
    let before = Snapshot::of(&state);
    let _ = crate::engine::actions::choices::resolve_choice(&mut state, &ResolvedChoice::ChosenIndex(usize::from(!first), String::new()), &registry);
    let sacrificed = if first { &pile } else { &rest };
    assert!(asked_each(sacrificed.len(), |i| Act::Sacrifice(sacrificed[i])) && Snapshot::of(&state) == Snapshot { asking: false, ..before.clone() });
    std::mem::forget((registry, state, permanents, pile, rest));
}

case_by_case! { liliana_sacrifices_a_pile;
    liliana_of_the_veil_against_her_controller_with_piles_000_sacrifices_the_first: (0, "000", true),
    liliana_of_the_veil_against_her_controller_with_piles_000_sacrifices_the_second: (0, "000", false),
    liliana_of_the_veil_against_her_controller_with_piles_001_sacrifices_the_first: (0, "001", true),
    liliana_of_the_veil_against_her_controller_with_piles_001_sacrifices_the_second: (0, "001", false),
    liliana_of_the_veil_against_her_controller_with_piles_010_sacrifices_the_first: (0, "010", true),
    liliana_of_the_veil_against_her_controller_with_piles_010_sacrifices_the_second: (0, "010", false),
    liliana_of_the_veil_against_her_controller_with_piles_011_sacrifices_the_first: (0, "011", true),
    liliana_of_the_veil_against_her_controller_with_piles_011_sacrifices_the_second: (0, "011", false),
    liliana_of_the_veil_against_her_controller_with_piles_100_sacrifices_the_first: (0, "100", true),
    liliana_of_the_veil_against_her_controller_with_piles_100_sacrifices_the_second: (0, "100", false),
    liliana_of_the_veil_against_her_controller_with_piles_101_sacrifices_the_first: (0, "101", true),
    liliana_of_the_veil_against_her_controller_with_piles_101_sacrifices_the_second: (0, "101", false),
    liliana_of_the_veil_against_her_controller_with_piles_110_sacrifices_the_first: (0, "110", true),
    liliana_of_the_veil_against_her_controller_with_piles_110_sacrifices_the_second: (0, "110", false),
    liliana_of_the_veil_against_her_controller_with_piles_111_sacrifices_the_first: (0, "111", true),
    liliana_of_the_veil_against_her_controller_with_piles_111_sacrifices_the_second: (0, "111", false),
    liliana_of_the_veil_against_the_other_player_with_piles_00_sacrifices_the_first: (1, "00", true),
    liliana_of_the_veil_against_the_other_player_with_piles_00_sacrifices_the_second: (1, "00", false),
    liliana_of_the_veil_against_the_other_player_with_piles_01_sacrifices_the_first: (1, "01", true),
    liliana_of_the_veil_against_the_other_player_with_piles_01_sacrifices_the_second: (1, "01", false),
    liliana_of_the_veil_against_the_other_player_with_piles_10_sacrifices_the_first: (1, "10", true),
    liliana_of_the_veil_against_the_other_player_with_piles_10_sacrifices_the_second: (1, "10", false),
    liliana_of_the_veil_against_the_other_player_with_piles_11_sacrifices_the_first: (1, "11", true),
    liliana_of_the_veil_against_the_other_player_with_piles_11_sacrifices_the_second: (1, "11", false),
}
