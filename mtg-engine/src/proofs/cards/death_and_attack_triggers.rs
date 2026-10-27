//! The creatures whose abilities trigger on a creature dying (CR 700.4,
//! 603.6c, 603.10a) or on attacking (CR 508.3a), and the statics printed
//! beside them.
//!
//! - Doomed Traveler, 1/1: "When this creature dies, create a 1/1 white
//!   Spirit creature token with flying."
//! - Mausoleum Guard, 2/1: "When this creature dies, create two 1/1 white
//!   Spirit creature tokens with flying."
//! - Pitchburn Devils, 3/3: "When this creature dies, it deals 3 damage to
//!   any target."
//! - Moldgraf Monstrosity, 8/8 Trample: "When this creature dies, exile it,
//!   then return two creature cards at random from your graveyard to the
//!   battlefield."
//! - Elder Cathar, 2/2 Human Soldier: "When this creature dies, put a +1/+1
//!   counter on target creature you control. If that creature is a Human,
//!   put two +1/+1 counters on it instead."
//! - Bitterheart Witch, 1/2 Deathtouch: "When this creature dies, you may
//!   search your library for a Curse card, put it onto the battlefield
//!   attached to target player, then shuffle."
//! - Galvanic Juggernaut, 5/5: "This creature attacks each combat if able.
//!   This creature doesn't untap during your untap step. Whenever another
//!   creature dies, untap this creature."
//! - Rage Thrower, 3/2: "Whenever another creature dies, this creature
//!   deals 2 damage to target player or planeswalker."
//! - Geist of Saint Traft, 2/2 Hexproof: "Whenever Geist of Saint Traft
//!   attacks, create a 4/4 white Angel creature token with flying that's
//!   tapped and attacking. Exile that token at end of combat."
//! - Grimgrin, Corpse-Born, 5/5: "Grimgrin enters tapped and doesn't untap
//!   during your untap step. Sacrifice another creature: Untap Grimgrin and
//!   put a +1/+1 counter on it. Whenever Grimgrin attacks, destroy target
//!   creature defending player controls, then put a +1/+1 counter on
//!   Grimgrin." (Its entering tapped is `proofs::cards::replacements`', and
//!   its ability `proofs::cards::activated`'.)
//! - Kessig Cagebreakers, 3/4: "Whenever this creature attacks, create a
//!   2/2 green Wolf creature token that's tapped and attacking for each
//!   creature card in your graveyard."
//!
//! Each proof pins the trigger the card declares and runs the card's code
//! for it resolving. A "when this creature dies" trigger resolves with the
//! creature in its owner's graveyard or moved on from there by then, and
//! is controlled by whoever controlled the creature as it died (CR 603.3a,
//! 608.2h), either player here; the others resolve with their creature on
//! the battlefield or gone, under either player's control. Each is beside a
//! card and a token of any characteristics. "Another creature" is the
//! engine's to decide as the ability triggers (a creature's own death does
//! not trigger its watcher, `triggers::collect`), proved in
//! `proofs::actions`.

use super::{act, acted, any_object, any_target, asked_for, board, card_proof, listed, new_ids, prints_effects, triggers, Act, Form, Kinds, Snapshot};
use crate::actions::Target;
use crate::cards::{AttackInfo, CardBehavior, CardRegistry, TargetRequirement, TriggerKind};
use crate::ids::{ObjectId, PlayerId};
use crate::state::{AwaitingAction, GameObject, GameState, PendingEffect, ResolutionChoiceKind};
use crate::types::{CardType, Color, ContinuousEffect as E, EffectScope, Keyword, Zone};

use crate::cards::isd;

const ANY_CREATURE: Kinds = Kinds { types: &[CardType::Creature, CardType::Artifact], subtypes: &[], keywords: &[] };

/// The board: the card's creature, owned by player 0, in `zone` (or, if
/// `zone` is the graveyard, perhaps moved on to exile since), last
/// controlled by `you`, either player; a card and a token beside it; and
/// the board before its trigger resolves.
struct Board {
    registry: CardRegistry,
    state: GameState,
    me: ObjectId,
    others: [ObjectId; 2],
    you: PlayerId,
}

fn creature_in(card: Box<dyn CardBehavior>, zone: Zone) -> Board {
    creature_among(card, zone, &ANY_CREATURE)
}

/// [`creature_in`], beside a card and a token of `kinds`.
fn creature_among(card: Box<dyn CardBehavior>, zone: Zone, kinds: &Kinds) -> Board {
    let (registry, mut state, blank, card) = board(card);
    let you = PlayerId(u8::from(kani::any::<bool>()));
    let zone = if zone == Zone::Graveyard && kani::any() { Zone::Exile } else { zone };
    let me = state.create_object(card, PlayerId(0), zone, None, None);
    if zone == Zone::Graveyard {
        state.players[0].graveyard_order.push(me);
    }
    let o = state.get_object_mut(me).expect("just created");
    if zone == Zone::Battlefield {
        // Controlled by either player, as a creature can be that its
        // owner does not control.
        o.controller = you;
    } else {
        o.last_controller = Some(you);
    }
    let others = [any_object(&mut state, blank, kinds, Form::Card), any_object(&mut state, blank, kinds, Form::Token)];
    Board { registry, state, me, others, you }
}

impl Board {
    fn card(&self) -> crate::ids::CardId {
        self.state.get_object(self.me).expect("on the board").card_id
    }

    fn those(&self, pred: impl Fn(&GameObject) -> bool) -> Vec<ObjectId> {
        self.others.iter().copied().filter(|id| pred(self.state.get_object(*id).expect("on the board"))).collect()
    }

    /// The creature cards in `you`'s graveyard but the card's own (CR
    /// 404.1, a graveyard is its owner's).
    fn creature_cards_in_your_graveyard(&self) -> Vec<ObjectId> {
        let mut graveyard: Vec<ObjectId> = self.those(|o| {
            o.zone == Zone::Graveyard && o.owner == self.you && !o.is_token && o.card_types.contains(&CardType::Creature)
        });
        // In the graveyard's order (CR 404.2), which `any_object` made the
        // order of creation.
        graveyard.sort();
        graveyard
    }

    /// The card's "when this creature dies" trigger resolving.
    fn dies(mut self, targets: &[Target]) -> (Self, Snapshot) {
        let before = Snapshot::of(&self.state);
        let card = self.card();
        self.registry.get(card).expect("registered").on_dies(&mut self.state, self.me, targets, &self.registry);
        (self, before)
    }
}

/// A 1/1 white Spirit creature token with flying, `n` of them, as one
/// event, under `you`.
fn spirits(n: u32, you: PlayerId) -> Act {
    Act::Tokens {
        n, controller: you, power: 1, toughness: 1, colors: vec![Color::White],
        types: vec![CardType::Creature], keywords: vec![Keyword::Flying], subtypes: vec!["Spirit".into()],
    }
}

card_proof! {
    /// Doomed Traveler: one Spirit for whoever controlled it as it died,
    /// and nothing else.
    fn doomed_traveler_leaves_a_spirit() {
        assert!(triggers(&isd::doomed_traveler::DoomedTraveler) == [(TriggerKind::SelfDies, None)]);
        let (b, before) = creature_in(Box::new(isd::doomed_traveler::DoomedTraveler), Zone::Graveyard).dies(&[]);
        assert!(asked_for(&[spirits(1, b.you)]) && Snapshot::of(&b.state) == before);
        std::mem::forget(b);
    }
}

card_proof! {
    /// Mausoleum Guard: two Spirits, as one event, for whoever controlled
    /// it as it died, and nothing else.
    fn mausoleum_guard_leaves_two_spirits() {
        assert!(triggers(&isd::mausoleum_guard::MausoleumGuard) == [(TriggerKind::SelfDies, None)]);
        let (b, before) = creature_in(Box::new(isd::mausoleum_guard::MausoleumGuard), Zone::Graveyard).dies(&[]);
        assert!(asked_for(&[spirits(2, b.you)]) && Snapshot::of(&b.state) == before);
        std::mem::forget(b);
    }
}

card_proof! {
    /// Pitchburn Devils: "any target" (CR 115.4), chosen as the trigger is
    /// put on the stack (603.3d); 3 damage from the Devils to it, whatever
    /// it is by then (a target gone illegal is the action's to ignore), and
    /// nothing else.
    fn pitchburn_devils_deals_three_to_any_target() {
        assert!(triggers(&isd::pitchburn_devils::PitchburnDevils) == [(TriggerKind::SelfDies, Some(TargetRequirement::AnyTarget))]);
        let b = creature_in(Box::new(isd::pitchburn_devils::PitchburnDevils), Zone::Graveyard);
        let target = any_target(b.others);
        let me = b.me;
        let (b, before) = b.dies(&[target.clone()]);
        assert!(asked_for(&[Act::Effect(target, PendingEffect::DealDamage { amount: 3, source_id: me })]));
        assert!(Snapshot::of(&b.state) == before);
        std::mem::forget(b);
    }
}

card_proof! {
    /// Moldgraf Monstrosity: exiled if it is still in the graveyard it died
    /// into (CR 400.7: anywhere else it is a new object), then two of the
    /// other creature cards in its controller's graveyard (all, if fewer)
    /// chosen at random and returned to the battlefield under their control
    /// (CR 110.2); and nothing else.
    fn moldgraf_monstrosity_exiles_itself_and_returns_two_at_random() {
        assert!(triggers(&isd::moldgraf_monstrosity::MoldgrafMonstrosity) == [(TriggerKind::SelfDies, None)]);
        let b = creature_in(Box::new(isd::moldgraf_monstrosity::MoldgrafMonstrosity), Zone::Graveyard);
        let in_graveyard = b.state.get_object(b.me).expect("on the board").zone == Zone::Graveyard;
        let returnable = b.creature_cards_in_your_graveyard();
        let (b, before) = b.dies(&[]);
        // Each index below written out, not computed: an action read at an
        // index Kani leaves open is a read of every recorded action at once.
        let returned_from = |first: usize| {
            Some(&Act::Random(listed(&returnable), 2)) == act(first)
                && acted() == first + 1 + returnable.len().min(2)
                && (1..=returnable.len().min(2)).all(|k| {
                    matches!(act(first + k), Some(Act::MoveUnder(id, Zone::Battlefield, controller)) if returnable.contains(id) && *controller == b.you)
                })
                && (returnable.len() < 2 || act(first + 1) != act(first + 2))
        };
        if in_graveyard {
            assert!(Some(&Act::Move(b.me, Zone::Exile)) == act(0) && returned_from(1));
        } else {
            assert!(returned_from(0));
        }
        assert!(Snapshot::of(&b.state) == before);
        std::mem::forget(b);
    }
}

/// The card's "whenever another creature dies" trigger resolving, for the
/// card's creature on the battlefield or gone, with `targets`.
fn another_dies(card: Box<dyn CardBehavior>, targets: impl FnOnce([ObjectId; 2]) -> Vec<Target>) -> (Board, Vec<Target>, Snapshot) {
    let on: bool = kani::any();
    let mut b = creature_in(card, if on { Zone::Battlefield } else { Zone::Graveyard });
    if on {
        b.state.get_object_mut(b.me).expect("on the board").tapped = kani::any();
    }
    let targets = targets(b.others);
    let dead = b.others[0];
    let before = Snapshot::of(&b.state);
    let card = b.card();
    b.registry.get(card).expect("registered").on_any_creature_dies(&mut b.state, b.me, dead,
        PlayerId(u8::from(kani::any::<bool>())), &[], kani::any(), kani::any(), &targets, &b.registry);
    (b, targets, before)
}

card_proof! {
    /// Galvanic Juggernaut: attacks each combat if able (CR 508.1d) and
    /// doesn't untap in its controller's untap step (502.3); untapped when
    /// another creature dies, and nothing else.
    fn galvanic_juggernaut_untaps_when_another_creature_dies() {
        assert!(prints_effects(&isd::galvanic_juggernaut::GalvanicJuggernaut,
            &[E::ForceAttack { scope: EffectScope::OnSelf }, E::PreventUntap { scope: EffectScope::OnSelf }]));
        assert!(triggers(&isd::galvanic_juggernaut::GalvanicJuggernaut) == [(TriggerKind::AnyCreatureDies, None)]);
        let (b, _, before) = another_dies(Box::new(isd::galvanic_juggernaut::GalvanicJuggernaut), |_| vec![]);
        assert!(asked_for(&[Act::Untap(b.me)]) && Snapshot::of(&b.state) == before);
        std::mem::forget(b);
    }
}

card_proof! {
    /// Rage Thrower: "target player or planeswalker", chosen as it
    /// triggers; 2 damage from the Rage Thrower to it, and nothing else.
    fn rage_thrower_deals_two_when_another_creature_dies() {
        assert!(prints_effects(&isd::rage_thrower::RageThrower, &[]));
        assert!(triggers(&isd::rage_thrower::RageThrower) == [(TriggerKind::AnyCreatureDies, Some(TargetRequirement::PlayerOrPlaneswalker))]);
        let (b, targets, before) = another_dies(Box::new(isd::rage_thrower::RageThrower), |others| vec![any_target(others)]);
        assert!(asked_for(&[Act::Effect(targets[0].clone(), PendingEffect::DealDamage { amount: 2, source_id: b.me })]));
        assert!(Snapshot::of(&b.state) == before);
        std::mem::forget(b);
    }
}

card_proof! {
    /// Elder Cathar: "target creature you control", chosen as it triggers
    /// by whoever controlled it as it died (CR 603.3d), its trigger's
    /// controller: a creature on the battlefield that player controls, and
    /// nothing else; resolving, two +1/+1 counters on that creature if it
    /// is a Human as the trigger resolves and one if not; its target gone,
    /// nothing (CR 608.2b). Nothing else.
    fn elder_cathar_puts_counters_on_your_creature_two_on_a_human() {
        let cathar = isd::elder_cathar::ElderCathar;
        assert!(triggers(&cathar) == [(TriggerKind::SelfDies, Some(TargetRequirement::Creature))]);
        const HUMANS: Kinds = Kinds { types: &[CardType::Creature, CardType::Artifact], subtypes: &["Human"], keywords: &[] };
        let b = creature_among(Box::new(cathar), Zone::Graveyard, &HUMANS);
        let card = b.card();
        let behavior = b.registry.get(card).expect("registered");
        for id in [b.me, b.others[0], b.others[1]] {
            let o = b.state.get_object(id).expect("on the board");
            let yours = o.zone == Zone::Battlefield && o.controller == b.you && o.card_types.contains(&CardType::Creature);
            assert!(behavior.is_valid_target(&b.state, b.you, &Target::Object(id), &b.registry) == yours);
        }
        assert!(!behavior.is_valid_target(&b.state, b.you, &Target::Player(b.you), &b.registry));
        let target = if kani::any() { Target::Object(b.others[usize::from(kani::any::<bool>())]) } else { Target::Illegal };
        let human = match &target {
            Target::Object(id) => b.state.get_object(*id).expect("on the board").subtypes.iter().any(|s| s == "Human"),
            _ => false,
        };
        let (b, before) = b.dies(&[target.clone()]);
        match target {
            Target::Object(id) => {
                assert!(asked_for(&[Act::Counters(id, crate::types::CounterType::PlusOnePlusOne, if human { 2 } else { 1 })]));
            }
            _ => {
                assert!(acted() == 0);
            }
        }
        assert!(Snapshot::of(&b.state) == before);
        std::mem::forget(b);
    }
}

/// Bitterheart Witch's board: the Witch in player 0's graveyard (it has
/// died), last controlled by `you`; player 0's library of any of a card of
/// the blank card and two of one Curse, in that order; player 1's of the
/// same Curse.
struct Witch {
    registry: CardRegistry,
    state: GameState,
    card: crate::ids::CardId,
    me: ObjectId,
    you: PlayerId,
    /// The Curse cards in `you`'s library, one of each card, in library
    /// order.
    curse: Option<ObjectId>,
}

fn witch() -> Witch {
    let (mut registry, mut state, blank, card) = board(Box::new(isd::bitterheart_witch::BitterheartWitch));
    let curse_card = registry.register(Box::new(isd::curse_of_the_pierced_heart::CurseOfThePiercedHeart));
    let you = PlayerId(u8::from(kani::any::<bool>()));
    let me = state.create_object(card, PlayerId(0), Zone::Graveyard, None, None);
    state.players[0].graveyard_order.push(me);
    state.get_object_mut(me).expect("just created").last_controller = Some(you);
    let mut first = [None; 2];
    for of in [blank, curse_card, curse_card] {
        let id = state.create_object(of, PlayerId(0), Zone::Library, None, None);
        if kani::any() {
            state.players[0].library_order.push(id);
            if of == curse_card && first[0].is_none() {
                first[0] = Some(id);
            }
        } else {
            state.get_object_mut(id).expect("just created").zone = Zone::Hand;
        }
    }
    let theirs = state.create_object(curse_card, PlayerId(1), Zone::Library, None, None);
    state.players[1].library_order.push(theirs);
    first[1] = Some(theirs);
    let curse = if you == PlayerId(0) { first[0] } else { first[1] };
    Witch { registry, state, card, me, you, curse }
}

card_proof! {
    /// Bitterheart Witch dying: "target player", chosen as it triggers
    /// (CR 603.3d); resolving, whoever controlled the Witch as it died is
    /// asked whether to search ("you may"). Declining, nothing. Searching,
    /// they search their own library and shuffle it (CR 701.23a: a search
    /// happens, and so does the shuffle, whether or not a card is found),
    /// and are offered one of each Curse card there, "none" an answer too
    /// (CR 701.23b); with none there, nothing more. With no player
    /// targeted, nothing at all. Nothing else.
    fn bitterheart_witch_may_search_for_a_curse() {
        assert!(triggers(&isd::bitterheart_witch::BitterheartWitch) == [(TriggerKind::SelfDies, Some(TargetRequirement::PlayerOnly))]);
        let Witch { registry, mut state, card, me, you, curse } = witch();
        let witch = registry.get(card).expect("registered");
        let before = Snapshot::of(&state);
        let target = match kani::any::<u8>() % 3 {
            0 => Target::Player(PlayerId(0)),
            1 => Target::Player(PlayerId(1)),
            _ => Target::Illegal,
        };
        witch.on_dies(&mut state, me, &[target.clone()], &registry);
        let Target::Player(p) = target else {
            assert!(acted() == 0 && Snapshot::of(&state) == before);
            std::mem::forget((registry, state));
            return;
        };
        let asked = matches!(&state.awaiting_action, Some(AwaitingAction::ResolutionChoice {
            player, source, choice: ResolutionChoiceKind::YesNo { source_card, .. },
        }) if *player == you && *source == me && *source_card == me);
        assert!(asked && acted() == 0);
        assert!(state.get_object(me).expect("on the board").card_state.get("curse_target") == Some(&ObjectId(u64::from(p.0))));
        state.awaiting_action = None;
        let yes: bool = kani::any();
        witch.on_yes_no_choice(&mut state, me, yes, &registry);
        if !yes {
            assert!(acted() == 0 && Snapshot::of(&state) == before);
        } else if let Some(curse) = curse {
            let asked = matches!(&state.awaiting_action, Some(AwaitingAction::ResolutionChoice {
                player, source, choice: ResolutionChoiceKind::ChooseTarget {
                    options, optional: true, effect: PendingEffect::CardEffect { source_id, .. }, ..
                },
            }) if *player == you && *source == me && *source_id == me && options.len() == 1 && options[0] == Target::Object(curse));
            assert!(asked && asked_for(&[Act::Shuffle(you)]));
        } else {
            assert!(asked_for(&[Act::Shuffle(you)]) && Snapshot::of(&state) == before);
        }
        std::mem::forget((registry, state));
    }
}

card_proof! {
    /// Bitterheart Witch, a Curse chosen: put onto the battlefield attached
    /// to the player the trigger targeted, if that player can be enchanted
    /// by it (CR 303.4h: an Aura that can't legally be attached to what it
    /// would enter attached to stays where it is; the card's ruling: a
    /// player with protection from its colour can't be); otherwise it stays
    /// in the library. Nothing else.
    fn bitterheart_witch_attaches_the_curse_to_its_target() {
        let Witch { registry, mut state, card, me, curse, .. } = witch();
        let Some(curse) = curse else {
            std::mem::forget((registry, state));
            return;
        };
        let p = PlayerId(u8::from(kani::any::<bool>()));
        state.get_object_mut(me).expect("on the board").card_state.insert("curse_target".into(), ObjectId(u64::from(p.0)));
        let can = state.player_can_be_enchanted_by(curse, p, &registry);
        let before = Snapshot::of(&state);
        registry.get(card).expect("registered").resolve_card_effect(&mut state, me, "choose", &Target::Object(curse), &registry);
        if can {
            assert!(asked_for(&[Act::Move(curse, Zone::Battlefield)]));
            assert!(before.with_attached_player(curse, p) == Snapshot::of(&state));
        } else {
            assert!(acted() == 0 && Snapshot::of(&state) == before);
        }
        std::mem::forget((registry, state));
    }
}

/// The card's "whenever this creature attacks" trigger resolving, for the
/// creature on the battlefield (or gone since) under either player's
/// control, attacking the other.
fn attacks(card: Box<dyn CardBehavior>) -> (Board, Snapshot) {
    let on: bool = kani::any();
    let mut b = creature_in(card, if on { Zone::Battlefield } else { Zone::Graveyard });
    let before = Snapshot::of(&b.state);
    let attack = AttackInfo::new(b.me, PlayerId(1 - b.you.0));
    let card = b.card();
    b.registry.get(card).expect("registered").on_attacks(&mut b.state, b.me, attack, &[], &b.registry);
    (b, before)
}

card_proof! {
    /// Geist of Saint Traft: a 4/4 white Angel creature token with flying
    /// for its controller, put onto the battlefield tapped and attacking
    /// (CR 508.4), and that token to be exiled at end of combat by a
    /// delayed trigger of the Geist's (603.7); and nothing else.
    fn geist_of_saint_traft_brings_an_angel_for_the_combat() {
        assert!(prints_effects(&isd::geist_of_saint_traft::GeistOfSaintTraft, &[]));
        assert!(triggers(&isd::geist_of_saint_traft::GeistOfSaintTraft) == [(TriggerKind::Attacks, None)]);
        let (b, before) = attacks(Box::new(isd::geist_of_saint_traft::GeistOfSaintTraft));
        let angel = new_ids(&b.state, 1);
        assert!(asked_for(&[
            Act::Tokens {
                n: 1, controller: b.you, power: 4, toughness: 4, colors: vec![Color::White],
                types: vec![CardType::Creature], keywords: vec![Keyword::Flying], subtypes: vec!["Angel".into()],
            },
            Act::EnterAttacking { source: b.me, controller: b.you, tokens: angel.clone() },
        ]));
        let exiles = &b.state.end_of_combat_exiles[before.end_of_combat_exiles()..];
        let card = b.state.get_object(b.me).expect("on the board").card_id;
        assert!(exiles.len() == 1 && exiles[0].target_id == angel[0] && exiles[0].source_id == b.me
            && exiles[0].source_card_id == card && exiles[0].controller == b.you);
        assert!(Snapshot::of(&b.state).without_end_of_combat_exiles() == before.without_end_of_combat_exiles());
        std::mem::forget(b);
    }
}

card_proof! {
    /// Kessig Cagebreakers: a 2/2 green Wolf creature token for each
    /// creature card in its controller's graveyard, as one event, put onto
    /// the battlefield tapped and attacking; with none, nothing.
    fn kessig_cagebreakers_brings_a_wolf_for_each_creature_card_in_your_graveyard() {
        assert!(prints_effects(&isd::kessig_cagebreakers::KessigCagebreakers, &[]));
        assert!(triggers(&isd::kessig_cagebreakers::KessigCagebreakers) == [(TriggerKind::Attacks, None)]);
        let (b, before) = attacks(Box::new(isd::kessig_cagebreakers::KessigCagebreakers));
        // The Cagebreakers' own card counts too, if it is in that graveyard.
        let mine = b.state.get_object(b.me).is_some_and(|o| o.zone == Zone::Graveyard && o.owner == b.you);
        let n = u32::try_from(b.creature_cards_in_your_graveyard().len() + usize::from(mine)).expect("small");
        if n == 0 {
            assert!(acted() == 0);
        } else {
            assert!(asked_for(&[
                Act::Tokens {
                    n, controller: b.you, power: 2, toughness: 2, colors: vec![Color::Green],
                    types: vec![CardType::Creature], keywords: vec![], subtypes: vec!["Wolf".into()],
                },
                Act::EnterAttacking { source: b.me, controller: b.you, tokens: new_ids(&b.state, n) },
            ]));
        }
        assert!(Snapshot::of(&b.state) == before);
        std::mem::forget(b);
    }
}

card_proof! {
    /// Grimgrin, Corpse-Born: doesn't untap in its controller's untap step
    /// (CR 502.3); its attack trigger targets "creature defending player
    /// controls", chosen as it triggers: a creature on the battlefield the
    /// opponent of its controller controls, the one player it can be
    /// attacking in a two-player game (CR 506.2), and nothing else.
    /// Resolving, that creature destroyed, and then a +1/+1 counter on
    /// Grimgrin whether the creature was destroyed or not (the card's
    /// ruling: one that regenerates, or is indestructible, still leaves
    /// Grimgrin its counter); with no target, nothing. Nothing else.
    fn grimgrin_destroys_a_defending_creature_and_grows() {
        let grimgrin = isd::grimgrin_corpse_born::GrimgrinCorpseBorn;
        assert!(prints_effects(&grimgrin, &[E::PreventUntap { scope: EffectScope::OnSelf }]));
        assert!(triggers(&grimgrin) == [(TriggerKind::Attacks, Some(TargetRequirement::Creature))]);
        let on: bool = kani::any();
        let mut b = creature_in(Box::new(grimgrin), if on { Zone::Battlefield } else { Zone::Graveyard });
        let card = b.card();
        let behavior = b.registry.get(card).expect("registered");
        let caster = PlayerId(u8::from(kani::any::<bool>()));
        for id in [b.me, b.others[0], b.others[1]] {
            let o = b.state.get_object(id).expect("on the board");
            let theirs = o.zone == Zone::Battlefield && o.controller != caster && o.card_types.contains(&CardType::Creature);
            assert!(behavior.is_valid_target(&b.state, caster, &Target::Object(id), &b.registry) == theirs);
        }
        assert!(!behavior.is_valid_target(&b.state, caster, &Target::Player(PlayerId(1 - caster.0)), &b.registry));
        let before = Snapshot::of(&b.state);
        let target = if kani::any() { Some(any_target(b.others)) } else { None };
        let attack = AttackInfo::new(b.me, PlayerId(1 - b.you.0));
        behavior.on_attacks(&mut b.state, b.me, attack, target.as_slice(), &b.registry);
        let effect = PendingEffect::CardEffect { source_id: b.me, key: String::new() };
        match &target {
            Some(t) => {
                assert!(acted() == 1 && Some(&Act::Effect(t.clone(), effect)) == act(0));
            }
            None => {
                assert!(acted() == 0);
            }
        }
        let target = target.unwrap_or(Target::Illegal);
        let n = acted();
        behavior.resolve_card_effect(&mut b.state, b.me, "", &target, &b.registry);
        match target {
            Target::Object(id) => {
                assert!(acted() == n + 2 && Some(&Act::Destroy(id)) == act(n));
                assert!(Some(&Act::Counters(b.me, crate::types::CounterType::PlusOnePlusOne, 1)) == act(n + 1));
            }
            _ => {
                assert!(acted() == n);
            }
        }
        assert!(Snapshot::of(&b.state) == before);
        std::mem::forget(b);
    }
}
