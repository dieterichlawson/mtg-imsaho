//! The LLM seat's "Pass until something happens": one row on every
//! priority offer, right after Pass, that passes now and keeps passing
//! every later plain priority offer without a model call until something
//! the seat would want to see happens. These tests drive the seat through
//! a fake `claude -p` (the stub backend `tests/claude_code_backend.rs`
//! uses) so that a model call is a subprocess that can be counted, and
//! hand it views built by hand so each stop condition is reached on its
//! own: a new stack entry, the seat's own turn, attackers against it, a
//! block against its attacker, its main phases, a land drop, a
//! sorcery-speed cast, a combat prompt, a prompt of another kind. The
//! recap after a stretch of unasked passes carries every event of the
//! stretch, and the row is one row.
#![cfg(unix)]

use std::collections::HashMap;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use mtg_engine::actions::{Action, ActivatableAbility, ActivatableAbilityOption, CastTargetSpec, CastableSpell, CombatPrompt, SetPrompt, SetPromptKind};
use mtg_engine::engine::LegalActions;
use mtg_engine::ids::{CardId, ObjectId, PlayerId};
use mtg_engine::types::{CardType, ManaPool, Step};
use mtg_engine::view::{AttackTarget, CardView, GameView, OpponentView, PermanentView, StackItemView};
use mtg_player::llm::{LlmPlayer, PASS_UNTIL_ROW};
use mtg_player::Player;

const YOU: PlayerId = PlayerId(0);
const OPP: PlayerId = PlayerId(1);

/// A fake `claude` that answers every call with the action index in
/// `$DIR/answer` and records each call's stdin, so the test can count the
/// model calls and read the prompts the seat sent.
struct Fake {
    dir: PathBuf,
}

impl Fake {
    fn new(name: &str) -> Fake {
        let dir = std::env::temp_dir().join(format!("mtg-pass-until-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let script = format!(
            "#!/bin/sh\n\
             if [ \"$1\" = \"--version\" ]; then echo 9.9.9; exit 0; fi\n\
             CALL=$(( $(cat \"{count}\" 2>/dev/null || echo 0) + 1 )); echo $CALL > \"{count}\"\n\
             {{ echo \"=== call $CALL\"; cat; echo; echo '--- end'; }} >> \"{log}\"\n\
             A=$(cat \"{answer}\" 2>/dev/null || echo 0)\n\
             printf '{{\"type\":\"result\",\"subtype\":\"success\",\"is_error\":false,\"session_id\":\"s\",\
             \"result\":\"{{}}\",\"structured_output\":{{\"action\":%s,\"thoughts\":\"t\",\
             \"attacker_indices\":[],\"blocks\":[],\"confirm\":false,\"card_indices\":[0],\"amount\":1}},\
             \"usage\":{{\"input_tokens\":1,\"output_tokens\":1,\"cache_read_input_tokens\":0,\
             \"cache_creation_input_tokens\":0}}}}\\n' \"$A\"\n",
            count = dir.join("count").display(),
            log = dir.join("log.txt").display(),
            answer = dir.join("answer").display(),
        );
        let bin = dir.join("claude");
        std::fs::write(&bin, script).unwrap();
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
        let fake = Fake { dir };
        fake.answer(0);
        fake
    }

    /// The index the next calls answer with.
    fn answer(&self, index: usize) {
        std::fs::write(self.dir.join("answer"), index.to_string()).unwrap();
    }

    fn calls(&self) -> Vec<String> {
        std::fs::read_to_string(self.dir.join("log.txt")).unwrap_or_default()
            .split("=== call ").skip(1).map(str::to_string).collect()
    }

    fn seat(&self) -> LlmPlayer {
        LlmPlayer::new_claude_code_with_binary("t", &self.dir.join("claude").display().to_string())
            .with_history(0)
            .with_pass_until(true)
    }
}

impl Drop for Fake {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn card(id: u64, name: &str, types: Vec<CardType>, oracle: &str) -> CardView {
    CardView {
        object_id: ObjectId(id),
        card_id: CardId(0),
        name: name.to_string(),
        cost: None,
        supertypes: vec![],
        card_types: types,
        power: None,
        toughness: None,
        oracle_text: oracle.to_string(),
        owner: YOU,
        flashback_costs: vec![],
    }
}

fn perm(id: u64, name: &str, controller: PlayerId) -> PermanentView {
    PermanentView {
        object_id: ObjectId(id),
        card_id: CardId(0),
        name: name.into(),
        supertypes: vec![],
        card_types: vec![CardType::Creature],
        controller,
        owner: controller,
        tapped: false,
        power: Some(2),
        toughness: Some(2),
        effective_power: Some(2),
        effective_toughness: Some(2),
        damage_marked: 0,
        regeneration_shields: 0,
        affected_by_summoning_sickness: false,
        attached_to: None,
        attached_to_player: None,
        keywords: vec![],
        colors: vec![],
        subtypes: vec![],
        printed_power: None,
        printed_toughness: None,
        star_pt: false,
        is_token: false,
        is_copy: false,
        protections: vec![],
        restrictions: vec![],
        granted_abilities: vec![],
        attacking: None,
        blocking: vec![],
        blocked_by: vec![],
        blocked: false,
        oracle_text: String::new(),
        counters: HashMap::new(),
        loyalty_abilities: vec![],
        mana_abilities: vec![],
        named_card: None,
    }
}

fn stack_item(id: u64, name: &str, controller: PlayerId) -> StackItemView {
    StackItemView {
        object_id: ObjectId(id),
        card_id: CardId(0),
        name: name.to_string(),
        source_id: Some(ObjectId(id)),
        controller,
        targets: vec![],
        x_value: None,
        cost: None,
        supertypes: vec![],
        card_types: vec![CardType::Creature],
        power: Some(2),
        toughness: Some(2),
        oracle_text: String::new(),
    }
}

/// A position: the seat holding a Lightning Bolt, the given turn and
/// step, the given player active.
fn view(turn: u32, step: Step, active: PlayerId) -> GameView {
    GameView {
        you: YOU,
        your_hand: vec![card(10, "Lightning Bolt", vec![CardType::Instant],
            "Lightning Bolt deals 3 damage to any target.")],
        your_life: 20,
        your_mana_pool: ManaPool::default(),
        your_library_size: 30,
        your_library_cards: vec![],
        your_mulligan_count: 0,
        opponents: vec![OpponentView {
            id: OPP, life: 20, hand_size: 7, library_size: 30,
            mana_pool: ManaPool::default(), mulligan_count: 0,
        }],
        battlefield: vec![],
        graveyards: vec![(YOU, vec![]), (OPP, vec![])],
        stack: vec![],
        first_strike_damage_step: false,
        exile: vec![],
        step,
        active_player: active,
        priority_player: Some(YOU),
        turn_number: turn,
        display_log: vec![],
        full_log: vec![],
        revealed_names: HashMap::new(),
    }
}

fn castable(id: u64, name: &str) -> CastableSpell {
    CastableSpell {
        object_id: ObjectId(id),
        name: name.to_string(),
        is_flashback: false,
        from_graveyard: false,
        target_spec: CastTargetSpec::NoTargets,
        tap_plan: vec![],
        exile_x_from_gy_max: None,
        sacrifice_options: vec![],
        additional_cost_label: None,
        alternative_cost: None,
    }
}

fn cast(id: u64) -> Action {
    Action::CastSpell {
        object_id: ObjectId(id), targets: vec![], sacrifice: None,
        exile_count: None, exile_ids: vec![], alternative_cost: None, tap_plan: vec![],
    }
}

/// A plain priority offer: Pass, cast the Bolt, Concede.
fn offer(context: &str) -> LegalActions {
    LegalActions {
        actions: vec![Action::PassPriority, cast(10), Action::Concede],
        combat_prompt: None,
        castable_spells: vec![castable(10, "Lightning Bolt")],
        activatable_abilities: vec![],
        context: Some(context.to_string()),
        resolution_prompt: None,
        set_prompt: None,
    }
}

/// Engage the pass-until on the opponent's turn 2, main phase 1: the
/// seat is asked once (one call) and answers with the row's index.
fn engaged_on_opponents_turn(fake: &Fake) -> LlmPlayer {
    let mut seat = fake.seat();
    let before = fake.calls().len();
    fake.answer(1);
    let chosen = seat.choose_action(&view(2, Step::PrecombatMain, OPP), &offer("OPPONENT'S TURN: Main Phase 1"));
    assert!(matches!(chosen, Action::PassPriority), "the row passes now: {chosen:?}");
    assert_eq!(fake.calls().len(), before + 1, "engaging is one call");
    fake.answer(0);
    seat
}

/// Engage it on the seat's own turn 3, main phase 1.
fn engaged_on_own_turn(fake: &Fake) -> LlmPlayer {
    let mut seat = fake.seat();
    let before = fake.calls().len();
    fake.answer(1);
    let chosen = seat.choose_action(&view(3, Step::PrecombatMain, YOU), &offer("MAIN PHASE 1"));
    assert!(matches!(chosen, Action::PassPriority), "{chosen:?}");
    assert_eq!(fake.calls().len(), before + 1, "engaging is one call");
    fake.answer(0);
    seat
}

/// The stop line the first prompt after a stop opens with.
fn stop_line(fake: &Fake) -> String {
    let last = fake.calls().pop().expect("a call was made");
    last.lines().find(|l| l.starts_with("You chose to pass until something happened"))
        .unwrap_or_else(|| panic!("the prompt after a stop says why it stopped:\n{last}"))
        .to_string()
}

#[test]
fn the_priority_menu_has_exactly_one_pass_until_row_right_after_pass() {
    let fake = Fake::new("row");
    let mut seat = fake.seat();
    seat.choose_action(&view(2, Step::PrecombatMain, OPP), &offer("OPPONENT'S TURN: Main Phase 1"));
    let prompt = fake.calls().pop().unwrap();
    let rows: Vec<&str> = prompt.lines().skip_while(|l| *l != "Available actions:").skip(1)
        .take_while(|l| !l.is_empty() && !l.starts_with("---")).collect();
    assert_eq!(rows[0], "0: Pass", "{prompt}");
    assert_eq!(rows[1], format!("1: {PASS_UNTIL_ROW}"), "{prompt}");
    assert_eq!(rows[2], "2: Cast Lightning Bolt", "the engine's rows follow, shifted by one: {prompt}");
    assert_eq!(rows[3], "3: Concede", "{prompt}");
    assert_eq!(rows.iter().filter(|r| r.contains("Pass until something happens")).count(), 1,
        "one row, not one per phase:\n{prompt}");

    // The row is an offer of the seat's own, so it can be left out — the
    // A/B knob, and what the tests of the engine's rows read.
    let mut plain = fake.seat().with_pass_until(false);
    plain.choose_action(&view(2, Step::PrecombatMain, OPP), &offer("OPPONENT'S TURN: Main Phase 1"));
    let prompt = fake.calls().pop().unwrap();
    assert!(!prompt.contains("Pass until something happens"), "{prompt}");
    assert!(prompt.contains("0: Pass\n1: Cast Lightning Bolt\n2: Concede"), "{prompt}");
}

#[test]
fn under_pass_until_a_plain_offer_is_passed_without_a_call() {
    let fake = Fake::new("passes");
    let mut seat = engaged_on_opponents_turn(&fake);
    // The rest of the opponent's turn, nothing on the stack, no attack:
    // every offer is passed, and the fake is never run.
    for step in [Step::BeginCombat, Step::DeclareAttackers, Step::EndCombat, Step::PostcombatMain, Step::EndStep] {
        let chosen = seat.choose_action(&view(2, step, OPP), &offer("OPPONENT'S TURN"));
        assert!(matches!(chosen, Action::PassPriority), "{step:?}: {chosen:?}");
    }
    assert_eq!(fake.calls().len(), 1, "no model call while passing:\n{:?}", fake.calls());
}

#[test]
fn a_new_entry_on_the_stack_stops_it() {
    let fake = Fake::new("stack");
    let mut seat = engaged_on_opponents_turn(&fake);
    let mut v = view(2, Step::PostcombatMain, OPP);
    v.stack.push(stack_item(40, "Grizzly Bears", OPP));
    seat.choose_action(&v, &offer("RESPOND TO opp's Grizzly Bears"));
    assert_eq!(fake.calls().len(), 2, "the opponent's spell is asked");
    assert!(stop_line(&fake).contains("Grizzly Bears (the opponent's) is on the stack"), "{}", stop_line(&fake));
}

#[test]
fn an_entry_that_was_already_on_the_stack_resolving_does_not_stop_it() {
    let fake = Fake::new("stack-unchanged");
    let mut seat = fake.seat();
    // Asked with two of the opponent's spells on the stack, the seat says
    // "go": the top one resolving and the bottom one still there is what
    // it declined to respond to, not news.
    let mut v = view(2, Step::PrecombatMain, OPP);
    v.stack.push(stack_item(40, "Grizzly Bears", OPP));
    v.stack.push(stack_item(41, "Savannah Lions", OPP));
    fake.answer(1);
    seat.choose_action(&v, &offer("RESPOND TO opp's Savannah Lions"));
    fake.answer(0);
    v.stack.pop();
    let chosen = seat.choose_action(&v, &offer("RESPOND TO opp's Grizzly Bears"));
    assert!(matches!(chosen, Action::PassPriority));
    assert_eq!(fake.calls().len(), 1, "the same stack, shorter, is passed unasked");
    // A different entry in the bottom one's place is new.
    v.stack.pop();
    v.stack.push(stack_item(42, "Grizzly Bears", OPP));
    seat.choose_action(&v, &offer("RESPOND TO opp's Grizzly Bears"));
    assert_eq!(fake.calls().len(), 2, "a second Bears, same name, is a new entry");
    // And at a later step anything on the stack is new, whatever it is
    // called: the stack was empty for the step to move on.
    let mut seat = engaged_on_opponents_turn(&fake);
    let mut later = view(2, Step::EndStep, OPP);
    later.stack.push(stack_item(40, "Grizzly Bears", OPP));
    seat.choose_action(&later, &offer("RESPOND TO opp's Grizzly Bears"));
    assert_eq!(fake.calls().len(), 4);
}

#[test]
fn the_seats_own_turn_beginning_stops_it() {
    let fake = Fake::new("own-turn");
    let mut seat = engaged_on_opponents_turn(&fake);
    seat.choose_action(&view(3, Step::Upkeep, YOU), &offer("UPKEEP"));
    assert_eq!(fake.calls().len(), 2);
    assert!(stop_line(&fake).contains("your turn 3 began"), "{}", stop_line(&fake));
}

#[test]
fn attackers_declared_against_the_seat_stop_it() {
    let fake = Fake::new("attacked");
    let mut seat = engaged_on_opponents_turn(&fake);
    let mut v = view(2, Step::DeclareAttackers, OPP);
    let mut bears = perm(40, "Grizzly Bears", OPP);
    bears.attacking = Some(AttackTarget::Player(YOU));
    v.battlefield.push(bears);
    seat.choose_action(&v, &offer("AFTER ATTACKERS DECLARED"));
    assert_eq!(fake.calls().len(), 2, "an attack is asked, before blocks");
    assert!(stop_line(&fake).contains("attackers declared against you: Grizzly Bears"), "{}", stop_line(&fake));

    // The opponent's combat with nobody attacking is not an attack.
    let mut seat = engaged_on_opponents_turn(&fake);
    let mut v = view(2, Step::DeclareAttackers, OPP);
    v.battlefield.push(perm(40, "Grizzly Bears", OPP));
    let chosen = seat.choose_action(&v, &offer("OPPONENT'S TURN: Declare Attackers"));
    assert!(matches!(chosen, Action::PassPriority));
    assert_eq!(fake.calls().len(), 3);
}

#[test]
fn a_block_against_the_seats_attacker_stops_it() {
    let fake = Fake::new("blocked");
    let mut seat = engaged_on_own_turn(&fake);
    let mut v = view(3, Step::DeclareBlockers, YOU);
    let mut lions = perm(30, "Savannah Lions", YOU);
    lions.attacking = Some(AttackTarget::Player(OPP));
    lions.blocked = true;
    lions.blocked_by = vec![ObjectId(40)];
    v.battlefield.push(lions);
    let mut bears = perm(40, "Grizzly Bears", OPP);
    bears.blocking = vec![ObjectId(30)];
    v.battlefield.push(bears);
    seat.choose_action(&v, &offer("AFTER BLOCKERS DECLARED"));
    assert_eq!(fake.calls().len(), 2, "a block is the moment for a trick");
    assert!(stop_line(&fake).contains("your attacker is blocked: Savannah Lions"), "{}", stop_line(&fake));

    // An unblocked attack is passed through to damage.
    let mut seat = engaged_on_own_turn(&fake);
    let mut v = view(3, Step::DeclareBlockers, YOU);
    let mut lions = perm(30, "Savannah Lions", YOU);
    lions.attacking = Some(AttackTarget::Player(OPP));
    v.battlefield.push(lions);
    let chosen = seat.choose_action(&v, &offer("AFTER BLOCKERS DECLARED"));
    assert!(matches!(chosen, Action::PassPriority));
    assert_eq!(fake.calls().len(), 3);
}

#[test]
fn the_seats_next_main_phase_stops_it() {
    let fake = Fake::new("main");
    // Engaged in main phase 1: combat is passed, main phase 2 is asked.
    let mut seat = engaged_on_own_turn(&fake);
    for step in [Step::BeginCombat, Step::DeclareAttackers, Step::EndCombat] {
        assert!(matches!(seat.choose_action(&view(3, step, YOU), &offer("BEGIN COMBAT")), Action::PassPriority));
    }
    assert_eq!(fake.calls().len(), 1);
    seat.choose_action(&view(3, Step::PostcombatMain, YOU), &offer("MAIN PHASE 2"));
    assert_eq!(fake.calls().len(), 2);
    assert!(stop_line(&fake).contains("your Main Phase 2"), "{}", stop_line(&fake));

    // Engaged on the opponent's turn: the seat's main phase 1 is asked
    // even when the offers before it were the engine's to pass.
    let mut seat = engaged_on_opponents_turn(&fake);
    seat.choose_action(&view(3, Step::PrecombatMain, YOU), &offer("MAIN PHASE 1"));
    assert_eq!(fake.calls().len(), 4);
    assert!(stop_line(&fake).contains("your turn 3 began"), "{}", stop_line(&fake));
}

#[test]
fn a_land_drop_or_a_sorcery_speed_cast_in_the_same_main_phase_stops_it() {
    let fake = Fake::new("sorcery");
    // The seat cast something at main phase 1 and said "go" to let it
    // resolve; back at the same main phase with an instant only, there is
    // nothing new to ask.
    let mut seat = engaged_on_own_turn(&fake);
    let chosen = seat.choose_action(&view(3, Step::PrecombatMain, YOU), &offer("MAIN PHASE 1"));
    assert!(matches!(chosen, Action::PassPriority));
    assert_eq!(fake.calls().len(), 1, "the same main phase with only an instant on offer is passed");

    // With a land to play, it is asked.
    let mut seat = engaged_on_own_turn(&fake);
    let mut with_land = offer("MAIN PHASE 1");
    with_land.actions.insert(1, Action::PlayLand { object_id: ObjectId(11) });
    seat.choose_action(&view(3, Step::PrecombatMain, YOU), &with_land);
    assert_eq!(fake.calls().len(), 3);
    assert!(stop_line(&fake).contains("you can play a land"), "{}", stop_line(&fake));

    // With a creature castable, it is asked; the Bolt alone was not.
    let mut seat = engaged_on_own_turn(&fake);
    let mut v = view(3, Step::PrecombatMain, YOU);
    v.your_hand.push(card(12, "Grizzly Bears", vec![CardType::Creature], ""));
    let mut with_bears = offer("MAIN PHASE 1");
    with_bears.actions.insert(2, cast(12));
    with_bears.castable_spells.push(castable(12, "Grizzly Bears"));
    seat.choose_action(&v, &with_bears);
    assert_eq!(fake.calls().len(), 5);
    assert!(stop_line(&fake).contains("you could cast Grizzly Bears at sorcery speed"), "{}", stop_line(&fake));

    // A flash creature is instant-speed, like the Bolt.
    let mut seat = engaged_on_own_turn(&fake);
    let mut v = view(3, Step::PrecombatMain, YOU);
    v.your_hand.push(card(13, "Ambush Viper", vec![CardType::Creature], "Flash\nDeathtouch"));
    let mut with_viper = offer("MAIN PHASE 1");
    with_viper.actions.insert(2, cast(13));
    with_viper.castable_spells.push(castable(13, "Ambush Viper"));
    assert!(matches!(seat.choose_action(&v, &with_viper), Action::PassPriority));
    assert_eq!(fake.calls().len(), 6);
}

#[test]
fn a_sorcery_speed_activation_in_the_same_main_phase_stops_it_whatever_its_text() {
    // Brain Weevil's sacrifice is activate-only-as-a-sorcery, and its text
    // ("Sacrifice: Target player discards two cards") does not say so: the
    // seat used to read sorcery speed from the words and passed it (#751).
    let fake = Fake::new("weevil");
    let ability = |id: u64, name: &str, desc: &str, sorcery_speed: bool| ActivatableAbility {
        object_id: ObjectId(id), ability_index: 0, source_card_id: None,
        name: name.to_string(), description: desc.to_string(),
        target_options: vec![], tap_plan: vec![],
        option_combos: vec![ActivatableAbilityOption { targets: vec![], sacrifice: None }],
        sorcery_speed,
    };
    let activate = |id: u64| Action::ActivateAbility {
        object_id: ObjectId(id), ability_index: 0, targets: vec![], tap_plan: vec![],
        sacrifice: None, x_value: None, source_card_id: None,
    };
    // An instant-speed ability is passed, like the Bolt.
    let mut seat = engaged_on_own_turn(&fake);
    let mut with_pump = offer("MAIN PHASE 1");
    with_pump.actions.insert(2, activate(22));
    with_pump.activatable_abilities.push(ability(22, "Darkthicket Wolf", "{2}{G}: +2/+2 until end of turn", false));
    assert!(matches!(seat.choose_action(&view(3, Step::PrecombatMain, YOU), &with_pump), Action::PassPriority));
    assert_eq!(fake.calls().len(), 1);
    // A sorcery-speed one is asked.
    let mut seat = engaged_on_own_turn(&fake);
    let mut with_weevil = offer("MAIN PHASE 1");
    with_weevil.actions.insert(2, activate(63));
    with_weevil.activatable_abilities.push(ability(63, "Brain Weevil", "Sacrifice: Target player discards two cards", true));
    seat.choose_action(&view(3, Step::PrecombatMain, YOU), &with_weevil);
    assert_eq!(fake.calls().len(), 3);
    assert!(stop_line(&fake).contains("you could activate Brain Weevil"), "{}", stop_line(&fake));
}

#[test]
fn a_combat_prompt_stops_it_and_is_asked() {
    let fake = Fake::new("combat");
    let mut seat = engaged_on_own_turn(&fake);
    let mut v = view(3, Step::DeclareAttackers, YOU);
    v.battlefield.push(perm(30, "Savannah Lions", YOU));
    v.battlefield.push(perm(31, "Grizzly Bears", YOU));
    let prompt = CombatPrompt::ChooseAttackers {
        eligible: vec![ObjectId(30), ObjectId(31)], must_attack: vec![],
        defending_player: OPP, defending_planeswalkers: vec![],
    };
    seat.choose_combat(&v, &prompt);
    assert_eq!(fake.calls().len(), 2, "the attack is asked");
    assert!(stop_line(&fake).contains("you must declare attackers"), "{}", stop_line(&fake));
    // And the pass-until is over: the next plain offer is asked too.
    seat.choose_action(&view(3, Step::DeclareAttackers, YOU), &offer("AFTER ATTACKERS DECLARED"));
    assert_eq!(fake.calls().len(), 3);

    let mut seat = engaged_on_opponents_turn(&fake);
    let mut v = view(2, Step::DeclareBlockers, OPP);
    v.battlefield.push(perm(30, "Savannah Lions", YOU));
    v.battlefield.push(perm(31, "Grizzly Bears", YOU));
    let mut attacker = perm(40, "Walking Corpse", OPP);
    attacker.attacking = Some(AttackTarget::Player(YOU));
    v.battlefield.push(attacker);
    let prompt = CombatPrompt::ChooseBlockers {
        eligible_blockers: vec![ObjectId(30), ObjectId(31)], attackers: vec![ObjectId(40)],
        legal_blocks: [(ObjectId(30), vec![ObjectId(40)]), (ObjectId(31), vec![ObjectId(40)])].into_iter().collect(),
        min_blockers: HashMap::new(),
    };
    seat.choose_combat(&v, &prompt);
    assert_eq!(fake.calls().len(), 5);
    assert!(stop_line(&fake).contains("you must declare blockers"), "{}", stop_line(&fake));
}

#[test]
fn a_prompt_of_another_kind_stops_it_and_is_asked() {
    let fake = Fake::new("other");
    let mut seat = engaged_on_own_turn(&fake);
    let mut v = view(3, Step::Cleanup, YOU);
    v.your_hand.push(card(12, "Grizzly Bears", vec![CardType::Creature], ""));
    let discard = LegalActions {
        actions: vec![],
        combat_prompt: None,
        castable_spells: vec![],
        activatable_abilities: vec![],
        context: Some("DISCARD 1 CARD".to_string()),
        resolution_prompt: None,
        set_prompt: Some(SetPrompt {
            kind: SetPromptKind::DiscardToHandSize, player: YOU,
            options: vec![ObjectId(10), ObjectId(12)], min: 1, max: 1,
        }),
    };
    seat.choose_action(&v, &discard);
    assert_eq!(fake.calls().len(), 2, "the discard is asked");
    assert!(stop_line(&fake).contains("you are asked something else: DISCARD 1 CARD"), "{}", stop_line(&fake));
    // Over: the next plain offer is asked.
    seat.choose_action(&view(4, Step::Upkeep, OPP), &offer("OPPONENT'S TURN: Upkeep"));
    assert_eq!(fake.calls().len(), 3);
}

#[test]
fn the_recap_after_a_stretch_of_unasked_passes_carries_every_event() {
    let fake = Fake::new("recap");
    let mut seat = fake.seat();
    let mut v = view(2, Step::PrecombatMain, OPP);
    v.display_log = vec!["── Turn 2 (p1) ──".to_string(), "p1 played Swamp".to_string()];
    fake.answer(1);
    seat.choose_action(&v, &offer("OPPONENT'S TURN: Main Phase 1"));
    fake.answer(0);
    // Four offers passed unasked, each with more in the log.
    let events = [
        "p1 cast Walking Corpse", "p1's Walking Corpse resolved", "p1 attacks with Walking Corpse",
        "p1's Walking Corpse deals 2 damage to p0", "── Turn 3 (p0) ──", "p0 drew a card",
    ];
    for (k, step) in [Step::BeginCombat, Step::DeclareAttackers, Step::CombatDamage, Step::EndStep].iter().enumerate() {
        v.step = *step;
        v.display_log.push(events[k].to_string());
        assert!(matches!(seat.choose_action(&v, &offer("OPPONENT'S TURN")), Action::PassPriority));
    }
    assert_eq!(fake.calls().len(), 1);
    // The stop: the seat's turn. Its prompt carries the whole stretch.
    let mut own = view(3, Step::Upkeep, YOU);
    own.display_log = v.display_log.clone();
    own.display_log.push(events[4].to_string());
    own.display_log.push(events[5].to_string());
    seat.choose_action(&own, &offer("UPKEEP"));
    let prompt = fake.calls().pop().unwrap();
    let recap = prompt.split("Recent events:\n").nth(1).unwrap_or_else(|| panic!("a recap:\n{prompt}"));
    // (Each in the seat's you/opp vocabulary; the wording is the
    // rewriter's, what matters here is that none is dropped.)
    for expected in ["Opp cast Walking Corpse", "Walking Corpse resolved", "Opp attacks with Walking Corpse",
                     "Walking Corpse deals 2 damage to", "── Turn 3 (your turn) ──", "You drew a card"] {
        assert!(recap.contains(expected), "the recap is missing {expected:?}:\n{prompt}");
    }
    assert!(!recap.contains("played Swamp"), "what the engaging prompt already carried is not repeated:\n{prompt}");
    assert!(prompt.contains("after 4 unasked passes it stopped: your turn 3 began"), "{prompt}");
    let stop = prompt.find("You chose to pass until").unwrap();
    assert!(stop < prompt.find("Recent events:").unwrap(), "the stop line comes before the recap:\n{prompt}");
}

#[test]
fn a_new_game_forgets_it() {
    let fake = Fake::new("new-game");
    let mut seat = engaged_on_opponents_turn(&fake);
    let registry = mtg_engine::cards::CardRegistry::with_all_cards();
    seat.init_conversation(&[("Lightning Bolt".to_string(), 4)], "", &registry, mtg_player::llm::MatchFormat::best_of(3));
    seat.choose_action(&view(1, Step::PrecombatMain, OPP), &offer("OPPONENT'S TURN: Main Phase 1"));
    assert_eq!(fake.calls().len(), 2, "game two's first offer is asked");
    let prompt = fake.calls().pop().unwrap();
    assert!(!prompt.contains("You chose to pass until"), "{prompt}");
}

#[test]
fn the_knob_turns_the_row_off() {
    use mtg_player::llm::{pass_until_enabled, PASS_UNTIL_ENV};
    std::env::remove_var(PASS_UNTIL_ENV);
    assert!(pass_until_enabled(), "offered by default");
    for off in ["off", "OFF", "0", "false", "no"] {
        std::env::set_var(PASS_UNTIL_ENV, off);
        assert!(!pass_until_enabled(), "{off:?} turns it off");
    }
    std::env::set_var(PASS_UNTIL_ENV, "on");
    assert!(pass_until_enabled());
    std::env::remove_var(PASS_UNTIL_ENV);
}
