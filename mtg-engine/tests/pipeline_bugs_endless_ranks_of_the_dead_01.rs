mod common;
use common::*;

use mtg_engine::cards::CardRegistry;
use mtg_engine::state::StackEntry;
use mtg_engine::triggers::PendingTrigger;
use mtg_engine::types::*;

// CR 113.7a: A triggered ability on the stack exists independently of its source.
// If Endless Ranks of the Dead is removed after its upkeep trigger stacks,
// the trigger should still resolve and create Zombie tokens.
#[test]
fn endless_ranks_resolves_after_enchantment_removed() {
    let reg = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::Upkeep, P0);

    let ranks = named_creature(&mut state, &reg, "Endless Ranks of the Dead", P0);
    let ranks_card = reg.get_id_by_name("Endless Ranks of the Dead").unwrap();

    for _ in 0..4 {
        let z = ready_creature(&mut state, P0, 2, 2);
        let obj = state.get_object_mut(z).unwrap();
        obj.is_token = true;
        obj.subtypes = vec!["Zombie".into()];
        obj.name = "Zombie".into();
    }

    assert_eq!(count_tokens_named(&state, "Zombie"), 4);

    state.stack.push(StackEntry::Trigger(PendingTrigger::UpkeepTrigger {
        object_id: ranks,
        card_id: ranks_card,
        controller: P0,
        description: "Endless Ranks of the Dead".into(),
        chosen_targets: vec![],
    }));

    state.move_object(ranks, Zone::Graveyard, &reg);
    mtg_engine::triggers::resolve_next_trigger(&mut state, &reg);

    assert_eq!(
        count_tokens_named(&state, "Zombie"),
        6,
        "CR 113.7a: Endless Ranks trigger should create 2 Zombie tokens (4/2=2) even after enchantment is removed"
    );
}
