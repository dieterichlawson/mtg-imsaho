//! What a player with priority is offered (`engine::legal`, CR 117.1,
//! 305, 307, 606): the timing a card may be cast at, from a hand or a
//! graveyard (`legal::casting::timing_allows`), and the loyalty abilities
//! and land plays offered (`legal::abilities::loyalty`). Whether a target
//! is legal for an ability is `targeting`'s proofs', any list here.

use crate::actions::{Action, Target};
use crate::cards::{CardBehavior, CardData, CardRegistry, LoyaltyAbilityDef, TargetRequirement};
use crate::ids::{ObjectId, PlayerId};
use crate::proofs::cards::card_proof;
use crate::state::GameState;
use crate::types::{CardType, CounterType, Keyword, Zone};

// The card's types are a list of up to seven, each asked of in turn.
card_proof! {
    unwind(8)
    /// A card may be cast now as its types and keywords allow (CR 117.1a,
    /// 307.1, 301.1, 302.1, 303.1, 306.1, 702.8a, 305.9): an instant or a
    /// card with flash at any time its caster has priority; a sorcery,
    /// creature, enchantment, artifact or planeswalker only when they
    /// could cast a sorcery; a land, or a card of none of these types,
    /// never. For every set of the seven types, with flash or without, at
    /// sorcery speed or not.
    fn a_card_may_be_cast_at_the_time_its_types_allow() {
        const TYPES: [CardType; 7] = [CardType::Instant, CardType::Sorcery, CardType::Creature,
            CardType::Enchantment, CardType::Artifact, CardType::Planeswalker, CardType::Land];
        let mut has = [false; 7];
        let mut card_types = Vec::with_capacity(7);
        for (i, t) in TYPES.iter().enumerate() {
            has[i] = kani::any();
            if has[i] {
                card_types.push(*t);
            }
        }
        let flash: bool = kani::any();
        let data = CardData { card_types, keywords: if flash { vec![Keyword::Flash] } else { Vec::new() }, ..CardData::default() };
        let speed: bool = kani::any();

        let allowed = crate::engine::legal::casting::timing_allows(&data, speed);

        assert!(allowed == (has[0] || flash || ((has[1] || has[2] || has[3] || has[4] || has[5]) && speed)));
        std::mem::forget(data);
    }
}

/// A planeswalker whose abilities are a +1 of no target (index 0) and a
/// −2 at a creature (index 1).
struct Walker;

impl CardBehavior for Walker {
    fn card_data(&self) -> CardData {
        CardData { name: "Walker".into(), card_types: vec![CardType::Planeswalker], ..CardData::default() }
    }
    fn loyalty_abilities(&self, _: &GameState, _: ObjectId) -> Vec<LoyaltyAbilityDef> {
        vec![
            LoyaltyAbilityDef { ability_index: 0, loyalty_change: 1, description: String::new(), target_requirement: None },
            LoyaltyAbilityDef { ability_index: 1, loyalty_change: -2, description: String::new(), target_requirement: Some(TargetRequirement::Creature) },
        ]
    }
}

/// A land, of `.0`'s name.
struct Land(&'static str);

impl CardBehavior for Land {
    fn card_data(&self) -> CardData {
        CardData { name: self.0.into(), card_types: vec![CardType::Land], ..CardData::default() }
    }
}

/// How many of the two targets the list offers: chosen the first time it
/// is asked, the same after.
static mut TARGETS: Option<usize> = None;

/// Stands in for `targeting::valid_targets_for_req`: the first none, one
/// or two of two targets.
#[allow(dead_code)] // named only by `kani::stub`
fn valid_targets(_: &GameState, _: PlayerId, _: ObjectId, _: &TargetRequirement, _: &dyn CardBehavior, _: &CardRegistry) -> Vec<Target> {
    // SAFETY: a proof runs on one thread, and only this function writes TARGETS.
    let n = *unsafe { &mut *std::ptr::addr_of_mut!(TARGETS) }.get_or_insert_with(|| {
        let n: usize = kani::any();
        kani::assume(n <= 2);
        n
    });
    [Target::Object(ObjectId(98)), Target::Object(ObjectId(99))][..n].to_vec()
}

card_proof! {
    #[kani::stub(crate::engine::targeting::valid_targets_for_req, valid_targets)]
    /// The loyalty abilities and land plays a player is offered (CR 606.3,
    /// 606.4, 118.3, 305.1-305.3): only when they could cast a sorcery;
    /// of each planeswalker they control whose loyalty ability has not
    /// been activated this turn, each ability whose cost they can pay (a
    /// minus of more than its loyalty they cannot), once for each target
    /// it may have, or once with none; then, with a land play left, one
    /// play of each land in their hand, the first of each card, and none
    /// of an opponent's. For a player either player, at sorcery speed or
    /// not, a planeswalker of theirs with 0 to 3 loyalty, its ability
    /// activated this turn or not, an opponent's planeswalker, three lands
    /// in their hand (two of one card), one in the opponent's, 0 to 2 land
    /// plays left, and the −2's targets none, one or two.
    fn a_player_is_offered_their_loyalty_abilities_and_land_plays_as_the_rules_allow() {
        let mut registry = CardRegistry::new();
        let walker = registry.register(Box::new(Walker));
        let land_a = registry.register(Box::new(Land("Land A")));
        let land_b = registry.register(Box::new(Land("Land B")));
        let mut state = crate::proofs::cards::new_game();
        let player = PlayerId(u8::from(kani::any::<bool>()));
        let other = state.opponent(player);
        state.priority_player = Some(player);
        let mine = state.create_object(walker, player, Zone::Battlefield, None, None);
        let loyalty: u32 = kani::any();
        kani::assume(loyalty <= 3);
        state.get_object_mut(mine).expect("just created").counters.insert(CounterType::Loyalty, loyalty);
        let used: bool = kani::any();
        if used {
            state.get_object_mut(mine).expect("just created").abilities_activated_this_turn.insert(999);
        }
        let theirs = state.create_object(walker, other, Zone::Battlefield, None, None);
        state.get_object_mut(theirs).expect("just created").counters.insert(CounterType::Loyalty, 3);
        let first = state.create_object(land_a, player, Zone::Hand, None, None);
        state.create_object(land_a, player, Zone::Hand, None, None);
        let third = state.create_object(land_b, player, Zone::Hand, None, None);
        state.create_object(land_b, other, Zone::Hand, None, None);
        let plays: u32 = kani::any();
        kani::assume(plays <= 2);
        state.get_player_mut(player).land_plays_remaining = plays;
        let speed: bool = kani::any();
        let ctx = crate::engine::legal::Ctx {
            state: &state,
            registry: &registry,
            player,
            prevent_artifact_abilities: false,
            is_sorcery_speed: speed,
            mana_sources: Vec::new(),
            demands: Vec::new(),
            casting_banned: Vec::new(),
        };
        let mut actions = Vec::with_capacity(8);

        crate::engine::legal::abilities::loyalty(&ctx, &mut actions);

        let mut expected = Vec::with_capacity(8);
        if speed && !used {
            expected.push(Action::ActivateLoyaltyAbility { object_id: mine, ability_index: 0, targets: Vec::new() });
            if loyalty >= 2 {
                // SAFETY: as in `valid_targets`.
                let n = unsafe { *std::ptr::addr_of!(TARGETS) }.expect("the targets listed");
                for t in [Target::Object(ObjectId(98)), Target::Object(ObjectId(99))].into_iter().take(n) {
                    expected.push(Action::ActivateLoyaltyAbility { object_id: mine, ability_index: 1, targets: vec![t] });
                }
            }
        }
        if speed && plays > 0 {
            expected.push(Action::PlayLand { object_id: first });
            expected.push(Action::PlayLand { object_id: third });
        }
        assert!(actions == expected);
        std::mem::forget((registry, state, actions, expected));
    }
}

/// Whether the permanent is new to its controller's control, and how many
/// other untapped creatures they control: any answer.
static mut SICK: Option<bool> = None;
static mut UNTAPPED: Option<usize> = None;

#[allow(dead_code)] // named only by `kani::stub`
fn has_summoning_sickness(_: &GameState, _: ObjectId, _: &CardRegistry) -> bool {
    // SAFETY: as in `valid_targets`, for SICK.
    *unsafe { &mut *std::ptr::addr_of_mut!(SICK) }.get_or_insert_with(kani::any::<bool>)
}

/// Stands in for `actions::abilities::tap_cost_candidates`: none to two
/// creatures.
#[allow(dead_code)] // named only by `kani::stub`
fn tap_cost_candidates(_: &GameState, _: PlayerId, _: ObjectId, _: &CardRegistry) -> Vec<ObjectId> {
    // SAFETY: as in `valid_targets`, for UNTAPPED.
    let n = *unsafe { &mut *std::ptr::addr_of_mut!(UNTAPPED) }.get_or_insert_with(|| {
        let n: usize = kani::any();
        kani::assume(n <= 2);
        n
    });
    [ObjectId(97), ObjectId(98)][..n].to_vec()
}

card_proof! {
    #[kani::stub(crate::state::GameState::has_summoning_sickness, has_summoning_sickness)]
    #[kani::stub(crate::engine::actions::abilities::tap_cost_candidates, tap_cost_candidates)]
    /// What restricts an activated ability beyond its mana
    /// (`legal::abilities::restrictions_allow`, CR 602.5b, 601.2h, 302.6):
    /// it may be activated now exactly when a `{T}` in its cost has the
    /// permanent untapped and not new to its controller's control; its
    /// counters cost has as many on the permanent; once each turn, it has
    /// not been activated this turn; a "tap N creatures" cost has N other
    /// untapped creatures to tap; and sorcery speed, it is that time. For
    /// every ability of each such cost and restriction, or none, and every
    /// permanent tapped or not, new or not, with 0 to 2 counters of the
    /// kind, the ability activated this turn or not.
    fn an_activated_ability_is_offered_exactly_as_its_costs_and_restrictions_allow() {
        let mut registry = CardRegistry::new();
        let land = registry.register(Box::new(Land("Land C")));
        let mut state = crate::proofs::cards::new_game();
        let player = PlayerId(u8::from(kani::any::<bool>()));
        let id = state.create_object(land, player, Zone::Battlefield, None, None);
        let (tapped, counters, used): (bool, u32, bool) = (kani::any(), kani::any(), kani::any());
        kani::assume(counters <= 2);
        let o = state.get_object_mut(id).expect("just created");
        o.tapped = tapped;
        o.counters.insert(CounterType::Study, counters);
        if used {
            o.abilities_activated_this_turn.insert(0);
        }
        let counter_cost: Option<u32> = kani::any::<bool>().then(kani::any);
        let tap_count: Option<usize> = kani::any::<bool>().then(kani::any);
        kani::assume(counter_cost.is_none_or(|n| n <= 2) && tap_count.is_none_or(|n| (1..=2).contains(&n)));
        let ab = crate::cards::ActivatedAbilityDef {
            ability_index: 0,
            description: String::new(),
            cost: crate::types::ManaCost::free(),
            requires_tap: kani::any(),
            sacrifice_cost: crate::cards::SacrificeCost::None,
            target_requirement: None,
            once_per_turn: kani::any(),
            sorcery_speed_only: kani::any(),
            counter_cost: counter_cost.map(|n| (CounterType::Study, n)),
            tap_cost: tap_count.map(|count| crate::cards::TapCreaturesCost { count }),
        };
        let speed: bool = kani::any();

        let allowed = crate::engine::legal::abilities::restrictions_allow(
            &state, state.get_object(id).expect("on the board"), &ab, player, speed, &registry);

        let tap_ok = !ab.requires_tap || (!tapped && !has_summoning_sickness(&state, id, &registry));
        let counters_ok = counter_cost.is_none_or(|n| counters >= n);
        let once_ok = !ab.once_per_turn || !used;
        let creatures_ok = tap_count.is_none_or(|n| tap_cost_candidates(&state, player, id, &registry).len() >= n);
        let speed_ok = !ab.sorcery_speed_only || speed;
        assert!(allowed == (tap_ok && counters_ok && once_ok && creatures_ok && speed_ok));
        std::mem::forget((registry, state, ab));
    }
}

/// What each of the board's objects (1 to 3) answers: whether it is a
/// creature, new to its controller, has defender, may attack, and may
/// block at all.
static mut OFFERED: [[Option<bool>; 5]; 3] = [[None; 5]; 3];

fn offered(id: ObjectId, k: usize) -> bool {
    let i = usize::try_from(id.0).expect("an id of the board");
    assert!((1..=3).contains(&i), "an object this proof does not make");
    // SAFETY: as in `valid_targets`, for OFFERED.
    let answers = unsafe { &mut *std::ptr::addr_of_mut!(OFFERED) };
    *answers[i - 1][k].get_or_insert_with(kani::any::<bool>)
}

#[allow(dead_code)] // each named only by `kani::stub`
fn creature(_: &GameState, id: ObjectId, _: &CardRegistry) -> bool {
    offered(id, 0)
}
#[allow(dead_code)]
fn sick(_: &GameState, id: ObjectId, _: &CardRegistry) -> bool {
    offered(id, 1)
}
#[allow(dead_code)]
fn defender(_: &GameState, id: ObjectId, keyword: Keyword, _: &CardRegistry) -> bool {
    assert!(keyword == Keyword::Defender, "a keyword the offer does not ask");
    offered(id, 2)
}
#[allow(dead_code)]
fn may_attack(_: &GameState, id: ObjectId, _: &CardRegistry) -> bool {
    offered(id, 3)
}
#[allow(dead_code)]
fn may_block(_: &GameState, id: ObjectId, _: &CardRegistry) -> bool {
    offered(id, 4)
}

/// Three objects, each either player's, on the battlefield or in a
/// graveyard, tapped or not.
fn three(registry: &mut CardRegistry) -> (GameState, [(PlayerId, bool, bool); 3]) {
    let blank = registry.register(Box::new(Land("Land D")));
    let mut state = crate::proofs::cards::new_game();
    let mut was = [(PlayerId(0), false, false); 3];
    for w in &mut was {
        *w = (PlayerId(u8::from(kani::any::<bool>())), kani::any(), kani::any());
        let id = state.create_object(blank, w.0, if w.1 { Zone::Battlefield } else { Zone::Graveyard }, Some(1), Some(1));
        state.get_object_mut(id).expect("just created").tapped = w.2;
    }
    (state, was)
}

card_proof! {
    #[kani::stub(crate::state::GameState::is_creature, creature)]
    #[kani::stub(crate::state::GameState::has_summoning_sickness, sick)]
    #[kani::stub(crate::state::GameState::has_keyword, defender)]
    #[kani::stub(crate::state::GameState::can_attack, may_attack)]
    /// The creatures a player is offered to attack with (`combat::
    /// eligible_attackers`, CR 508.1a, 302.6, 702.3b): exactly, in the
    /// board's order, those on the battlefield under their control that
    /// are untapped creatures, not new to their control, without defender,
    /// and that nothing stops attacking: the same as who may attack in a
    /// declaration (`attacking`). For three objects, each either player's,
    /// on the battlefield or not, tapped or not, and each question any
    /// answer.
    fn a_player_is_offered_exactly_the_creatures_that_may_attack() {
        let mut registry = CardRegistry::new();
        let (state, was) = three(&mut registry);
        let player = PlayerId(u8::from(kani::any::<bool>()));

        let eligible = crate::combat::eligible_attackers(&state, player, &registry);

        let mut expected = [ObjectId(0); 3];
        let mut n = 0;
        for (i, &(controller, on, tapped)) in was.iter().enumerate() {
            let id = ObjectId(i as u64 + 1);
            if on && controller == player && offered(id, 0) && !tapped && !offered(id, 1) && !offered(id, 2) && offered(id, 3) {
                expected[n] = id;
                n += 1;
            }
        }
        assert!(eligible.as_slice() == &expected[..n]);
        std::mem::forget((registry, state, eligible));
    }
}

card_proof! {
    #[kani::stub(crate::state::GameState::is_creature, creature)]
    #[kani::stub(crate::combat::can_block_at_all, may_block)]
    /// The creatures a player is offered to block with (`combat::
    /// eligible_blockers`, CR 509.1a): exactly, in the board's order, those
    /// on the battlefield under their control that are untapped creatures
    /// and that nothing stops blocking. For three objects as above.
    fn a_player_is_offered_exactly_the_creatures_that_may_block() {
        let mut registry = CardRegistry::new();
        let (state, was) = three(&mut registry);
        let player = PlayerId(u8::from(kani::any::<bool>()));

        let eligible = crate::combat::eligible_blockers(&state, player, &registry);

        let mut expected = [ObjectId(0); 3];
        let mut n = 0;
        for (i, &(controller, on, tapped)) in was.iter().enumerate() {
            let id = ObjectId(i as u64 + 1);
            if on && controller == player && offered(id, 0) && !tapped && offered(id, 4) {
                expected[n] = id;
                n += 1;
            }
        }
        assert!(eligible.as_slice() == &expected[..n]);
        std::mem::forget((registry, state, eligible));
    }
}
