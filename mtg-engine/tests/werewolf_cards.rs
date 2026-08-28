//! Tests for Innistrad werewolf double-faced cards.

mod common;
use common::*;
use mtg_engine::actions::{Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::types::*;

// ── Every werewolf, front face to back ────────────────────────────

/// Transforming does not make a new object (CR 711.5) — it turns the same
/// permanent's other face up, and every characteristic the game reads must
/// come from that face afterwards.
///
/// Seven of these were written one card at a time, each asserting the back
/// face's printed numbers ("should be 7/7", "should be 3/2"). Those numbers
/// are already written down in the card file, so restating them proved
/// nothing about the engine; what needs checking is that the *accessors*
/// follow the face. This asserts each card against its own back-face data,
/// for every werewolf in the set at once.
#[test]
fn every_werewolf_reads_its_back_face_after_transforming() {
    let reg = registry();

    let werewolves: Vec<String> = reg.all_names().iter()
        .filter(|name| {
            reg.get_id_by_name(name)
                .and_then(|id| reg.get(id))
                .and_then(|b| b.back_face_data().map(|back| {
                    let front = b.card_data();
                    front.subtypes.iter().chain(back.subtypes.iter()).any(|s| s == "Werewolf")
                }))
                .unwrap_or(false)
        })
        .map(|n| (*n).to_string())
        .collect();
    assert!(werewolves.len() >= 8,
        "only {} werewolves found — this sweep has stopped covering the set",
        werewolves.len());

    for name in &werewolves {
        let mut state = game_at_step(Step::Upkeep, P0);
        let id = named_permanent(&mut state, &reg, name, P0);
        let behavior = reg.get(state.get_object(id).unwrap().card_id).unwrap();
        let front = behavior.card_data();
        let back = behavior.back_face_data().unwrap();

        // Front face first, so a card that never transformed cannot pass by
        // having had the back face's characteristics all along.
        assert_eq!(state.effective_power(id, &reg), front.power,
            "{name} does not start with its front-face power");
        assert_eq!(state.effective_toughness(id, &reg), front.toughness,
            "{name} does not start with its front-face toughness");

        // "At the beginning of each upkeep, if no spells were cast last turn,
        // transform this creature." No spells were cast, so it transforms.
        fire_step_trigger(&mut state, Step::Upkeep, &reg);

        let obj = state.get_object(id).unwrap();
        assert!(obj.is_transformed, "{name} did not transform on a quiet upkeep");
        assert_eq!(obj.name, back.name, "{name} kept its front-face name");
        assert_eq!(state.effective_power(id, &reg), back.power,
            "{name} kept its front-face power after transforming");
        assert_eq!(state.effective_toughness(id, &reg), back.toughness,
            "{name} kept its front-face toughness after transforming");

        for kw in &back.keywords {
            assert!(state.has_keyword(id, *kw, &reg),
                "{} did not pick up its back face's {kw:?}", back.name);
        }
        for kw in &front.keywords {
            if !back.keywords.contains(kw) {
                assert!(!state.has_keyword(id, *kw, &reg),
                    "{} kept {kw:?} from its front face", back.name);
            }
        }
        for sub in &back.subtypes {
            assert!(state.has_subtype(id, sub, &reg),
                "{} did not pick up its back face's {sub:?} subtype", back.name);
        }
        for sub in &front.subtypes {
            if !back.subtypes.contains(sub) {
                assert!(!state.has_subtype(id, sub, &reg),
                    "{} kept {sub:?} from its front face", back.name);
            }
        }
    }
}

// ── Reckless Waif ─────────────────────────────────────────────────

#[test]
fn reckless_waif_transforms_on_the_games_first_upkeep() {
    // "At the beginning of each upkeep, if no spells were cast last turn,
    // transform this creature." There is no first-turn exception in the
    // oracle text: with no previous turn, no spells were cast in it. (Twelve
    // werewolves each carried a private copy of the condition, and every copy
    // had invented `&& !state.is_first_turn`.)
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);
    state.is_first_turn = true;
    let waif = named_permanent(&mut state, &reg, "Reckless Waif", P0);

    fire_step_trigger(&mut state, Step::Upkeep, &reg);

    let obj = state.get_object(waif).unwrap();
    assert!(obj.is_transformed,
        "no spells were cast last turn — there was no last turn — so the Waif transforms");
}

#[test]
fn reckless_waif_stays_human_when_spells_cast() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);
    state.num_spells_cast_last_turn.insert(P0, 1);
    let waif = named_permanent(&mut state, &reg, "Reckless Waif", P0);

    fire_step_trigger(&mut state, Step::Upkeep, &reg);

    let obj = state.get_object(waif).unwrap();
    assert!(!obj.is_transformed, "Should not transform when spells were cast");
}

#[test]
fn reckless_waif_transforms_back_when_two_spells_cast() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);
    let waif = named_permanent(&mut state, &reg, "Reckless Waif", P0);

    // Manually transform to werewolf side
    state.get_object_mut(waif).unwrap().is_transformed = true;
    state.get_object_mut(waif).unwrap().name = "Merciless Predator".into();

    // Set up: a player cast 2 spells last turn
    state.num_spells_cast_last_turn.insert(P0, 2);

    fire_step_trigger(&mut state, Step::Upkeep, &reg);

    let obj = state.get_object(waif).unwrap();
    assert!(!obj.is_transformed, "Should transform back when 2+ spells cast");
    assert_eq!(obj.name, "Reckless Waif");
    assert_eq!(state.effective_power(waif, &reg).unwrap(), 1);
}

// ── Gatstaf Shepherd ──────────────────────────────────────────────

#[test]
fn gatstaf_shepherd_loses_intimidate_on_transform_back() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);
    let shepherd = named_permanent(&mut state, &reg, "Gatstaf Shepherd", P0);

    // Transform to werewolf
    state.get_object_mut(shepherd).unwrap().is_transformed = true;
    assert!(state.has_keyword(shepherd, Keyword::Intimidate, &reg));

    // Set up transform back
    state.num_spells_cast_last_turn.insert(P1, 2);
    fire_step_trigger(&mut state, Step::Upkeep, &reg);

    assert!(!state.get_object(shepherd).unwrap().is_transformed);
    assert!(!state.has_keyword(shepherd, Keyword::Intimidate, &reg),
        "Gatstaf Shepherd should not have Intimidate");
}

/// CR 204.2: a transforming back face has no mana cost, so its colors come
/// from the color indicator printed beside its type line. Gatstaf Howler's is
/// green.
///
/// This is not decoration. The Howler's own intimidate reads "except by
/// artifact creatures and/or creatures that share a color with it", and with
/// the back face colorless nothing shares a color with it — only artifact
/// creatures could ever block, which is a strictly better card than the one
/// printed.
#[test]
fn gatstaf_howler_is_green_and_its_intimidate_lets_green_through() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);

    let howler = named_permanent(&mut state, &reg, "Gatstaf Shepherd", P0);
    mtg_engine::cards::helpers::apply_transform(&mut state, howler, &reg);
    assert_eq!(state.get_object(howler).unwrap().name, "Gatstaf Howler", "test setup");

    assert_eq!(state.colors_of(howler, &reg), vec![Color::Green],
        "the back face is green by its color indicator, not colorless");

    // A green creature shares a color and may block; a black one may not.
    let green = named_permanent(&mut state, &reg, "Darkthicket Wolf", P1);
    let black = named_permanent(&mut state, &reg, "Walking Corpse", P1);
    assert_eq!(state.colors_of(green, &reg), vec![Color::Green], "test setup");
    assert_eq!(state.colors_of(black, &reg), vec![Color::Black], "test setup");

    assert!(mtg_engine::combat::can_block_attacker(&state, green, howler, &reg),
        "a green creature shares a color with the Howler and can block it");
    assert!(!mtg_engine::combat::can_block_attacker(&state, black, howler, &reg),
        "a black creature shares no color with it and cannot");
}

/// And the front face, which does have a mana cost, still takes its color from
/// that cost — the indicator belongs to the back face only (CR 204.2).
#[test]
fn gatstaf_shepherd_is_green_from_its_mana_cost() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let shepherd = named_permanent(&mut state, &reg, "Gatstaf Shepherd", P0);

    assert_eq!(state.colors_of(shepherd, &reg), vec![Color::Green],
        "a {{1}}{{G}} cost makes the front face green");
}

// ── Village Ironsmith ─────────────────────────────────────────────

// ── Villagers of Estwald ──────────────────────────────────────────

// ── Hanweir Watchkeep ─────────────────────────────────────────────

#[test]
fn hanweir_watchkeep_loses_defender_gains_force_attack() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);
    let watchkeep = named_permanent(&mut state, &reg, "Hanweir Watchkeep", P0);

    // Front face: Defender
    assert!(state.has_keyword(watchkeep, Keyword::Defender, &reg));
    assert_eq!(state.effective_power(watchkeep, &reg).unwrap(), 1);

    fire_step_trigger(&mut state, Step::Upkeep, &reg);

    // Back face (Bane of Hanweir): 5/5, no Defender, attacks each combat
    assert!(state.get_object(watchkeep).unwrap().is_transformed);
    assert_eq!(state.effective_power(watchkeep, &reg).unwrap(), 5);
    assert!(!state.has_keyword(watchkeep, Keyword::Defender, &reg),
        "Bane of Hanweir should not have Defender");

    // ForceAttack is a continuous effect on the back face
    assert!(state.has_effect(watchkeep, &|e| matches!(e, ContinuousEffect::ForceAttack { .. }), &reg), "Bane of Hanweir should have ForceAttack");
}

/// Put Bane of Hanweir — the back face — onto `owner`'s battlefield, ready to
/// attack.
fn bane_of_hanweir(
    state: &mut mtg_engine::state::GameState,
    reg: &mtg_engine::cards::CardRegistry,
    owner: PlayerId,
) -> ObjectId {
    let id = named_permanent(state, reg, "Hanweir Watchkeep", owner);
    mtg_engine::cards::helpers::apply_transform(state, id, reg);
    assert_eq!(state.get_object(id).unwrap().name, "Bane of Hanweir", "test setup");
    id
}

/// "This creature attacks each combat if able." The existing test asserts the
/// effect is *present*; this one asserts it reaches combat — the Bane is an
/// attacker even though its controller declared no attackers at all.
#[test]
fn bane_of_hanweir_attacks_whether_you_declare_it_or_not() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let bane = bane_of_hanweir(&mut state, &reg, P0);
    // A creature with no such effect, to show the force is `OnSelf` and not
    // dragging the whole board in with it.
    let bystander = ready_creature(&mut state, P0, 2, 2);

    submit_declare_attackers(&mut state, &[], &reg);

    let combat = state.combat.as_ref().expect("combat exists");
    assert!(combat.attackers.contains_key(&bane),
        "the Bane attacks each combat if able, declared or not");
    assert!(!combat.attackers.contains_key(&bystander),
        "and the effect is on itself only — it does not drag other creatures in");
}

/// "…if **able**." A creature that cannot attack is not forced to: the Bane
/// summoning-sick, and the Bane already tapped.
#[test]
fn bane_of_hanweir_is_not_forced_when_it_cannot_attack() {
    for (label, sick, tapped) in [("summoning sick", true, false), ("tapped", false, true)] {
        let reg = registry();
        let mut state = game_at_step(Step::DeclareAttackers, P0);

        let bane = bane_of_hanweir(&mut state, &reg, P0);
        {
            let obj = state.get_object_mut(bane).unwrap();
            obj.summoning_sick = sick;
            obj.tapped = tapped;
        }

        submit_declare_attackers(&mut state, &[], &reg);

        let attacking = state.combat.as_ref()
            .is_some_and(|c| c.attackers.contains_key(&bane));
        assert!(!attacking,
            "a {label} creature is not able to attack, so 'attacks each combat \
             if able' does not force it");
    }
}

/// The front face has Defender, so it cannot attack — and a "must attack"
/// effect could not make it, either.
#[test]
fn hanweir_watchkeep_cannot_attack_behind_its_defender() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let watchkeep = named_permanent(&mut state, &reg, "Hanweir Watchkeep", P0);
    state.get_object_mut(watchkeep).unwrap().summoning_sick = false;
    assert!(state.has_keyword(watchkeep, Keyword::Defender, &reg), "test setup");

    assert!(!mtg_engine::combat::eligible_attackers(&state, P0, &reg).contains(&watchkeep),
        "a creature with defender is not an eligible attacker");

    submit_declare_attackers(&mut state, &[(watchkeep, P1)], &reg);

    let attacking = state.combat.as_ref()
        .is_some_and(|c| c.attackers.contains_key(&watchkeep));
    assert!(!attacking, "and declaring it anyway does not make it one");
}

// ── Tormented Pariah ──────────────────────────────────────────────

// ── Grizzled Outcasts ─────────────────────────────────────────────

/// The plainest werewolf in the set: a vanilla body on both faces, so what
/// there is to get wrong is the body itself and the colour it keeps.
///
/// Its section here was an empty header — the card was only ever exercised
/// incidentally, by the two tests that flip several werewolves at once.
#[test]
fn grizzled_outcasts_is_a_green_4_4_that_becomes_a_green_7_7() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let outcasts = named_permanent(&mut state, &reg, "Grizzled Outcasts", P0);
    assert_eq!(state.effective_power(outcasts, &reg), Some(4));
    assert_eq!(state.effective_toughness(outcasts, &reg), Some(4));
    assert_eq!(state.colors_of(outcasts, &reg), vec![Color::Green],
        "green from its {{4}}{{G}} cost");
    assert!(state.has_subtype(outcasts, "Human", &reg), "Human on the front face");

    mtg_engine::cards::helpers::apply_transform(&mut state, outcasts, &reg);

    assert_eq!(state.get_object(outcasts).unwrap().name, "Krallenhorde Wantons");
    assert_eq!(state.effective_power(outcasts, &reg), Some(7));
    assert_eq!(state.effective_toughness(outcasts, &reg), Some(7));
    // CR 204.2: the back face has no mana cost, so this can only come from the
    // colour indicator. Without one it would be colourless.
    assert_eq!(state.colors_of(outcasts, &reg), vec![Color::Green],
        "green from its colour indicator");
    assert!(!state.has_subtype(outcasts, "Human", &reg),
        "the back face is a Werewolf and no longer a Human");
    assert!(state.has_subtype(outcasts, "Werewolf", &reg));
}

// ── Mayor of Avabruck ─────────────────────────────────────────────

#[test]
fn mayor_of_avabruck_buffs_humans_on_front_face() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let mayor = named_permanent(&mut state, &reg, "Mayor of Avabruck", P0);
    let human = named_permanent(&mut state, &reg, "Reckless Waif", P0);

    // Mayor gives other Humans +1/+1
    assert_eq!(state.effective_power(human, &reg).unwrap(), 2, "Reckless Waif should get +1/+1 from Mayor");
    assert_eq!(state.effective_toughness(human, &reg).unwrap(), 2);
    // Mayor doesn't buff itself
    assert_eq!(state.effective_power(mayor, &reg).unwrap(), 1);
}

#[test]
fn mayor_of_avabruck_transforms_and_buffs_werewolves() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);
    let mayor = named_permanent(&mut state, &reg, "Mayor of Avabruck", P0);
    let other_wolf = named_permanent(&mut state, &reg, "Reckless Waif", P0);

    // Transform both
    fire_step_trigger(&mut state, Step::Upkeep, &reg);

    // Mayor is now Howlpack Alpha (3/3), gives other Werewolves/Wolves +1/+1
    assert!(state.get_object(mayor).unwrap().is_transformed);
    assert_eq!(state.effective_power(mayor, &reg).unwrap(), 3);

    // Other werewolf should get the buff: Merciless Predator is 3/2 + 1/1 = 4/3
    assert!(state.get_object(other_wolf).unwrap().is_transformed);
    assert_eq!(state.effective_power(other_wolf, &reg).unwrap(), 4,
        "Merciless Predator should get +1/+1 from Howlpack Alpha");
    assert_eq!(state.effective_toughness(other_wolf, &reg).unwrap(), 3);
}

#[test]
fn howlpack_alpha_creates_wolf_token_on_end_step() {
    let reg = registry();
    let mut state = game_at_step(Step::EndStep, P0);
    let mayor = named_permanent(&mut state, &reg, "Mayor of Avabruck", P0);
    mtg_engine::cards::helpers::apply_transform(&mut state, mayor, &reg);
    assert_eq!(state.name_of(mayor, &reg), "Howlpack Alpha");

    fire_step_trigger(&mut state, Step::EndStep, &reg);

    // Should have created a 2/2 Wolf token
    assert_eq!(count_tokens_named(&state, "Wolf Token"), 1, "Howlpack Alpha should create one Wolf token");
    let wolf = find_token_named(&state, "Wolf Token").unwrap();
    assert_eq!(state.get_object(wolf).unwrap().power, Some(2));
    assert_eq!(state.get_object(wolf).unwrap().toughness, Some(2));
}

#[test]
fn howlpack_alpha_does_not_create_token_on_front_face() {
    let reg = registry();
    let mut state = game_at_step(Step::EndStep, P0);
    let _mayor = named_permanent(&mut state, &reg, "Mayor of Avabruck", P0);
    // Front face (not transformed)

    fire_step_trigger(&mut state, Step::EndStep, &reg);

    assert_eq!(count_tokens_named(&state, "Wolf Token"), 0,
        "Front face Mayor should not create Wolf tokens");
}

#[test]
fn howlpack_alpha_does_not_create_token_on_opponents_end_step() {
    let reg = registry();
    // Active player is P1 (opponent), but Mayor belongs to P0
    let mut state = game_at_step(Step::EndStep, P1);
    let mayor = named_permanent(&mut state, &reg, "Mayor of Avabruck", P0);
    // Transform to Howlpack Alpha
    state.get_object_mut(mayor).unwrap().is_transformed = true;
    state.get_object_mut(mayor).unwrap().name = "Howlpack Alpha".into();

    fire_step_trigger(&mut state, Step::EndStep, &reg);

    // Should NOT create a Wolf token on opponent's end step
    assert_eq!(count_tokens_named(&state, "Wolf Token"), 0,
        "Howlpack Alpha should not create Wolf tokens on opponent's end step");
}

#[test]
fn mayor_of_avabruck_does_not_transform_when_a_spell_was_cast() {
    // The condition is about spells, not about which turn it is: a single
    // spell cast last turn keeps the front face up, on turn one or any other.
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);
    state.is_first_turn = true;
    state.num_spells_cast_last_turn.insert(P0, 1);
    let mayor = named_permanent(&mut state, &reg, "Mayor of Avabruck", P0);

    fire_step_trigger(&mut state, Step::Upkeep, &reg);

    let obj = state.get_object(mayor).unwrap();
    assert!(!obj.is_transformed,
        "a spell was cast last turn, so Mayor of Avabruck stays human");
}

#[test]
fn howlpack_alpha_werewolf_wolf_creature_gets_only_plus_one() {
    // Ruling [2025-01-24]: A creature that is both a Werewolf and a Wolf will
    // only get +1/+1 from Howlpack Alpha's first ability.
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let mayor = named_permanent(&mut state, &reg, "Mayor of Avabruck", P0);
    // Transform to Howlpack Alpha
    state.get_object_mut(mayor).unwrap().is_transformed = true;
    state.get_object_mut(mayor).unwrap().name = "Howlpack Alpha".into();

    // Create a token that is both a Werewolf and a Wolf (2/2 base)
    let dual_id = state.create_token_with_subtypes(
        "Test Werewolf Wolf",
        P0,
        2, 2,
        vec![Color::Green],
        vec![CardType::Creature],
        vec![],
        vec!["Werewolf".into(), "Wolf".into()],
        &reg,
    )[0];

    // The creature should get exactly +1/+1 (not +2/+2) from Howlpack Alpha
    assert_eq!(state.effective_power(dual_id, &reg).unwrap(), 3,
        "Werewolf+Wolf creature should get only +1/+1 from Howlpack Alpha (not +2/+2)");
    assert_eq!(state.effective_toughness(dual_id, &reg).unwrap(), 3,
        "Werewolf+Wolf creature should get only +1/+1 from Howlpack Alpha (not +2/+2)");
}

// ── Daybreak Ranger ───────────────────────────────────────────────

#[test]
fn daybreak_ranger_has_activated_ability_on_front_face() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let ranger = named_permanent(&mut state, &reg, "Daybreak Ranger", P0);

    let abilities = reg.get(state.get_object(ranger).unwrap().card_id).unwrap()
        .activated_abilities(&state, ranger, &reg);
    assert_eq!(abilities.len(), 1);
    assert!(abilities[0].description.contains("flying"));
}

#[test]
fn nightfall_predator_has_fight_ability() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let ranger = named_permanent(&mut state, &reg, "Daybreak Ranger", P0);
    state.get_object_mut(ranger).unwrap().is_transformed = true;

    let abilities = reg.get(state.get_object(ranger).unwrap().card_id).unwrap()
        .activated_abilities(&state, ranger, &reg);
    assert_eq!(abilities.len(), 1);
    assert!(abilities[0].description.contains("Fight"));
}

#[test]
fn nightfall_predator_can_fight_own_creature() {
    // Per oracle: "{R}, {T}: This creature fights target creature." — no controller restriction.
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let ranger = named_permanent(&mut state, &reg, "Daybreak Ranger", P0);
    state.get_object_mut(ranger).unwrap().is_transformed = true;
    state.get_object_mut(ranger).unwrap().name = "Nightfall Predator".into();

    // Own creature to fight.
    let own_creature = ready_creature(&mut state, P0, 2, 2);

    // Give mana for the {R} cost.
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 1);

    let new_state = activate(&state, &reg, ranger, 0, vec![Target::Object(own_creature)]);

    // Both creatures should have dealt damage to each other.
    // Nightfall Predator is 4/4, own creature is 2/2.
    assert_eq!(new_state.get_object(own_creature).unwrap().damage_marked, 4,
        "Own creature should take 4 damage from Nightfall Predator");
    assert_eq!(new_state.get_object(ranger).unwrap().damage_marked, 2,
        "Nightfall Predator should take 2 damage from own creature");
}

// ── Instigator Gang ───────────────────────────────────────────────

#[test]
fn instigator_gang_transforms_and_gains_trample() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);
    let gang = named_permanent(&mut state, &reg, "Instigator Gang", P0);

    assert!(!state.has_keyword(gang, Keyword::Trample, &reg));

    fire_step_trigger(&mut state, Step::Upkeep, &reg);

    assert!(state.get_object(gang).unwrap().is_transformed);
    assert_eq!(state.effective_power(gang, &reg).unwrap(), 5);
    assert!(state.has_keyword(gang, Keyword::Trample, &reg),
        "Wildblood Pack should have Trample");
}

#[test]
fn instigator_gang_buffs_itself_when_attacking() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);
    let gang = named_permanent(&mut state, &reg, "Instigator Gang", P0);

    // Base power is 2.
    assert_eq!(state.effective_power(gang, &reg).unwrap(), 2);

    // Declare Instigator Gang as attacker — it should buff itself +1/+0.
    submit_declare_attackers(&mut state, &[(gang, P1)], &reg);

    // Should be 2 base + 1 from own buff = 3.
    assert_eq!(state.effective_power(gang, &reg).unwrap(), 3,
        "Instigator Gang should buff itself when attacking (+1/+0)");
}

#[test]
fn instigator_gang_buffs_other_attackers_you_control() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);
    let gang = named_permanent(&mut state, &reg, "Instigator Gang", P0);
    let ally = ready_creature(&mut state, P0, 3, 3);

    // Declare both as attackers.
    submit_declare_attackers(&mut state, &[(gang, P1), (ally, P1)], &reg);

    // Gang: 2 + 1 = 3 (buffs itself too).
    assert_eq!(state.effective_power(gang, &reg).unwrap(), 3,
        "Instigator Gang should be 3 power when attacking");
    // Ally: 3 + 1 = 4 (buffed by Instigator Gang).
    assert_eq!(state.effective_power(ally, &reg).unwrap(), 4,
        "Ally should get +1/+0 from Instigator Gang");
}

#[test]
fn instigator_gang_does_not_buff_opponent_attackers() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P1);
    let _gang = named_permanent(&mut state, &reg, "Instigator Gang", P0);
    let enemy = ready_creature(&mut state, P1, 2, 2);

    // Opponent's creature attacks.
    submit_declare_attackers(&mut state, &[(enemy, P0)], &reg);

    // Enemy should NOT be buffed (different controller).
    assert_eq!(state.effective_power(enemy, &reg).unwrap(), 2,
        "Opponent's creature should not get Instigator Gang's buff");
}

#[test]
fn wildblood_pack_buffs_itself_plus_3() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);
    let gang = named_permanent(&mut state, &reg, "Instigator Gang", P0);

    // Transform to Wildblood Pack.
    state.get_object_mut(gang).unwrap().is_transformed = true;
    assert_eq!(state.effective_power(gang, &reg).unwrap(), 5);

    // Declare Wildblood Pack as attacker — should buff itself +3/+0.
    submit_declare_attackers(&mut state, &[(gang, P1)], &reg);

    // Should be 5 base + 3 from own buff = 8.
    assert_eq!(state.effective_power(gang, &reg).unwrap(), 8,
        "Wildblood Pack should buff itself +3/+0 when attacking");
}

// ── Ulvenwald Mystics ─────────────────────────────────────────────

#[test]
fn ulvenwald_mystics_transforms_and_gains_regenerate() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);
    let mystics = named_permanent(&mut state, &reg, "Ulvenwald Mystics", P0);

    // Front face: no activated abilities
    let front_abilities = reg.get(state.get_object(mystics).unwrap().card_id).unwrap()
        .activated_abilities(&state, mystics, &reg);
    assert_eq!(front_abilities.len(), 0, "Front face should have no activated abilities");

    fire_step_trigger(&mut state, Step::Upkeep, &reg);

    // Back face: {G}: Regenerate
    assert!(state.get_object(mystics).unwrap().is_transformed);
    assert_eq!(state.effective_power(mystics, &reg).unwrap(), 5);
    let back_abilities = reg.get(state.get_object(mystics).unwrap().card_id).unwrap()
        .activated_abilities(&state, mystics, &reg);
    assert_eq!(back_abilities.len(), 1, "Ulvenwald Primordials should have regenerate ability");
    assert!(back_abilities[0].description.contains("Regenerate"));
}

/// Ruling: "You can regenerate Ulvenwald Primordials in response to the
/// triggered ability that would transform it. If you do, the regeneration
/// shield will apply to Ulvenwald Mystics that turn."
///
/// The shield outlives the transform because transforming does not make a new
/// object (CR 712.8) — it is not a zone change, so none of the leave-the-
/// battlefield cleanup runs. This is the card's only published ruling and had
/// no test.
#[test]
fn a_regeneration_shield_survives_transforming_back_into_ulvenwald_mystics() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let mystics = named_permanent(&mut state, &reg, "Ulvenwald Mystics", P0);
    mtg_engine::cards::helpers::apply_transform(&mut state, mystics, &reg);
    assert_eq!(state.name_of(mystics, &reg), "Ulvenwald Primordials");

    // Regenerate the Primordials — this is the "in response" part of the ruling.
    state.get_player_mut(P0).mana_pool.add(ManaType::Green, 1);
    let mut state = activate(&state, &reg, mystics, 0, vec![]);
    assert_eq!(state.get_object(mystics).unwrap().regeneration_shields, 1);

    // Then the transform ability resolves and it becomes Mystics again.
    mtg_engine::cards::helpers::apply_transform(&mut state, mystics, &reg);
    assert_eq!(state.name_of(mystics, &reg), "Ulvenwald Mystics");
    assert_eq!(state.get_object(mystics).unwrap().regeneration_shields, 1,
        "the shield came with it — transforming is not a zone change");

    // And it still works, on the front face, which has no regenerate ability
    // of its own.
    let result = mtg_engine::destruction::try_destroy(&mut state, mystics, &reg);
    assert_eq!(result, mtg_engine::destruction::DestroyResult::Regenerated,
        "the shield applies to Ulvenwald Mystics that turn");
    assert_eq!(state.get_object(mystics).unwrap().zone, Zone::Battlefield);
}

/// CR 701.15: a regeneration shield lasts until end of turn. An unused one does
/// not bank into the next turn.
#[test]
fn an_unused_regeneration_shield_does_not_survive_the_turn() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let mystics = named_permanent(&mut state, &reg, "Ulvenwald Mystics", P0);
    mtg_engine::cards::helpers::apply_transform(&mut state, mystics, &reg);
    state.get_player_mut(P0).mana_pool.add(ManaType::Green, 1);
    let mut state = activate(&state, &reg, mystics, 0, vec![]);
    assert_eq!(state.get_object(mystics).unwrap().regeneration_shields, 1);

    advance_to_next_turn(&mut state, &reg);

    assert_eq!(state.get_object(mystics).unwrap().regeneration_shields, 0,
        "the shield expired at end of turn");
}

// ── Kruin Outlaw ──────────────────────────────────────────────────

#[test]
fn kruin_outlaw_transforms_gains_double_strike_and_menace() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);
    let outlaw = named_permanent(&mut state, &reg, "Kruin Outlaw", P0);

    // Front: first strike, no double strike or menace
    assert!(state.has_keyword(outlaw, Keyword::FirstStrike, &reg));
    assert!(!state.has_keyword(outlaw, Keyword::DoubleStrike, &reg));
    assert!(!state.has_keyword(outlaw, Keyword::Menace, &reg));

    fire_step_trigger(&mut state, Step::Upkeep, &reg);

    // Back: double strike, "can't be blocked except by two or more creatures" for
    // all Werewolves (implemented as MinimumBlockers, not as Keyword::Menace).
    assert!(state.get_object(outlaw).unwrap().is_transformed);
    assert_eq!(state.effective_power(outlaw, &reg).unwrap(), 3);
    assert!(state.has_keyword(outlaw, Keyword::DoubleStrike, &reg),
        "Terror of Kruin Pass should have Double Strike");
    // The blocking restriction is enforced via MinimumBlockers continuous effect,
    // not as a menace keyword. See kruin_outlaw.rs tests for blocking validation.
}

// ── Subtype tests ─────────────────────────────────────────────────

#[test]
fn transformed_werewolf_has_werewolf_subtype_not_human() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let waif = named_permanent(&mut state, &reg, "Reckless Waif", P0);

    // Front face: Human subtype
    assert!(state.matches_filter(waif,
        &CreatureFilter::HasSubtype("Human".into()), P0, &reg),
        "Front face should have Human subtype");

    // Transform
    state.get_object_mut(waif).unwrap().is_transformed = true;

    // Back face: Werewolf subtype, not Human
    assert!(state.matches_filter(waif,
        &CreatureFilter::HasSubtype("Werewolf".into()), P0, &reg),
        "Back face should have Werewolf subtype");
    assert!(!state.matches_filter(waif,
        &CreatureFilter::HasSubtype("Human".into()), P0, &reg),
        "Back face should not have Human subtype");
}

// ── All werewolves transform together ─────────────────────────────

#[test]
fn multiple_werewolves_transform_on_same_upkeep() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);
    let waif = named_permanent(&mut state, &reg, "Reckless Waif", P0);
    let shepherd = named_permanent(&mut state, &reg, "Gatstaf Shepherd", P0);
    let outcasts = named_permanent(&mut state, &reg, "Grizzled Outcasts", P0);

    // No spells cast last turn, all should transform
    fire_step_trigger(&mut state, Step::Upkeep, &reg);

    assert!(state.get_object(waif).unwrap().is_transformed, "Reckless Waif should transform");
    assert!(state.get_object(shepherd).unwrap().is_transformed, "Gatstaf Shepherd should transform");
    assert!(state.get_object(outcasts).unwrap().is_transformed, "Grizzled Outcasts should transform");
}

#[test]
fn multiple_werewolves_transform_back_together() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);
    let waif = named_permanent(&mut state, &reg, "Reckless Waif", P0);
    let shepherd = named_permanent(&mut state, &reg, "Gatstaf Shepherd", P0);

    // Manually transform to werewolf side
    state.get_object_mut(waif).unwrap().is_transformed = true;
    state.get_object_mut(shepherd).unwrap().is_transformed = true;

    // A player cast 2 spells last turn
    state.num_spells_cast_last_turn.insert(P1, 2);

    fire_step_trigger(&mut state, Step::Upkeep, &reg);

    assert!(!state.get_object(waif).unwrap().is_transformed, "Should transform back");
    assert!(!state.get_object(shepherd).unwrap().is_transformed, "Should transform back");
}

// ── Werewolf does not transform when shouldn't ────────────────────

#[test]
fn werewolf_side_stays_if_one_spell_cast() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);
    let waif = named_permanent(&mut state, &reg, "Reckless Waif", P0);
    state.get_object_mut(waif).unwrap().is_transformed = true;

    // Only 1 spell cast last turn: not enough to transform back
    state.num_spells_cast_last_turn.insert(P0, 1);

    fire_step_trigger(&mut state, Step::Upkeep, &reg);

    assert!(state.get_object(waif).unwrap().is_transformed,
        "Werewolf should stay transformed with only 1 spell cast");
}

#[test]
fn human_side_stays_if_any_spell_cast() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);
    let waif = named_permanent(&mut state, &reg, "Reckless Waif", P0);

    // Opponent cast 1 spell last turn
    state.num_spells_cast_last_turn.insert(P1, 1);

    fire_step_trigger(&mut state, Step::Upkeep, &reg);

    assert!(!state.get_object(waif).unwrap().is_transformed,
        "Human should stay on front face when any spell was cast last turn");
}

// -------------------------------------------------------------------------
// From the bug-audit files, re-filed by the rule each one exercises.
// -------------------------------------------------------------------------

/// Bug: `num_spells_cast_this_turn` is never incremented when spells are cast.
/// This breaks werewolf transform conditions which check `num_spells_cast_last_turn`.
/// If no spells are ever counted, the "no spells cast last turn" condition
/// is always true and werewolves would transform every upkeep.
#[test]
fn bug_num_spells_cast_this_turn_never_incremented() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Record spells cast before
    let cast_before: u32 = state.num_spells_cast_this_turn.values().sum();

    // Cast a spell
    let bolt = castable_spell(&mut state, &registry, "Lightning Bolt", P0);
    let target = ready_creature(&mut state, P1, 3, 3);
    state = cast_and_resolve(&state, &registry, bolt, vec![Target::Object(target)]);

    // num_spells_cast_this_turn should have been incremented
    let cast_after: u32 = state.num_spells_cast_this_turn.values().sum();

    // BUG: Count is still 0 because submit_action never updates num_spells_cast_this_turn
    assert!(cast_after > cast_before,
        "num_spells_cast_this_turn should increment when a spell is cast. Before: {cast_before}, After: {cast_after}");
}

// -------------------------------------------------------------------------
// Kruin Outlaw / Terror of Kruin Pass
// -------------------------------------------------------------------------

/// Put Terror of Kruin Pass — the back face — onto `owner`'s battlefield.
fn terror_of_kruin_pass(
    state: &mut mtg_engine::state::GameState,
    reg: &mtg_engine::cards::CardRegistry,
    owner: PlayerId,
) -> ObjectId {
    let id = named_permanent(state, reg, "Kruin Outlaw", owner);
    // Through the engine's own transform, so the object ends up in the state a
    // real transform leaves it in rather than one the test invented.
    mtg_engine::cards::helpers::apply_transform(state, id, reg);
    assert_eq!(state.get_object(id).unwrap().name, "Terror of Kruin Pass", "test setup");
    id
}

/// Declare `blockers` against `attacker` and report how many the engine kept.
fn blockers_accepted(
    state: &mut mtg_engine::state::GameState,
    reg: &mtg_engine::cards::CardRegistry,
    attacker: ObjectId,
    defender: PlayerId,
    blockers: &[ObjectId],
) -> usize {
    attacks_unblocked(state, attacker, defender);

    let pairs: Vec<_> = blockers.iter().map(|b| (*b, attacker)).collect();
    submit_declare_blockers(state, defender, &pairs, reg);
    state.combat.as_ref().unwrap().blocker_assignments[&attacker].len()
}

/// Who the menace reaches, and who it does not.
///
/// The negative rows are what separate this from a Terror that gave menace to
/// every creature on the battlefield — including the opponent's, which would
/// be helping them.
#[test]
fn terror_of_kruin_pass_gives_menace_to_your_werewolves_only() {
    // (whose creature, its subtype, does one blocker suffice)
    const CASES: &[(bool, &str, bool, &str)] = &[
        (true, "Werewolf", false, "your Werewolf needs two blockers"),
        (true, "Human", true, "a non-Werewolf of yours does not"),
        (false, "Werewolf", true, "and neither does an opponent's Werewolf"),
    ];

    for &(yours, subtype, one_blocker_is_enough, why) in CASES {
        let reg = registry();
        let (attacker_owner, defender) = if yours { (P0, P1) } else { (P1, P0) };
        let mut state = game_at_step(Step::DeclareBlockers, attacker_owner);
        state.active_player = attacker_owner;

        terror_of_kruin_pass(&mut state, &reg, P0);

        let attacker = ready_creature(&mut state, attacker_owner, 3, 3);
        state.get_object_mut(attacker).unwrap().subtypes = vec![subtype.into()];
        let blocker = ready_creature(&mut state, defender, 2, 2);

        let accepted = blockers_accepted(&mut state, &reg, attacker, defender, &[blocker]);
        assert_eq!(accepted == 1, one_blocker_is_enough, "{why}");
    }
}

/// "Werewolves you control" includes the Terror itself, so it needs two
/// blockers too — and two is enough.
#[test]
fn terror_of_kruin_pass_needs_two_blockers_itself() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);

    let terror = terror_of_kruin_pass(&mut state, &reg, P0);
    let first = ready_creature(&mut state, P1, 2, 2);
    let second = ready_creature(&mut state, P1, 2, 2);

    assert_eq!(blockers_accepted(&mut state, &reg, terror, P1, &[first]), 0,
        "one blocker is turned away");
    assert_eq!(blockers_accepted(&mut state, &reg, terror, P1, &[first, second]), 2,
        "two are accepted — the restriction is 'except by two or more', not \
         'can't be blocked'");
}

/// Ruling: "If Kruin Outlaw somehow transforms after blockers have been
/// declared but before combat ends, any Werewolves you control that are
/// blocked by a single creature will remain blocked."
///
/// Menace is a restriction on *declaring* blockers (CR 509.1b), so a block
/// already declared is not re-examined. CR 509.2 also makes blocked-ness
/// permanent for the combat.
#[test]
fn transforming_after_blockers_leaves_a_single_blocker_in_place() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);

    // The Outlaw is on its front face, so nothing has menace yet.
    let outlaw = named_permanent(&mut state, &reg, "Kruin Outlaw", P0);
    assert!(!state.get_object(outlaw).unwrap().is_transformed, "test setup");

    let pack_mate = ready_creature(&mut state, P0, 3, 3);
    state.get_object_mut(pack_mate).unwrap().subtypes = vec!["Werewolf".into()];
    let blocker = ready_creature(&mut state, P1, 2, 2);

    assert_eq!(blockers_accepted(&mut state, &reg, pack_mate, P1, &[blocker]), 1,
        "test precondition: one blocker is legal before the transform");

    // Now it flips, and every Werewolf P0 controls gains menace.
    mtg_engine::cards::helpers::apply_transform(&mut state, outlaw, &reg);
    assert!(state.has_keyword(pack_mate, Keyword::Menace, &reg),
        "test precondition: the Werewolf has menace now");

    let combat = state.combat.as_ref().unwrap();
    assert_eq!(combat.blocker_assignments[&pack_mate], vec![blocker],
        "the block was declared legally and menace does not undo it");
    assert!(combat.blocked_attackers.contains(&pack_mate),
        "CR 509.2: it stays a blocked creature for the rest of this combat");
}

/// The restriction is the Menace keyword, so it shows up in `has_keyword` and
/// not only in the blocker validation.
#[test]
fn terror_of_kruin_pass_grants_menace_as_a_keyword() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let terror = terror_of_kruin_pass(&mut state, &reg, P0);
    let wolf = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(wolf).unwrap().subtypes = vec!["Werewolf".into()];
    let human = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(human).unwrap().subtypes = vec!["Human".into()];

    assert!(state.has_keyword(terror, Keyword::Menace, &reg), "the Terror itself");
    assert!(state.has_keyword(wolf, Keyword::Menace, &reg), "another of your Werewolves");
    assert!(!state.has_keyword(human, Keyword::Menace, &reg), "but not a non-Werewolf");
}

/// CR 113.7a: a triggered ability on the stack exists independently of its
/// source. "At the beginning of your end step, create a 2/2 green Wolf creature
/// token" says nothing about Howlpack Alpha, so killing the Alpha in response to
/// its own trigger does not stop the Wolf.
///
/// The handler used to require the source to still be on the battlefield *and*
/// still transformed — and leaving the battlefield clears `is_transformed`, so a
/// dead Alpha failed both checks and the token silently never appeared.
#[test]
fn howlpack_alphas_wolf_arrives_even_if_the_alpha_dies_in_response() {
    let reg = registry();
    let mut state = game_at_step(Step::EndStep, P0);
    let mayor = named_permanent(&mut state, &reg, "Mayor of Avabruck", P0);
    mtg_engine::cards::helpers::apply_transform(&mut state, mayor, &reg);

    // The trigger goes on the stack...
    state.events.push(mtg_engine::events::GameEvent::StepStarted { step: Step::EndStep });
    mtg_engine::triggers::collect_triggers(&mut state, &reg);
    assert!(!state.stack.is_empty(), "the end-step trigger is on the stack");

    // ...and the Alpha is killed in response.
    state.move_object(mayor, Zone::Graveyard, &reg);

    mtg_engine::triggers::resolve_next_trigger(&mut state, &reg);

    assert_eq!(count_tokens_named(&state, "Wolf Token"), 1,
        "the Wolf is created even though its source is gone");
    let wolf = find_token_named(&state, "Wolf Token").unwrap();
    assert_eq!(state.get_object(wolf).unwrap().controller, P0,
        "under the player who controlled the Alpha when it triggered");
}

/// CR 603.4 + CR 712.8: an intervening-if re-checked on resolution is the
/// condition of the ability that *triggered*, and that ability belongs to one
/// face. Transforming in between does not hand the trigger the other face's
/// condition.
///
/// Moonmist ("Transform all Humans") in response to a front-face Werewolf's
/// upkeep trigger is the reachable case. Mayor of Avabruck is a Human, so
/// Moonmist flips it to Howlpack Alpha. The trigger on the stack is still the
/// front face's — "if no spells were cast last turn, transform this creature" —
/// and casting Moonmist this turn does not change what was cast *last* turn, so
/// it resolves and flips the Alpha back.
///
/// Reading the current face instead tested the back face's condition ("a player
/// cast two or more spells last turn"), found it false, and did nothing.
#[test]
fn a_werewolf_trigger_keeps_its_own_faces_condition_across_a_transform() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);
    // Nobody cast anything last turn: the front face's condition holds.
    state.num_spells_cast_last_turn.clear();

    let mayor = named_permanent(&mut state, &reg, "Mayor of Avabruck", P0);
    assert!(!state.get_object(mayor).unwrap().is_transformed);

    // The upkeep trigger goes on the stack, from the front face.
    state.events.push(mtg_engine::events::GameEvent::StepStarted { step: Step::Upkeep });
    mtg_engine::triggers::collect_triggers(&mut state, &reg);
    assert!(!state.stack.is_empty(), "the front face's transform trigger is on the stack");

    // In response, Moonmist transforms it — it is a Human.
    mtg_engine::cards::helpers::apply_transform(&mut state, mayor, &reg);
    assert_eq!(state.name_of(mayor, &reg), "Howlpack Alpha");

    // The front face's ability resolves. Its condition is still true.
    mtg_engine::triggers::resolve_next_trigger(&mut state, &reg);

    assert!(!state.get_object(mayor).unwrap().is_transformed,
        "the front face's ability transformed it, so it is a Mayor again");
    assert_eq!(state.name_of(mayor, &reg), "Mayor of Avabruck");
}

/// CR 701.12b: "If one or both creatures instructed to fight are no longer on
/// the battlefield or are no longer creatures, neither of them fights or deals
/// damage."
///
/// Killing Nightfall Predator in response to its own fight ability spares the
/// target completely. The ability still resolves (CR 113.7a) — it just does
/// nothing. `combat::fight` used to read the dead creature's printed power off
/// its face and deal the damage anyway.
#[test]
fn nightfall_predators_fight_does_nothing_if_the_predator_dies_in_response() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let ranger = named_permanent(&mut state, &reg, "Daybreak Ranger", P0);
    mtg_engine::cards::helpers::apply_transform(&mut state, ranger, &reg);
    assert_eq!(state.name_of(ranger, &reg), "Nightfall Predator");

    let victim = ready_creature(&mut state, P1, 2, 2);

    // The Predator is killed while its ability is on the stack.
    state.move_object(ranger, Zone::Graveyard, &reg);

    mtg_engine::combat::fight(&mut state, ranger, victim, &reg);

    assert_eq!(state.get_object(victim).unwrap().damage_marked, 0,
        "neither creature fights, so the target takes nothing");
}

/// The other half of the same rule: the *target* leaving does not let the
/// fighter deal its damage into an empty seat either.
#[test]
fn a_fight_deals_no_damage_when_the_target_has_left_the_battlefield() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let fighter = ready_creature(&mut state, P0, 4, 4);
    let victim = ready_creature(&mut state, P1, 2, 2);
    state.move_object(victim, Zone::Graveyard, &reg);

    mtg_engine::combat::fight(&mut state, fighter, victim, &reg);

    assert_eq!(state.get_object(fighter).unwrap().damage_marked, 0,
        "neither of them fights, so the survivor takes nothing back");
}
