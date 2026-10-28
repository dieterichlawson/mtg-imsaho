//! The creatures whose power and toughness count something (CR 604.3,
//! 208.2a, 613.4a, 613.4c), and the abilities printed beside the count.
//!
//! - Boneyard Wurm, */*: "Boneyard Wurm's power and toughness are each
//!   equal to the number of creature cards in your graveyard."
//! - Splinterfright, */* Trample: "Splinterfright's power and toughness are
//!   each equal to the number of creature cards in your graveyard. At the
//!   beginning of your upkeep, mill two cards."
//! - Sturmgeist, */* Flying: "Sturmgeist's power and toughness are each
//!   equal to the number of cards in your hand. Whenever this creature
//!   deals combat damage to a player, draw a card."
//! - Geist-Honored Monk, */* Vigilance: "Geist-Honored Monk's power and
//!   toughness are each equal to the number of creatures you control. When
//!   this creature enters, create two 1/1 white Spirit creature tokens with
//!   flying."
//! - Scourge of Geier Reach, 3/3: "This creature gets +1/+1 for each
//!   creature your opponents control."
//! - Skaab Ruinator, 5/6 Flying: "As an additional cost to cast this spell,
//!   exile three creature cards from your graveyard. You may cast this card
//!   from your graveyard."
//!
//! - Wreath of Geists, Aura: "Enchanted creature gets +X/+X, where X is
//!   the number of creature cards in your graveyard."
//! - Runechanter's Pike, Equipment: "Equipped creature has first strike
//!   and gets +X/+0, where X is the number of instant and sorcery cards in
//!   your graveyard."
//!
//! A characteristic-defining ability works in every zone (CR 604.3), and
//! "you" is the creature's controller, which off the battlefield is its
//! owner (CR 108.4, and `move_object` sets it so). Each count is proved
//! over the creature on the battlefield under either player or in its
//! owner's graveyard, beside two cards, each either player's, a 1/1
//! creature or not a creature (and with no power and toughness, CR 208.3),
//! in a hand, a graveyard or on the battlefield, and a token on the
//! battlefield. Where the engine puts the count among the layers is
//! `proofs::actions`'.

use super::{acted, asked_for, board, card_proof, triggers, Act, Snapshot};
use crate::cards::{AdditionalCost, CardBehavior, CardRegistry, TriggerKind, TriggerScope};
use crate::ids::{CardId, ObjectId, PlayerId};
use crate::state::{GameObject, GameState};
use crate::types::{CardType, Color, Keyword, Zone};

use crate::cards::isd;

struct Board {
    registry: CardRegistry,
    state: GameState,
    me: ObjectId,
    others: [ObjectId; 3],
    /// The creature's controller: on the battlefield, either player; off
    /// it, its owner, player 0.
    you: PlayerId,
}

fn counted(card: Box<dyn CardBehavior>) -> Board {
    let (registry, mut state, blank, card) = board(card);
    let on: bool = kani::any();
    let you = if on { PlayerId(u8::from(kani::any::<bool>())) } else { PlayerId(0) };
    let me = state.create_object(card, PlayerId(0), if on { Zone::Battlefield } else { Zone::Graveyard }, None, None);
    state.get_object_mut(me).expect("just created").controller = you;
    if !on {
        state.players[0].graveyard_order.push(me);
    }
    let mut others = [ObjectId(0); 3];
    for slot in others.iter_mut().take(2) {
        let owner = PlayerId(u8::from(kani::any::<bool>()));
        let zone = match kani::any::<u8>() % 3 { 0 => Zone::Hand, 1 => Zone::Graveyard, _ => Zone::Battlefield };
        // A power and toughness for a creature only, as the game gives
        // them: the engine takes an object with a power for a creature
        // (`GameState::has_card_type`), and a board giving every card one
        // had Boneyard Wurm count a non-creature card, a state no game
        // reaches.
        let creature: bool = kani::any();
        let pt = creature.then_some(1);
        let id = state.create_object(blank, owner, zone, pt, pt);
        if zone == Zone::Graveyard {
            if owner == PlayerId(0) {
                state.players[0].graveyard_order.push(id);
            } else {
                state.players[1].graveyard_order.push(id);
            }
        }
        let o = state.get_object_mut(id).expect("just created");
        o.card_types = Vec::with_capacity(1);
        if creature {
            o.card_types.push(CardType::Creature);
        }
        *slot = id;
    }
    let token = state.create_object(CardId(0), PlayerId(u8::from(kani::any::<bool>())), Zone::Battlefield, Some(1), Some(1));
    let t = state.get_object_mut(token).expect("just created");
    t.is_token = true;
    t.card_types = vec![CardType::Creature];
    others[2] = token;
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

    fn count(&self, pred: impl Fn(&GameObject) -> bool) -> i32 {
        i32::try_from(self.others.iter().filter(|id| pred(self.get(**id))).count()).expect("small")
    }

    fn creature(o: &GameObject) -> bool {
        o.card_types.contains(&CardType::Creature)
    }

    /// The creature cards in `you`'s graveyard, the creature's own card
    /// among them if it is there.
    fn creature_cards_in_your_graveyard(&self) -> i32 {
        let mine = self.get(self.me).zone == Zone::Graveyard;
        i32::from(mine) + self.count(|o| o.zone == Zone::Graveyard && o.owner == self.you && !o.is_token && Self::creature(o))
    }
}

card_proof! {
    /// Boneyard Wurm: */* (CR 208.2a), each the creature cards in its
    /// controller's graveyard.
    fn boneyard_wurm_counts_the_creature_cards_in_your_graveyard() {
        let b = counted(Box::new(isd::boneyard_wurm::BoneyardWurm));
        assert!(b.behavior().prints_star_pt());
        let n = b.creature_cards_in_your_graveyard();
        assert!(b.behavior().dynamic_pt(&b.state, b.me, &b.registry) == Some((n, n)));
        std::mem::forget(b);
    }
}

card_proof! {
    /// Splinterfright: */*, each the creature cards in its controller's
    /// graveyard; at the beginning of its controller's upkeep, they mill
    /// two cards.
    fn splinterfright_counts_your_creature_cards_and_mills_two() {
        let fright = isd::splinterfright::Splinterfright;
        assert!(triggers(&fright) == [(TriggerKind::Upkeep, None)]);
        assert!(fright.step_trigger_scope(&TriggerKind::Upkeep, false) == TriggerScope::Your);
        let mut b = counted(Box::new(fright));
        assert!(b.behavior().prints_star_pt());
        let n = b.creature_cards_in_your_graveyard();
        assert!(b.behavior().dynamic_pt(&b.state, b.me, &b.registry) == Some((n, n)));
        let before = Snapshot::of(&b.state);
        let card = b.card();
        b.registry.get(card).expect("registered").on_upkeep(&mut b.state, b.me, &[], &b.registry);
        assert!(asked_for(&[Act::Mill(b.you, 2)]) && Snapshot::of(&b.state) == before);
        std::mem::forget(b);
    }
}

card_proof! {
    /// Sturmgeist: */*, each the cards in its controller's hand; when it
    /// deals combat damage to a player, its controller draws a card.
    fn sturmgeist_counts_your_hand_and_draws() {
        assert!(triggers(&isd::sturmgeist::Sturmgeist) == [(TriggerKind::CombatDamageToPlayer, None)]);
        let mut b = counted(Box::new(isd::sturmgeist::Sturmgeist));
        assert!(b.behavior().prints_star_pt());
        let n = b.count(|o| o.zone == Zone::Hand && o.owner == b.you);
        assert!(b.behavior().dynamic_pt(&b.state, b.me, &b.registry) == Some((n, n)));
        let before = Snapshot::of(&b.state);
        let card = b.card();
        b.registry.get(card).expect("registered").on_combat_damage_to_player(&mut b.state, b.me, PlayerId(1 - b.you.0), 2, &b.registry);
        assert!(asked_for(&[Act::Draw(b.you, 1)]) && Snapshot::of(&b.state) == before);
        std::mem::forget(b);
    }
}

card_proof! {
    /// Geist-Honored Monk: */*, each the creatures its controller controls,
    /// itself among them on the battlefield; as it enters, two 1/1 white
    /// Spirit creature tokens with flying for its controller.
    fn geist_honored_monk_counts_your_creatures_and_brings_two_spirits() {
        let monk = isd::geist_honored_monk::GeistHonoredMonk;
        assert!(monk.has_etb_handler() && triggers(&monk) == [(TriggerKind::EntersBattlefield, None)]);
        let mut b = counted(Box::new(monk));
        assert!(b.behavior().prints_star_pt());
        let on = b.get(b.me).zone == Zone::Battlefield;
        let n = i32::from(on) + b.count(|o| o.zone == Zone::Battlefield && o.controller == b.you && Board::creature(o));
        assert!(b.behavior().dynamic_pt(&b.state, b.me, &b.registry) == Some((n, n)));
        let before = Snapshot::of(&b.state);
        let card = b.card();
        b.registry.get(card).expect("registered").on_enter_battlefield(&mut b.state, b.me, &[], &b.registry);
        assert!(asked_for(&[Act::Tokens {
            n: 2, controller: b.you, power: 1, toughness: 1, colors: vec![Color::White],
            types: vec![CardType::Creature], keywords: vec![Keyword::Flying], subtypes: vec!["Spirit".into()],
        }]));
        assert!(Snapshot::of(&b.state) == before);
        std::mem::forget(b);
    }
}

card_proof! {
    /// Scourge of Geier Reach: +1/+1 for each creature on the battlefield
    /// an opponent of its controller controls (CR 613.4c), and no other
    /// ability.
    fn scourge_of_geier_reach_grows_with_your_opponents_creatures() {
        let b = counted(Box::new(isd::scourge_of_geier_reach::ScourgeOfGeierReach));
        assert!(!b.behavior().prints_star_pt());
        let n = b.count(|o| o.zone == Zone::Battlefield && o.controller != b.you && Board::creature(o));
        assert!(b.behavior().self_static_pt_mod(&b.state, b.me, &b.registry) == Some((n, n)));
        assert!(acted() == 0);
        std::mem::forget(b);
    }
}

card_proof! {
    /// Skaab Ruinator: its additional cost exiles three creature cards from
    /// its caster's graveyard (CR 601.2h), and it may be cast from a
    /// graveyard (CR 601.2a), which the engine pays and allows.
    fn skaab_ruinator_costs_three_creature_cards_and_casts_from_the_graveyard() {
        let ruinator = isd::skaab_ruinator::SkaabRuinator;
        assert!(ruinator.can_cast_from_graveyard());
        let data = std::mem::ManuallyDrop::new(ruinator.card_data());
        assert!(data.additional_cost == Some(AdditionalCost::ExileCreaturesFromGraveyard(3)));
    }
}

card_proof! {
    /// Wreath of Geists: its X, the number of creature cards (no token,
    /// CR 109.1) in its controller's graveyard, for both power and
    /// toughness (which creature gets it is the engine's: the one the Aura
    /// enchants, `proofs::actions`).
    fn wreath_of_geists_counts_your_creature_cards() {
        let b = counted(Box::new(isd::wreath_of_geists::WreathOfGeists));
        let n = b.count(|o| o.zone == Zone::Graveyard && o.owner == b.you && !o.is_token && Board::creature(o));
        assert!(b.behavior().dynamic_pt(&b.state, b.me, &b.registry) == Some((n, n)));
        assert!(acted() == 0);
        std::mem::forget(b);
    }
}

card_proof! {
    /// Runechanter's Pike: on the battlefield, its X, the number of instant
    /// and sorcery cards (no token) in its controller's graveyard, for
    /// power alone; off it, none. Over the two cards each an instant, a
    /// sorcery or a creature.
    fn runechanters_pike_counts_your_instant_and_sorcery_cards() {
        let mut b = counted(Box::new(isd::runechanters_pike::RunechantersPike));
        for id in &b.others[..2] {
            let kind = match kani::any::<u8>() % 3 { 0 => CardType::Instant, 1 => CardType::Sorcery, _ => CardType::Creature };
            let o = b.state.get_object_mut(*id).expect("on the board");
            o.card_types.clear();
            o.card_types.push(kind);
        }
        let n = b.count(|o| o.zone == Zone::Graveyard && o.owner == b.you && !o.is_token
            && (o.card_types.contains(&CardType::Instant) || o.card_types.contains(&CardType::Sorcery)));
        let on = b.get(b.me).zone == Zone::Battlefield;
        assert!(b.behavior().dynamic_pt(&b.state, b.me, &b.registry) == on.then_some((n, 0)));
        assert!(acted() == 0);
        std::mem::forget(b);
    }
}
