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

use super::{act, acted, any_object, any_target, asked_for, board, card_proof, each_is, Act, Form, Kinds, Snapshot};
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
/// its index, its loyalty change, and its target.
fn ability(def: &LoyaltyAbilityDef) -> (usize, i32, Option<TargetRequirement>) {
    (def.ability_index, def.loyalty_change, def.target_requirement.clone())
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
            assert!(defs.len() == 2 && ability(&defs[0]) == (0, 0, Some(TargetRequirement::Creature)) && ability(&defs[1]) == (1, 0, None));
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

card_proof! {
    /// Garruk Relentless's second 0, a 2/2 green Wolf creature token for
    /// the player who activated it; Garruk, the Veil-Cursed's +1, a 1/1
    /// black Wolf creature token with deathtouch. Nothing else.
    fn garruk_makes_wolves() {
        let mut b = planeswalker(Box::new(isd::garruk_relentless::GarrukRelentless));
        let veil_cursed: bool = kani::any();
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
                b.state.awaiting_action = None;
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
        assert!(each_is(added, 2 * creatures.len(), |i| if i % 2 == 0 {
            T::ModifyPT { target: creatures[i / 2], power_mod: x, toughness_mod: x }
        } else {
            T::GrantKeyword { target: creatures[i / 2], keyword: Keyword::Trample }
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
            && ability(&defs[1]) == (1, -2, Some(TargetRequirement::PlayerOnly))
            && ability(&defs[2]) == (2, -6, Some(TargetRequirement::PlayerOnly)));
        std::mem::forget((registry, state));
    }
}

card_proof! {
    /// Liliana of the Veil's +1: each player still in the game discards a
    /// card, the active player choosing first and then the other (CR
    /// 101.4): a player with one card in hand discards it, a player with
    /// more is asked which, and one with none discards nothing; and the
    /// cards are discarded together once every player has chosen, in the
    /// order chosen. Over each player's hand of up to two cards, either
    /// player active, and either having lost.
    fn liliana_of_the_veil_makes_each_player_discard() {
        let (registry, mut state, blank, card) = board(Box::new(isd::liliana_of_the_veil::LilianaOfTheVeil));
        let me = state.create_object(card, PlayerId(0), Zone::Battlefield, None, None);
        state.active_player = PlayerId(u8::from(kani::any::<bool>()));
        let mut hands = [[None; 2]; 2];
        for (p, hand) in hands.iter_mut().enumerate() {
            for slot in hand.iter_mut() {
                if kani::any() {
                    *slot = Some(state.create_object(blank, PlayerId(p as u8), Zone::Hand, None, None));
                }
            }
            state.players[p].lost = kani::any();
        }
        let liliana = registry.get(card).expect("registered");
        let before = Snapshot::of(&state);
        liliana.on_loyalty_ability(&mut state, me, 0, &[], &registry);
        let active = state.active_player;
        let mut chosen = Vec::with_capacity(2);
        for p in [active, PlayerId(1 - active.0)] {
            let hand: Vec<ObjectId> = hands[usize::from(p.0)].iter().flatten().copied().collect();
            if state.players[usize::from(p.0)].lost || hand.is_empty() {
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
            }) if *player == p && *from == p && *source == me && *cards == hand);
            assert!(asked && acted() == 0);
            state.awaiting_action = None;
            let card = hand[usize::from(kani::any::<bool>())];
            liliana.on_discard_choice(&mut state, me, card, &registry);
            chosen.push(card);
        }
        assert!(state.awaiting_action.is_none());
        let mut expected = Vec::with_capacity(2);
        for card in &chosen {
            expected.push(Act::Discard(*card));
        }
        assert!(asked_for(&expected) && Snapshot::of(&state) == before);
        assert!(state.get_object(me).expect("on the board").card_state.is_empty());
        std::mem::forget((registry, state, chosen, expected));
    }
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
                assert!(acted() == 1 && source == b.me && player == p && source_name == "Liliana of the Veil");
                assert!(options.len() == creatures.len());
                for (o, c) in options.iter().zip(&creatures) {
                    assert!(*o == Target::Object(*c));
                }
                std::mem::forget((options, source_name));
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
    /// Liliana of the Veil's −6: the player who activated her divides every
    /// permanent her target player controls into two piles (asked of them,
    /// with those permanents); with none, nothing; her target gone,
    /// nothing. Then, the engine's handler for the answer: a division
    /// naming anything not among those permanents is refused and the
    /// question stands; otherwise the target player is asked which pile to
    /// sacrifice, the one named and the rest, and sacrifices every
    /// permanent in the pile they choose.
    fn liliana_of_the_veil_makes_her_target_sacrifice_a_pile() {
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
        assert!(asked && acted() == 0);
        let mut pile = Vec::with_capacity(3);
        let mut rest = Vec::with_capacity(3);
        for id in &permanents {
            if kani::any() {
                pile.push(*id);
            } else {
                rest.push(*id);
            }
        }
        let stray = kani::any::<bool>() && !permanents.contains(&b.others[1]);
        if stray {
            pile.push(b.others[1]);
        }
        let _ = crate::engine::actions::choices::resolve_choice(&mut b.state, &ResolvedChoice::ChosenSubset(pile.clone()), &b.registry);
        if stray {
            let stands = matches!(&b.state.awaiting_action, Some(AwaitingAction::ResolutionChoice {
                choice: ResolutionChoiceKind::DividePermanentsIntoPiles { .. }, ..
            }));
            assert!(stands && acted() == 0);
            std::mem::forget((b, permanents, pile, rest));
            return;
        }
        let piles = matches!(&b.state.awaiting_action, Some(AwaitingAction::ResolutionChoice {
            player, source, choice: ResolutionChoiceKind::ChoosePile { pile_1, pile_2, .. },
        }) if *player == p && *source == b.me && *pile_1 == pile && *pile_2 == rest);
        assert!(piles && acted() == 0);
        let first: bool = kani::any();
        let _ = crate::engine::actions::choices::resolve_choice(&mut b.state, &ResolvedChoice::ChosenIndex(usize::from(!first), String::new()), &b.registry);
        let sacrificed = if first { &pile } else { &rest };
        let mut expected = Vec::with_capacity(3);
        for id in sacrificed {
            expected.push(Act::Sacrifice(*id));
        }
        assert!(asked_for(&expected) && b.state.awaiting_action.is_none());
        std::mem::forget((b, permanents, pile, rest, expected));
    }
}
