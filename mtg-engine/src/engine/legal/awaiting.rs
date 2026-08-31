//! What a player may do while the engine is waiting on a specific answer.
//!
//! When `awaiting_action` is set the ordinary priority-based options do not
//! apply: the only legal moves are the ones that answer the question.

use super::super::*;
use crate::actions::Action;
use crate::cards::CardRegistry;
use crate::combat;
use crate::ids::ObjectId;
use crate::state::{AwaitingAction, GameState};

/// The legal actions for the pending `awaiting_action`, or `None` if the
/// engine is not waiting on one.
pub(crate) fn legal_actions_while_awaiting(
    state: &GameState,
    registry: &CardRegistry,
) -> Option<LegalActions> {
    let awaiting = state.awaiting_action.as_ref()?;
    Some(match awaiting {
        AwaitingAction::DeclareAttackers => {
            let active = state.active_player;
            let eligible = combat::eligible_attackers(state, active, registry);
            let defending = state.opponent(active);
            // Find creatures that must attack (e.g., enchanted by Furor of the Bitten).
            let must_attack: Vec<ObjectId> = eligible
                .iter()
                .filter(|&&id| state.must_attack(id, registry))
                .copied()
                .collect();
            LegalActions {
                actions: vec![],
                combat_prompt: Some(crate::actions::CombatPrompt::ChooseAttackers {
                    eligible,
                    must_attack,
                    defending_player: defending,
                    // CR 508.1a: each attacker may be sent at the player or at
                    // a planeswalker that player controls.
                    defending_planeswalkers: state.objects_in_zone(crate::types::Zone::Battlefield, defending)
                        .iter()
                        .filter(|o| state.has_card_type(o.id, crate::types::CardType::Planeswalker, registry))
                        .map(|o| o.id)
                        .collect(),
                }),
                castable_spells: vec![],
                activatable_abilities: vec![],
                context: Some("DECLARE ATTACKERS".into()),
                resolution_prompt: None,
            }
        }
        AwaitingAction::DeclareBlockers { defending_player } => {
            let eligible_blockers = combat::eligible_blockers(state, *defending_player, registry);
            let attacker_ids: Vec<_> = state
                .combat
                .as_ref()
                .map(|c| c.attackers.keys().copied().collect())
                .unwrap_or_default();
            let mut legal_blocks = std::collections::HashMap::new();
            for &blocker_id in &eligible_blockers {
                let can_block: Vec<_> = attacker_ids
                    .iter()
                    .filter(|&&att_id| {
                        combat::can_block_attacker(state, blocker_id, att_id, registry)
                    })
                    .copied()
                    .collect();
                legal_blocks.insert(blocker_id, can_block);
            }
            LegalActions {
                actions: vec![],
                combat_prompt: Some(crate::actions::CombatPrompt::ChooseBlockers {
                    eligible_blockers,
                    attackers: attacker_ids,
                    legal_blocks,
                }),
                castable_spells: vec![],
                activatable_abilities: vec![],
                context: Some("DECLARE BLOCKERS".into()),
                resolution_prompt: None,
            }
        }
        AwaitingAction::DiscardToHandSize {
            player,
            discard_count,
        } => LegalActions {
            actions: legal_discard_actions(state, *player, *discard_count),
            combat_prompt: None,
            castable_spells: vec![],
            activatable_abilities: vec![],
            context: Some(format!(
                "DISCARD {} CARD{}",
                discard_count,
                if *discard_count == 1 { "" } else { "S" }
            )),
            resolution_prompt: None,
        },
        AwaitingAction::MulliganDecision { player } => {
            let mull_count = state.get_player(*player).mulligan_count;
            let mut actions = vec![Action::MulliganKeep];
            if mull_count < crate::state::LONDON_MULLIGAN_CAP {
                actions.push(Action::MulliganMull);
            }
            LegalActions {
                actions,
                combat_prompt: None,
                castable_spells: vec![],
                activatable_abilities: vec![],
                context: Some(format!(
                    "MULLIGAN DECISION (mulligans taken: {}/{})",
                    mull_count,
                    crate::state::LONDON_MULLIGAN_CAP
                )),
                resolution_prompt: None,
            }
        }
        AwaitingAction::BottomAfterMulligan { player, count } => {
            // Enumerate combinations of `count` cards from hand so the
            // action list is self-contained for simple players. Rich
            // players (LLM/CLI) can bypass this and construct a
            // BottomCards action directly — submit_action validates that
            // the chosen cards are in hand and distinct.
            let hand: Vec<ObjectId> = state
                .objects_in_zone(Zone::Hand, *player)
                .iter()
                .map(|o| o.id)
                .collect();
            let combos = combinations(&hand, *count);
            let actions: Vec<Action> = combos
                .into_iter()
                .map(|cards| Action::BottomCards { cards })
                .collect();
            LegalActions {
                actions,
                combat_prompt: None,
                castable_spells: vec![],
                activatable_abilities: vec![],
                context: Some(format!(
                    "BOTTOM {} CARD{} AFTER MULLIGAN",
                    count,
                    if *count == 1 { "" } else { "S" }
                )),
                resolution_prompt: None,
            }
        }
        AwaitingAction::ResolutionChoice {
            choice,
            source,
            player,
            ..
        } => {
            use crate::actions::ResolvedChoice;
            use crate::state::ResolutionChoiceKind;
            let source_name = card_name(state, registry, *source);
            let actions = match choice {
                ResolutionChoiceKind::PayOrNot { cost, .. } => {
                    // Declining is always available; paying is offered only
                    // when the player can actually produce the mana, here
                    // or by tapping (CR 608.2g).
                    let mut acts = Vec::new();
                    if can_pay_with_sources(state, *player, cost, registry) {
                        acts.push(Action::ResolveChoice {
                            choice: ResolvedChoice::PayDecision(true),
                        });
                    }
                    acts.push(Action::ResolveChoice {
                        choice: ResolvedChoice::PayDecision(false),
                    });
                    acts
                }
                ResolutionChoiceKind::ChooseTarget {
                    options, optional, ..
                } => {
                    let mut acts: Vec<Action> = options
                        .iter()
                        .map(|t| Action::ResolveChoice {
                            choice: ResolvedChoice::ChosenTarget(Some(t.clone())),
                        })
                        .collect();
                    if *optional {
                        acts.push(Action::ResolveChoice {
                            choice: ResolvedChoice::ChosenTarget(None),
                        });
                    }
                    acts
                }
                ResolutionChoiceKind::YesNo { .. } => {
                    vec![
                        Action::ResolveChoice {
                            choice: ResolvedChoice::YesNoDecision(true),
                        },
                        Action::ResolveChoice {
                            choice: ResolvedChoice::YesNoDecision(false),
                        },
                    ]
                }
                ResolutionChoiceKind::ChooseCardFromHand { cards, .. } => cards
                    .iter()
                    .map(|&id| Action::ResolveChoice {
                        choice: ResolvedChoice::ChosenCard(id),
                    })
                    .collect(),
                ResolutionChoiceKind::ChooseFromRevealed { revealed, .. } => revealed
                    .iter()
                    .map(|&id| Action::ResolveChoice {
                        choice: ResolvedChoice::ChosenCard(id),
                    })
                    .collect(),
                ResolutionChoiceKind::ChooseFromLibrary { options, .. } => options
                    .iter()
                    .map(|&id| Action::ResolveChoice {
                        choice: ResolvedChoice::ChosenCard(id),
                    })
                    // CR 701.19b: searching a hidden zone never forces a find,
                    // so "take none of them" is always one of the answers.
                    .chain(std::iter::once(Action::ResolveChoice {
                        choice: ResolvedChoice::ChosenTarget(None),
                    }))
                    .collect(),
                ResolutionChoiceKind::ChooseCardType { options, .. } => options
                    .iter()
                    .enumerate()
                    .map(|(i, name)| Action::ResolveChoice {
                        choice: ResolvedChoice::ChosenIndex(i, name.clone()),
                    })
                    .collect(),
                ResolutionChoiceKind::DividePermanentsIntoPiles { permanents, .. } => {
                    // Generate all possible subsets of permanents (each subset = pile 1).
                    // With N permanents there are 2^N subsets. This is fine for typical
                    // board states (up to ~15 permanents = 32768 actions).
                    let n = permanents.len();
                    (0..(1u64 << n))
                        .map(|mask| {
                            let subset: Vec<ObjectId> = (0..n)
                                .filter(|&i| mask & (1u64 << i) != 0)
                                .map(|i| permanents[i])
                                .collect();
                            Action::ResolveChoice {
                                choice: ResolvedChoice::ChosenSubset(subset),
                            }
                        })
                        .collect()
                }
                ResolutionChoiceKind::ChoosePile { pile_1, pile_2, .. } => {
                    let fmt_pile = |ids: &[ObjectId]| -> String {
                        if ids.is_empty() {
                            return "empty".to_string();
                        }
                        ids.iter()
                            .filter_map(|id| state.objects.get(id).map(|o| o.name.clone()))
                            .collect::<Vec<_>>()
                            .join(", ")
                    };
                    vec![
                        Action::ResolveChoice {
                            choice: ResolvedChoice::ChosenIndex(
                                0,
                                format!("Pile 1: [{}]", fmt_pile(pile_1)),
                            ),
                        },
                        Action::ResolveChoice {
                            choice: ResolvedChoice::ChosenIndex(
                                1,
                                format!("Pile 2: [{}]", fmt_pile(pile_2)),
                            ),
                        },
                    ]
                }
                ResolutionChoiceKind::ChooseCardName { options, .. } => options
                    .iter()
                    .enumerate()
                    .map(|(i, name)| Action::ResolveChoice {
                        choice: ResolvedChoice::ChosenIndex(i, name.clone()),
                    })
                    .collect(),
                ResolutionChoiceKind::ChooseTriggerOrder { options, .. } => options
                    .iter()
                    .enumerate()
                    .map(|(i, name)| Action::ResolveChoice {
                        choice: ResolvedChoice::ChosenIndex(i, name.clone()),
                    })
                    .collect(),
                ResolutionChoiceKind::ChooseXFunding { .. }
                | ResolutionChoiceKind::ChooseExileFromGraveyard { .. } => {
                    // Structured prompt — can't be enumerated as a flat
                    // action list. Player implementations see the
                    // `resolution_prompt` field and construct the
                    // response directly (XFunding / ChosenExileSet).
                    vec![]
                }
            };
            let context = match choice {
                ResolutionChoiceKind::ChooseTarget { description, .. }
                | ResolutionChoiceKind::PayOrNot { description, .. }
                | ResolutionChoiceKind::ChooseCardFromHand { description, .. }
                | ResolutionChoiceKind::ChooseCardName { description, .. }
                | ResolutionChoiceKind::ChooseXFunding { description, .. }
                | ResolutionChoiceKind::ChooseExileFromGraveyard { description, .. }
                | ResolutionChoiceKind::ChooseTriggerOrder { description, .. } => {
                    description.clone()
                }
                ResolutionChoiceKind::YesNo { .. } => format!("{source_name}: choose yes or no"),
                ResolutionChoiceKind::ChooseFromRevealed { .. } => {
                    format!("{source_name}: choose a card")
                }
                ResolutionChoiceKind::ChooseFromLibrary { .. } => {
                    format!("{source_name}: search library")
                }
                ResolutionChoiceKind::ChooseCardType { options, .. } => {
                    let opts = options
                        .iter()
                        .enumerate()
                        .map(|(i, name)| format!("{i}: {name}"))
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!("{source_name}: choose a card type ({opts})")
                }
                ResolutionChoiceKind::DividePermanentsIntoPiles { .. } => {
                    format!("{source_name}: divide into piles")
                }
                ResolutionChoiceKind::ChoosePile { pile_1, pile_2, .. } => {
                    let fmt_pile = |ids: &[ObjectId]| -> String {
                        if ids.is_empty() {
                            return "empty".to_string();
                        }
                        ids.iter()
                            .filter_map(|id| state.objects.get(id).map(|o| o.name.clone()))
                            .collect::<Vec<_>>()
                            .join(", ")
                    };
                    format!(
                        "{}: choose which pile to sacrifice (0: [{}], 1: [{}])",
                        source_name,
                        fmt_pile(pile_1),
                        fmt_pile(pile_2)
                    )
                }
            };
            LegalActions {
                actions,
                combat_prompt: None,
                castable_spells: vec![],
                activatable_abilities: vec![],
                context: Some(context),
                resolution_prompt: Some(choice.clone()),
            }
        }
    })
}
