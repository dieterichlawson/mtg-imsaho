//! The double-faced cards that ask their controller whether to transform
//! (CR 701.28, 712).
//!
//! - Delver of Secrets // Insectile Aberration: "At the beginning of your
//!   upkeep, look at the top card of your library. You may reveal that
//!   card. If an instant or sorcery card is revealed this way, transform
//!   this creature." // "Flying"
//! - Cloistered Youth // Unholy Fiend: "At the beginning of your upkeep,
//!   you may transform this creature." // "At the beginning of your end
//!   step, you lose 1 life."
//!
//! Each face's triggers are pinned, the engine reading the face that is up
//! (CR 712.8d, `proofs::actions`). Each proof runs the card's code over the
//! creature on the battlefield or gone by then, front face up or back,
//! under either player's control; and, for Delver, a library whose top
//! card is an instant (Lightning Bolt), a sorcery (Lava Axe), the blank
//! card, or no card at all.

use super::{acted, asked_for, board, card_proof, case_by_case, picked, triggers, Act, Snapshot};
use crate::cards::{CardBehavior, CardRegistry, TriggerKind, TriggerScope};
use crate::ids::{CardId, ObjectId, PlayerId};
use crate::state::{AwaitingAction, GameState, ResolutionChoiceKind};
use crate::types::Zone;

use crate::cards::isd;

struct Board {
    registry: CardRegistry,
    state: GameState,
    me: ObjectId,
    you: PlayerId,
}

fn creature(card: Box<dyn CardBehavior>) -> Board {
    creature_of(card, PlayerId(u8::from(kani::any::<bool>())))
}

/// [`creature`]'s board, controlled (or last controlled) by `you`.
fn creature_of(card: Box<dyn CardBehavior>, you: PlayerId) -> Board {
    creature_at(card, you, kani::any(), kani::any())
}

/// [`creature_of`]'s board, the creature in its owner's graveyard exactly
/// if `gone`, and transformed exactly if `transformed`.
fn creature_at(card: Box<dyn CardBehavior>, you: PlayerId, gone: bool, transformed: bool) -> Board {
    let (registry, mut state, _, card) = board(card);
    let me = state.create_object(card, PlayerId(0), if gone { Zone::Graveyard } else { Zone::Battlefield }, None, None);
    let o = state.get_object_mut(me).expect("just created");
    o.is_transformed = transformed;
    if gone {
        o.last_controller = Some(you);
        state.players[0].graveyard_order.push(me);
    } else {
        o.controller = you;
    }
    Board { registry, state, me, you }
}

impl Board {
    fn card(&self) -> CardId {
        self.state.get_object(self.me).expect("on the board").card_id
    }

    /// Whether the creature is on the battlefield front face up.
    fn front_up(&self) -> bool {
        let o = self.state.get_object(self.me).expect("on the board");
        o.zone == Zone::Battlefield && !o.is_transformed
    }

    /// Whether its controller has been asked yes or no about it.
    fn asked(&self) -> bool {
        matches!(&self.state.awaiting_action, Some(AwaitingAction::ResolutionChoice {
            player, source, choice: ResolutionChoiceKind::YesNo { source_card, .. },
        }) if *player == self.you && *source == self.me && *source_card == self.me)
    }

    fn upkeep(&mut self) -> Snapshot {
        let before = Snapshot::of(&self.state);
        let card = self.card();
        self.registry.get(card).expect("registered").on_upkeep(&mut self.state, self.me, &[], &self.registry);
        before
    }

    /// The answer to the question, after it was asked.
    fn answer(&mut self, yes: bool) {
        self.state.awaiting_action = None;
        let card = self.card();
        self.registry.get(card).expect("registered").on_yes_no_choice(&mut self.state, self.me, yes, &self.registry);
    }
}

/// The back face's triggered abilities, as the engine reads them with it up.
fn back_triggers(card: &dyn CardBehavior) -> Vec<TriggerKind> {
    let back = std::mem::ManuallyDrop::new(card.back_face_data().expect("a back face"));
    back.triggered_abilities.iter().map(|t| t.kind.clone()).collect()
}

/// Delver of Secrets: at its controller's upkeep, front face up, they
/// look at the top card of their library and are asked whether to
/// reveal it (with no card there, nothing to look at, nothing asked);
/// revealed and an instant or sorcery card, Delver transforms; not
/// revealed, or another card, nothing. Back face up, or gone, nothing.
///
/// Its controller, and the top card of their library (Lightning
/// Bolt, Lava Axe, the blank card, or none), are constants in each
/// harness: a card the solver chose is an index into the registry it
/// leaves open, a library made or not a list of a length it leaves
/// open, and a controller it chose an index into the players.
fn delver_of_secrets_transforms_on_a_revealed_instant_or_sorcery(case: u8) {
    let delver = isd::delver_of_secrets::DelverOfSecrets;
    assert!(triggers(&delver) == [(TriggerKind::Upkeep, None)] && back_triggers(&delver).is_empty());
    assert!(delver.step_trigger_scope(&TriggerKind::Upkeep, false) == TriggerScope::Your);
    let mut b = creature_of(Box::new(isd::delver_of_secrets::DelverOfSecrets), PlayerId(case / 4));
    // It transforms only as the trigger says, never on its own.
    assert!(!isd::delver_of_secrets::DelverOfSecrets.should_transform(&b.state, b.me, &b.registry));
    let bolt = b.registry.register(Box::new(crate::cards::lightning_bolt::LightningBolt));
    let axe = b.registry.register(Box::new(crate::cards::lava_axe::LavaAxe));
    let which = case % 4;
    if which < 3 {
        let card = match which { 0 => bolt, 1 => axe, _ => CardId(1) };
        let top = b.state.create_object(card, b.you, Zone::Library, None, None);
        if b.you == PlayerId(0) {
            b.state.players[0].library_order.push(top);
        } else {
            b.state.players[1].library_order.push(top);
        }
    }
    let front_up = b.front_up();
    let before = b.upkeep();
    if front_up && which < 3 {
        assert!(b.asked() && acted() == 0);
        let yes: bool = kani::any();
        b.answer(yes);
        if yes && which < 2 {
            assert!(asked_for(&[Act::Transform(b.me)]));
        } else {
            assert!(acted() == 0);
        }
    } else {
        assert!(!b.asked() && acted() == 0);
    }
    assert!(Snapshot::of(&b.state) == before);
    std::mem::forget(b);
}

case_by_case! { delver_of_secrets_transforms_on_a_revealed_instant_or_sorcery;
    delver_of_secrets_of_the_first_player_over_lightning_bolt: 0,
    delver_of_secrets_of_the_first_player_over_lava_axe: 1,
    delver_of_secrets_of_the_first_player_over_a_blank_card: 2,
    delver_of_secrets_of_the_first_player_over_nothing: 3,
    delver_of_secrets_of_the_second_player_over_lightning_bolt: 4,
    delver_of_secrets_of_the_second_player_over_lava_axe: 5,
    delver_of_secrets_of_the_second_player_over_a_blank_card: 6,
    delver_of_secrets_of_the_second_player_over_nothing: 7,
}

card_proof! {
    /// Cloistered Youth // Unholy Fiend: an upkeep trigger on the front
    /// face and an end step trigger on the back, each its controller's
    /// alone. What they do is the proof below.
    fn cloistered_youth_and_unholy_fiend_trigger_at_your_upkeep_and_end_step() {
        let youth = isd::cloistered_youth::CloisteredYouth;
        assert!(triggers(&youth) == [(TriggerKind::Upkeep, None)] && back_triggers(&youth) == [TriggerKind::EndStep]);
        for back in [false, true] {
            assert!(youth.step_trigger_scope(&TriggerKind::Upkeep, back) == TriggerScope::Your);
            assert!(youth.step_trigger_scope(&TriggerKind::EndStep, back) == TriggerScope::Your);
        }
    }
}

/// Cloistered Youth: at its controller's upkeep, front face up, they are
/// asked whether to transform it, and it transforms on "yes"; Unholy
/// Fiend: at its controller's end step, they lose 1 life. In its owner's
/// graveyard for `placed` 0, front face up on the battlefield for 1 and
/// transformed there for 2, and answered `yes` or not, each a constant in
/// its harness, as the cases of one harness add up.
fn cloistered_youth_may_transform_and_unholy_fiend_costs_a_life((placed, yes): (u8, bool)) {
    let mut b = creature_at(Box::new(isd::cloistered_youth::CloisteredYouth), PlayerId(u8::from(kani::any::<bool>())), placed == 0, placed == 2);
    let before = b.upkeep();
    if placed == 1 {
        assert!(b.asked() && acted() == 0);
        b.answer(yes);
        if yes {
            assert!(asked_for(&[Act::Transform(b.me)]));
        } else {
            assert!(acted() == 0);
        }
    } else {
        assert!(!b.asked() && acted() == 0);
        let card = b.card();
        b.registry.get(card).expect("registered").on_end_step(&mut b.state, b.me, &[], &b.registry);
        assert!(asked_for(&[Act::Life(b.you, -1)]));
    }
    assert!(Snapshot::of(&b.state) == before);
    std::mem::forget(b);
}

case_by_case! { cloistered_youth_may_transform_and_unholy_fiend_costs_a_life;
    cloistered_youth_gone_asks_nothing: (0, false),
    cloistered_youth_transforms_on_yes: (1, true),
    cloistered_youth_stays_on_no: (1, false),
    unholy_fiend_costs_a_life: (2, false),
}

card_proof! {
    /// Screeching Bat // Stalking Vampire: an upkeep trigger on each face,
    /// its controller's alone. What it does is the proof below.
    fn screeching_bat_triggers_at_your_upkeep_either_way() {
        let bat = isd::screeching_bat::ScreechingBat;
        assert!(triggers(&bat) == [(TriggerKind::Upkeep, None)] && back_triggers(&bat) == [TriggerKind::Upkeep]);
        for back in [false, true] {
            assert!(bat.step_trigger_scope(&TriggerKind::Upkeep, back) == TriggerScope::Your);
        }
    }
}

/// Screeching Bat // Stalking Vampire: at its controller's upkeep, on the
/// battlefield either face up, they are asked whether to pay {2}{B}{B} if
/// they can (with no way to pay it the "may" has one answer, and nothing
/// is asked); on "yes" they pay it and, if it was paid and the creature is
/// still there, it transforms. In its owner's graveyard for `placed` 0,
/// front face up on the battlefield for 1 and transformed there for 2, and
/// answered `yes` or not, each a constant in its harness, as the cases of
/// one harness add up.
fn screeching_bat_may_pay_to_transform_either_way((placed, yes): (u8, bool)) {
    let mut b = creature_at(Box::new(isd::screeching_bat::ScreechingBat), PlayerId(u8::from(kani::any::<bool>())), placed == 0, placed == 2);
    let before = b.upkeep();
    if b.asked() {
        assert!(placed != 0 && acted() == 0);
        b.answer(yes);
        let cost = crate::types::ManaCost::new(vec![
            crate::types::ManaSymbol::Generic(2),
            crate::types::ManaSymbol::Colored(crate::types::Color::Black),
            crate::types::ManaSymbol::Colored(crate::types::Color::Black),
        ]);
        match super::act(0) {
            None => {
                assert!(!yes);
            }
            Some(Act::Pay(p, paid_cost, paid)) => {
                assert!(yes && *p == b.you && *paid_cost == cost);
                if *paid {
                    assert!(acted() == 2 && Some(&Act::Transform(b.me)) == super::act(1));
                } else {
                    assert!(acted() == 1);
                }
            }
            _ => panic!("pays first"),
        }
    } else {
        assert!(acted() == 0);
    }
    assert!(Snapshot::of(&b.state) == before);
    std::mem::forget(b);
}

case_by_case! { screeching_bat_may_pay_to_transform_either_way;
    screeching_bat_gone_asks_nothing: (0, false),
    screeching_bat_pays_and_transforms_on_yes: (1, true),
    screeching_bat_stays_on_no: (1, false),
    stalking_vampire_pays_and_transforms_on_yes: (2, true),
    stalking_vampire_stays_on_no: (2, false),
}

card_proof! {
    /// Thraben Sentry // Thraben Militia: triggers, front face up, for
    /// another creature its controller controlled as it died, and for no
    /// other death (back face up, it has no such ability). What it does is
    /// the proof below.
    fn thraben_sentry_triggers_when_another_creature_of_yours_dies() {
        let sentry = isd::thraben_sentry::ThrabenSentry;
        assert!(triggers(&sentry) == [(TriggerKind::AnyCreatureDies, None)] && back_triggers(&sentry).is_empty());
        let b = creature(Box::new(sentry));
        let transformed = b.state.get_object(b.me).expect("on the board").is_transformed;
        let dead_controller = PlayerId(u8::from(kani::any::<bool>()));
        let card = b.card();
        let fires = b.registry.get(card).expect("registered").should_trigger_on_creature_dies(
            &b.state, b.me, ObjectId(100), dead_controller, &[], 1, false, &[], &b.registry);
        assert!(fires == (!transformed && dead_controller == b.you));
        std::mem::forget(b);
    }
}

/// Thraben Sentry's trigger resolving: its controller is asked whether to
/// transform it, still there front face up, and it transforms on "yes";
/// gone, or transformed already, nothing is asked. In its owner's
/// graveyard for `placed` 0, front face up on the battlefield for 1 and
/// transformed there for 2, and answered `yes` or not, each a constant in
/// its harness: with them the solver's, and the trigger in the same
/// harness, the proof ran out of memory at four gigabytes in symbolic
/// execution.
fn thraben_sentry_may_transform_when_another_creature_of_yours_dies((placed, yes): (u8, bool)) {
    let mut b = creature_at(Box::new(isd::thraben_sentry::ThrabenSentry), PlayerId(u8::from(kani::any::<bool>())), placed == 0, placed == 2);
    let card = b.card();
    let before = Snapshot::of(&b.state);
    b.registry.get(card).expect("registered").on_any_creature_dies(
        &mut b.state, b.me, ObjectId(100), b.you, &[], 1, false, &[], &b.registry);
    if placed == 1 {
        assert!(b.asked() && acted() == 0);
        b.answer(yes);
        if yes {
            assert!(asked_for(&[Act::Transform(b.me)]));
        } else {
            assert!(acted() == 0);
        }
    } else {
        assert!(!b.asked() && acted() == 0);
    }
    assert!(Snapshot::of(&b.state) == before);
    std::mem::forget(b);
}

case_by_case! { thraben_sentry_may_transform_when_another_creature_of_yours_dies;
    thraben_sentry_gone_asks_nothing: (0, false),
    thraben_sentry_transforms_on_yes: (1, true),
    thraben_sentry_stays_on_no: (1, false),
    thraben_militia_asks_nothing: (2, false),
}

/// The card's activated abilities as the engine reads them, each at its
/// index; compared with `Ability::describes`, as the activated-ability
/// proofs pin them.
fn abilities(b: &Board) -> std::mem::ManuallyDrop<Vec<crate::cards::ActivatedAbilityDef>> {
    let card = b.card();
    let defs = std::mem::ManuallyDrop::new(b.registry.get(card).expect("registered").activated_abilities(&b.state, b.me, &b.registry));
    for (i, d) in defs.iter().enumerate() {
        assert!(d.ability_index == i);
    }
    defs
}

/// Puts a card in `you`'s `zone`: Grizzly Bears, a creature card, or the
/// blank card, which is none.
fn card_of_yours(b: &mut Board, bears: CardId, zone: Zone) -> ObjectId {
    let card = if kani::any() { bears } else { CardId(1) };
    let id = b.state.create_object(card, b.you, zone, None, None);
    if zone == Zone::Library {
        if b.you == PlayerId(0) {
            b.state.players[0].library_order.push(id);
        } else {
            b.state.players[1].library_order.push(id);
        }
    }
    id
}

/// Civilized Scholar // Homicidal Brute: front face up on the battlefield,
/// "{T}: Draw a card, then discard a card"; transformed or off the
/// battlefield, no ability. Its resolution is the proof below. Where it is
/// and which face is up are constants in each harness, as for Ulvenwald
/// Mystics: whether it has the ability is a list of abilities of a length
/// they decide, and with them the solver's, and the resolution in the same
/// harness, the proof ran out of memory at four and a half gigabytes in
/// symbolic execution. In its owner's graveyard for 0, front face up on
/// the battlefield for 1, and transformed there for 2.
fn civilized_scholar_placed(placed: u8) {
    let b = creature_at(Box::new(isd::civilized_scholar::CivilizedScholar), PlayerId(u8::from(kani::any::<bool>())), placed == 0, placed == 2);
    let got = abilities(&b);
    if placed == 1 {
        assert!(got.len() == 1 && super::activated::Ability { tap: true, ..super::activated::Ability::default() }.describes(&got[0]));
    } else {
        assert!(got.is_empty());
    }
    std::mem::forget((b, got));
}

case_by_case! { civilized_scholar_placed;
    civilized_scholar_gone_has_no_ability: 0,
    civilized_scholar_face_up_has_its_ability: 1,
    homicidal_brute_has_no_ability: 2,
}

/// Civilized Scholar's ability resolving: its controller draws, then
/// discards a card of their hand as it then is (the only one without
/// being asked, else the one they choose); a creature card discarded, the
/// Scholar untaps and then transforms. With a card, Grizzly Bears or the
/// blank card, in its controller's library exactly if `in_library` and in
/// their hand exactly if `in_hand`, each a constant in its harness: the
/// hand drawn into is a list of a length they decide.
fn civilized_scholar_draws_discards_and_transforms_on_a_creature((in_library, in_hand): (bool, bool)) {
    let mut b = creature(Box::new(isd::civilized_scholar::CivilizedScholar));
    let bears = b.registry.register(Box::new(crate::cards::grizzly_bears::GrizzlyBears));
    if in_library {
        card_of_yours(&mut b, bears, Zone::Library);
    }
    if in_hand {
        card_of_yours(&mut b, bears, Zone::Hand);
    }
    b.state.resolving_ability_activator = Some(b.you);
    let card = b.card();
    b.registry.get(card).expect("registered").resolve_activated_ability(&mut b.state, b.me, 0, &[], &b.registry);
    let hand = picked(&b.state.objects_in_zone(Zone::Hand, b.you), |o| Some(o.id));
    let bears_in_hand = picked(&hand, |id| Some(b.state.get_object(*id).expect("on the board").card_id == bears));
    let creature_card = |id: ObjectId| hand.iter().zip(&bears_in_hand).any(|(h, is)| *h == id && *is);
    assert!(Some(&Act::Draw(b.you, 1)) == super::act(0));
    match hand.as_slice() {
        [] => {
            assert!(acted() == 1);
        }
        [only] if creature_card(*only) => {
            assert!(asked_for(&[Act::Draw(b.you, 1), Act::Discard(*only), Act::Untap(b.me), Act::Transform(b.me)]));
        }
        [only] => {
            assert!(asked_for(&[Act::Draw(b.you, 1), Act::Discard(*only)]));
        }
        _ => {
            let asked = matches!(&b.state.awaiting_action, Some(AwaitingAction::ResolutionChoice {
                player, source, choice: ResolutionChoiceKind::ChooseCardFromHand { player: chooser, cards, discard_immediately: true, remaining: 1, .. },
            }) if *player == b.you && *source == b.me && *chooser == b.you && *cards == hand);
            assert!(asked && acted() == 1);
            let discarded = hand[usize::from(kani::any::<bool>())];
            b.state.awaiting_action = None;
            b.registry.get(card).expect("registered").on_discard_choice(&mut b.state, b.me, discarded, &b.registry);
            if creature_card(discarded) {
                assert!(asked_for(&[Act::Draw(b.you, 1), Act::Untap(b.me), Act::Transform(b.me)]));
            } else {
                assert!(acted() == 1);
            }
        }
    }
    std::mem::forget((b, hand, bears_in_hand));
}

case_by_case! { civilized_scholar_draws_discards_and_transforms_on_a_creature;
    civilized_scholar_draws_from_an_empty_library_with_an_empty_hand: (false, false),
    civilized_scholar_draws_from_an_empty_library_with_a_card_in_hand: (false, true),
    civilized_scholar_draws_a_card_with_an_empty_hand: (true, false),
    civilized_scholar_draws_a_card_with_a_card_in_hand: (true, true),
}

card_proof! {
    /// Homicidal Brute: triggers at its controller's end step (as the back
    /// face's ability, CR 712.8d), if it didn't attack this turn, checked
    /// as it triggers and as it resolves; tapped, then transformed.
    fn homicidal_brute_taps_and_transforms_if_it_did_not_attack() {
        let scholar = isd::civilized_scholar::CivilizedScholar;
        assert!(triggers(&scholar).is_empty() && back_triggers(&scholar) == [TriggerKind::EndStep]);
        assert!(scholar.step_trigger_scope(&TriggerKind::EndStep, true) == TriggerScope::Your);
        let mut b = creature(Box::new(scholar));
        let attacked: bool = kani::any();
        if attacked {
            b.state.get_object_mut(b.me).expect("on the board").attacked_on_turn = Some(b.state.turn_number);
        }
        let card = b.card();
        assert!(b.registry.get(card).expect("registered").should_trigger(&b.state, b.me, &TriggerKind::EndStep, &b.registry) == !attacked);
        let o = b.state.get_object(b.me).expect("on the board");
        let back_up = o.zone == Zone::Battlefield && o.is_transformed;
        let before = Snapshot::of(&b.state);
        b.registry.get(card).expect("registered").on_end_step(&mut b.state, b.me, &[], &b.registry);
        if back_up && !attacked {
            assert!(asked_for(&[Act::Tap(b.me), Act::Transform(b.me)]));
        } else {
            assert!(acted() == 0);
        }
        assert!(Snapshot::of(&b.state) == before);
        std::mem::forget(b);
    }
}

card_proof! {
    /// Lord of Lineage, Bloodline Keeper's back face: "Other Vampire
    /// creatures you control get +2/+2". Its abilities and their
    /// resolutions are the proofs below.
    fn lord_of_lineage_pumps_your_other_vampires() {
        let back = std::mem::ManuallyDrop::new(isd::bloodline_keeper::BloodlineKeeper.back_face_data().expect("a back face"));
        assert!(matches!(back.continuous_effects.as_slice(), [crate::types::ContinuousEffect::ModifyPT {
            power: 2, toughness: 2, scope: crate::types::EffectScope::GlobalOther(f),
        }] if super::both(f, |f| matches!(f, crate::types::CreatureFilter::ControlledByYou), |f| super::has_subtype(f, "Vampire"))));
    }
}

/// Bloodline Keeper // Lord of Lineage: either face, "{T}: Create a 2/2
/// black Vampire creature token with flying"; front face up, a card, and
/// with five or more Vampires its controller controls, "{B}: Transform
/// this creature"; off the battlefield, no ability. Over four tokens
/// beside it, each either player's and a Vampire or not; with it in its
/// owner's graveyard for `placed` 0, front face up on the battlefield for
/// 1 and transformed there for 2, and, front face up, `yours` of the
/// tokens Vampires of its controller and the others each a Vampire of the
/// other player or not a Vampire, each a constant in its harness: the
/// abilities are a list of a length they decide, and with them the
/// solver's the proof ran out of memory at four and a half gigabytes in
/// symbolic execution.
fn bloodline_keeper_has_its_abilities((placed, yours): (u8, Option<u8>)) {
    let mut b = creature_at(Box::new(isd::bloodline_keeper::BloodlineKeeper), PlayerId(u8::from(kani::any::<bool>())), placed == 0, placed == 2);
    let mut vampires = 1;
    for k in 0..4 {
        // Either player's and a Vampire or not; or, `yours` given, the
        // first `yours` Vampires of `you`, and each other one not a
        // Vampire of `you`.
        let (controller, vampire) = match yours {
            None => (PlayerId(u8::from(kani::any::<bool>())), kani::any()),
            Some(n) if k < n => (b.you, true),
            Some(_) => {
                let vampire: bool = kani::any();
                (if vampire { PlayerId(1 - b.you.0) } else { PlayerId(u8::from(kani::any::<bool>())) }, vampire)
            }
        };
        let id = b.state.create_object(CardId(0), controller, Zone::Battlefield, Some(2), Some(2));
        let o = b.state.get_object_mut(id).expect("just created");
        o.is_token = true;
        o.card_types = vec![crate::types::CardType::Creature];
        o.subtypes = Vec::with_capacity(1);
        if vampire {
            o.subtypes.push("Vampire".into());
            if controller == b.you {
                vampires += 1;
            }
        }
    }
    let make = super::activated::Ability { tap: true, ..super::activated::Ability::default() };
    let transform = super::activated::Ability {
        cost: crate::types::ManaCost::new(vec![crate::types::ManaSymbol::Colored(crate::types::Color::Black)]),
        ..super::activated::Ability::default()
    };
    let got = abilities(&b);
    if placed == 0 {
        assert!(got.is_empty());
    } else if placed == 1 && vampires >= 5 {
        assert!(got.len() == 2 && make.describes(&got[0]) && transform.describes(&got[1]));
    } else {
        assert!(got.len() == 1 && make.describes(&got[0]));
    }
    std::mem::forget((b, got, make, transform));
}

case_by_case! { bloodline_keeper_has_its_abilities;
    bloodline_keeper_gone_has_no_ability: (0, None),
    lord_of_lineage_makes_vampires: (2, None),
    bloodline_keeper_with_no_other_vampire_of_yours_only_makes_vampires: (1, Some(0)),
    bloodline_keeper_with_one_other_vampire_of_yours_only_makes_vampires: (1, Some(1)),
    bloodline_keeper_with_two_other_vampires_of_yours_only_makes_vampires: (1, Some(2)),
    bloodline_keeper_with_three_other_vampires_of_yours_only_makes_vampires: (1, Some(3)),
    bloodline_keeper_with_four_other_vampires_of_yours_may_transform: (1, Some(4)),
}

/// Bloodline Keeper's abilities resolving, either face up, on the
/// battlefield or gone: the first, a 2/2 black Vampire token with flying
/// for the player who activated it; the second, it transformed. Nothing
/// else. Which ability is `index`, a constant in each harness. Neither
/// reads the other creatures, which the proof above is over.
fn bloodline_keeper_resolves(index: usize) {
    let mut b = creature(Box::new(isd::bloodline_keeper::BloodlineKeeper));
    b.state.resolving_ability_activator = Some(b.you);
    let before = Snapshot::of(&b.state);
    let card = b.card();
    b.registry.get(card).expect("registered").resolve_activated_ability(&mut b.state, b.me, index, &[], &b.registry);
    if index == 0 {
        assert!(asked_for(&[Act::Tokens {
            n: 1, controller: b.you, power: 2, toughness: 2, colors: vec![crate::types::Color::Black],
            types: vec![crate::types::CardType::Creature], keywords: vec![crate::types::Keyword::Flying], subtypes: vec!["Vampire".into()],
        }]));
    } else {
        assert!(asked_for(&[Act::Transform(b.me)]));
    }
    assert!(Snapshot::of(&b.state) == before);
    std::mem::forget(b);
}

case_by_case! { bloodline_keeper_resolves;
    bloodline_keeper_makes_a_vampire: 0,
    bloodline_keeper_transforms: 1,
}

/// Ludevic's Test Subject // Ludevic's Abomination: front face up on the
/// battlefield, "{1}{U}: Put a hatchling counter on this creature. Then if
/// there are five or more hatchling counters on it, remove all of them
/// and transform it"; transformed or off the battlefield, no ability. Its
/// resolution is the proof below. Where it is and which face is up are
/// constants in each harness, as for Ulvenwald Mystics: whether it has
/// the ability is a list of abilities of a length they decide, and with
/// them the solver's, and the resolution in the same harness, the proof
/// ran out of memory at four and a half gigabytes in symbolic execution.
/// In its owner's graveyard for 0, front face up on the battlefield for
/// 1, and transformed there for 2.
fn ludevics_test_subject_placed(placed: u8) {
    let b = creature_at(Box::new(isd::ludevics_test_subject::LudevicsTestSubject), PlayerId(u8::from(kani::any::<bool>())), placed == 0, placed == 2);
    let got = abilities(&b);
    if placed == 1 {
        assert!(got.len() == 1 && super::activated::Ability {
            cost: crate::types::ManaCost::new(vec![
                crate::types::ManaSymbol::Generic(1), crate::types::ManaSymbol::Colored(crate::types::Color::Blue),
            ]),
            ..super::activated::Ability::default()
        }.describes(&got[0]));
    } else {
        assert!(got.is_empty());
    }
    std::mem::forget((b, got));
}

case_by_case! { ludevics_test_subject_placed;
    ludevics_test_subject_gone_has_no_ability: 0,
    ludevics_test_subject_face_up_has_its_ability: 1,
    ludevics_abomination_has_no_ability: 2,
}

card_proof! { adding_counters
    /// Ludevic's Test Subject's ability resolving: a hatchling counter on
    /// it, and then, with five or more there, all of them removed and it
    /// transformed. Over any number of hatchling counters from none to five
    /// already on it.
    fn ludevics_test_subject_hatches_at_five_counters() {
        let mut b = creature(Box::new(isd::ludevics_test_subject::LudevicsTestSubject));
        let had: u32 = kani::any();
        kani::assume(had <= 5);
        if had > 0 {
            b.state.get_object_mut(b.me).expect("on the board").counters.insert(crate::types::CounterType::Hatchling, had);
        }
        let on = b.state.get_object(b.me).expect("on the board").zone == Zone::Battlefield;
        let card = b.card();
        b.registry.get(card).expect("registered").resolve_activated_ability(&mut b.state, b.me, 0, &[], &b.registry);
        let now = had + u32::from(on);
        let add = Act::Counters(b.me, crate::types::CounterType::Hatchling, 1);
        if now >= 5 {
            assert!(asked_for(&[add, Act::RemoveCounters(b.me, crate::types::CounterType::Hatchling, now), Act::Transform(b.me)]));
        } else {
            assert!(asked_for(&[add]));
        }
        std::mem::forget(b);
    }
}
