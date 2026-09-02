use crate::ids::{ObjectId, PlayerId, CardId};
use crate::state::GameState;
use crate::cards::CardRegistry;
use crate::types::{ManaPool, Step, ManaCost, CardType, Keyword, CounterType, Zone};

/// A player's view of the game — hidden info filtered out.
#[derive(Debug, Clone)]
pub struct GameView {
    pub you: PlayerId,
    pub your_hand: Vec<CardView>,
    pub your_life: i32,
    pub your_mana_pool: ManaPool,
    pub your_library_size: usize,
    /// Card names in your library (you know your decklist, just not the order).
    pub your_library_cards: Vec<CardView>,
    /// Number of mulligans this player has already taken this game (London
    /// mulligan). Used by the LLM mulligan prompt so it can reason about the
    /// resulting hand size on keep vs. mull.
    pub your_mulligan_count: u32,

    pub opponents: Vec<OpponentView>,

    pub battlefield: Vec<PermanentView>,
    pub graveyards: Vec<(PlayerId, Vec<CardView>)>,
    pub stack: Vec<StackItemView>,
    pub exile: Vec<CardView>,

    pub step: Step,
    pub active_player: PlayerId,
    pub priority_player: Option<PlayerId>,
    pub turn_number: u32,

    /// Display-worthy log entries (Event level and above).
    pub display_log: Vec<String>,
    /// Full detailed log (all levels).
    pub full_log: Vec<String>,

    /// Names of objects referenced in pending choices (revealed cards, opponent's
    /// hand from Night Terrors, etc.) that aren't otherwise visible in the view.
    pub revealed_names: std::collections::HashMap<ObjectId, String>,
}

#[derive(Debug, Clone)]
pub struct CardView {
    pub object_id: ObjectId,
    pub card_id: CardId,
    pub name: String,
    pub cost: Option<ManaCost>,
    pub card_types: Vec<CardType>,
    pub power: Option<i32>,
    pub toughness: Option<i32>,
    pub oracle_text: String,
    pub owner: PlayerId,
    pub flashback_cost: Option<ManaCost>,
}

#[derive(Debug, Clone)]
pub struct PermanentView {
    pub object_id: ObjectId,
    pub card_id: CardId,
    pub name: String,
    pub card_types: Vec<CardType>,
    pub controller: PlayerId,
    pub owner: PlayerId,
    pub tapped: bool,
    pub power: Option<i32>,
    pub toughness: Option<i32>,
    pub effective_power: Option<i32>,
    pub effective_toughness: Option<i32>,
    pub damage_marked: u32,
    pub summoning_sick: bool,
    pub attached_to: Option<ObjectId>,
    /// The player this Aura enchants (Curses, CR 702.5c). A Curse's entire
    /// identity is whom it curses, and the display had nowhere to read it —
    /// two curses on opposite players rendered identically (issue #81).
    pub attached_to_player: Option<PlayerId>,
    pub keywords: Vec<Keyword>,
    /// Oracle text of the card (from the registry). Used by display code to
    /// surface short effect summaries for attached auras/equipment.
    pub oracle_text: String,
    /// Counters on the permanent (+1/+1, -1/-1, loyalty, etc). Exposed so
    /// the LLM prompt can render counter state alongside effective P/T.
    pub counters: std::collections::HashMap<CounterType, u32>,
    /// Loyalty abilities as (ability_index, "+1: Each player discards a
    /// card."). Exposed so ability menus can NAME the ability a player is
    /// choosing — an index alone made two abilities on one walker render
    /// identically (#61). Empty for non-planeswalkers.
    pub loyalty_abilities: Vec<(usize, String)>,
    /// Mana abilities as (ability_index, "Add {W}"). Exposed so the menu can
    /// NAME the mana an entry produces — a dual land rendered as two
    /// byte-identical "Tap ... for mana" rows, a filter land as six (#118).
    pub mana_abilities: Vec<(usize, String)>,
}

#[derive(Debug, Clone)]
pub struct StackItemView {
    pub object_id: ObjectId,
    pub card_id: CardId,
    pub name: String,
    pub controller: PlayerId,
    pub targets: Vec<crate::actions::Target>,
}

#[derive(Debug, Clone)]
pub struct OpponentView {
    pub id: PlayerId,
    pub life: i32,
    pub hand_size: usize,
    pub library_size: usize,
    pub mana_pool: ManaPool,
    /// Number of London mulligans this opponent has taken so far. Used by
    /// the pre-game mulligan prompt so the deciding player knows the
    /// opponent's mull count.
    pub mulligan_count: u32,
}

impl GameView {
    /// Build a view of the game state for a specific player.
    #[must_use]
    pub fn for_player(state: &GameState, player: PlayerId, registry: &CardRegistry) -> Self {
        let player_state = state.get_player(player);

        // Your hand: you can see all cards.
        let your_hand = state.objects_in_zone(Zone::Hand, player)
            .iter()
            .map(|obj| card_view(state, obj, registry))
            .collect();

        // Your library cards (you know what's in your deck, not the order).
        let your_library_cards = player_state.library_order.iter()
            .filter_map(|&obj_id| state.get_object(obj_id))
            .map(|obj| card_view(state, obj, registry))
            .collect();

        // Opponents.
        let opponents = state.players.iter()
            .filter(|p| p.id != player)
            .map(|p| OpponentView {
                id: p.id,
                life: p.life,
                hand_size: state.objects_in_zone(Zone::Hand, p.id).len(),
                library_size: p.library_order.len(),
                mana_pool: p.mana_pool.clone(),
                mulligan_count: p.mulligan_count,
            })
            .collect();

        // Battlefield: all permanents are visible.
        let all_keywords = [
            Keyword::Flying, Keyword::FirstStrike, Keyword::DoubleStrike,
            Keyword::Trample, Keyword::Deathtouch, Keyword::Lifelink,
            Keyword::Vigilance, Keyword::Flash, Keyword::Reach,
            Keyword::Haste, Keyword::Defender, Keyword::Hexproof,
            Keyword::Intimidate, Keyword::Menace, Keyword::Indestructible,
        ];
        let battlefield = state.all_objects_in_zone(Zone::Battlefield)
            .iter()
            .map(|obj| {
                let keywords: Vec<Keyword> = all_keywords.iter()
                    .filter(|kw| state.has_keyword(obj.id, **kw, registry))
                    .copied()
                    .collect();
                // For transformed DFCs, show the back-face name and card types
                // so the display matches the active face. Without this, a
                // transformed Villagers of Estwald shows as "Villagers of
                // Estwald 4/6" (front name, back P/T), misleading the LLM.
                let face_data = if obj.is_transformed {
                    registry.get(obj.card_id).and_then(super::cards::CardBehavior::back_face_data)
                } else {
                    registry.card_data(obj.card_id)
                };
                PermanentView {
                    object_id: obj.id,
                    card_id: obj.card_id,
                    name: face_data.as_ref()
                        .map_or_else(|| obj.name.clone(), |d| d.name.clone()),
                    card_types: face_data.as_ref()
                        .map_or_else(|| obj.card_types.clone(), |d| d.card_types.clone()),
                    controller: obj.controller,
                    owner: obj.owner,
                    tapped: obj.tapped,
                    power: obj.power,
                    toughness: obj.toughness,
                    effective_power: state.effective_power(obj.id, registry),
                    effective_toughness: state.effective_toughness(obj.id, registry),
                    damage_marked: obj.damage_marked,
                    summoning_sick: obj.summoning_sick,
                    attached_to: obj.attached_to,
                    attached_to_player: obj.attached_to_player,
                    keywords,
                    oracle_text: face_data.as_ref()
                        .map(|d| d.oracle_text.clone())
                        .unwrap_or_default(),
                    counters: obj.counters.clone(),
                    mana_abilities: registry.get(obj.card_id)
                        .map(|b| b.mana_abilities(state, obj.id).iter()
                            .map(|ab| (ab.ability_index, ab.description.clone()))
                            .collect())
                        .unwrap_or_default(),
                    loyalty_abilities: registry.get(obj.card_id)
                        .map(|b| b.loyalty_abilities(state, obj.id).iter().map(|ab| {
                            let sign = if ab.loyalty_change > 0 {
                                format!("+{}", ab.loyalty_change)
                            } else {
                                ab.loyalty_change.to_string()
                            };
                            // Most descriptions already lead with their cost
                            // ("+1: Each player discards a card."); only add
                            // the sign when the card's text doesn't carry it.
                            let d = ab.description.trim();
                            let label = if d.starts_with(&sign) {
                                d.to_string()
                            } else {
                                format!("{sign}: {d}")
                            };
                            (ab.ability_index, label)
                        }).collect())
                        .unwrap_or_default(),
                }
            })
            .collect();

        // Graveyards: public zones, all visible.
        let graveyards = state.players.iter()
            .map(|p| {
                let cards = state.objects_in_zone(Zone::Graveyard, p.id)
                    .iter()
                    .map(|obj| card_view(state, obj, registry))
                    .collect();
                (p.id, cards)
            })
            .collect();

        // Stack.
        let stack = state.stack.iter()
            .rev() // top of stack first
            .filter_map(|entry| {
                match entry {
                    crate::state::StackEntry::Spell(obj_id) => {
                        let obj = state.get_object(*obj_id)?;
                        Some(StackItemView {
                            object_id: obj.id,
                            card_id: obj.card_id,
                            name: registry.card_data(obj.card_id)
                                .map_or_else(|| "Unknown".into(), |d| d.name),
                            controller: obj.controller,
                            targets: obj.targets.clone(),
                        })
                    }
                    crate::state::StackEntry::Trigger(trigger) => {
                        Some(StackItemView {
                            object_id: ObjectId(0), // triggers don't have an object ID
                            card_id: CardId(0),
                            name: trigger.display_name_with_state(registry, Some(state)),
                            controller: trigger.controller(),
                            targets: vec![],
                        })
                    }
                    crate::state::StackEntry::Ability { source_id, behavior_card_id, activator, targets, .. } => {
                        Some(StackItemView {
                            object_id: *source_id,
                            card_id: *behavior_card_id,
                            name: registry.card_data(*behavior_card_id)
                                .map_or_else(|| "Ability".into(), |d| format!("{} ability", d.name)),
                            controller: *activator,
                            targets: targets.clone(),
                        })
                    }
                }
            })
            .collect();

        // Exile.
        let exile = state.all_objects_in_zone(Zone::Exile)
            .iter()
            .map(|obj| card_view(state, obj, registry))
            .collect();

        // Collect names of objects referenced in pending resolution choices
        // that might not be in any visible zone (e.g. opponent's hand, library).
        let mut revealed_names = std::collections::HashMap::new();
        if let Some(ref awaiting) = state.awaiting_action {
            let ids_to_resolve: Vec<ObjectId> = match awaiting {
                crate::state::AwaitingAction::ResolutionChoice { choice, .. } => {
                    use crate::state::ResolutionChoiceKind;
                    match choice {
                        ResolutionChoiceKind::ChooseTarget { options, .. } => {
                            options.iter().filter_map(|t| match t {
                                crate::actions::Target::Object(id) => Some(*id),
                                crate::actions::Target::Player(_) => None,
                                // CR 608.2b: a target that stopped being legal is skipped.
                                crate::actions::Target::Illegal => None,
                            }).collect()
                        }
                        ResolutionChoiceKind::ChooseCardFromHand { cards, .. } => cards.clone(),
                        ResolutionChoiceKind::ChooseFromRevealed { revealed, .. } => revealed.clone(),
                        ResolutionChoiceKind::ChooseFromLibrary { options, .. } => options.clone(),
                        _ => vec![],
                    }
                }
                _ => vec![],
            };
            for id in ids_to_resolve {
                if let Some(obj) = state.get_object(id) {
                    let name = registry.card_data(obj.card_id)
                        .map_or_else(|| obj.name.clone(), |d| d.name.clone());
                    revealed_names.insert(id, name);
                }
            }
        }

        GameView {
            you: player,
            your_hand,
            your_life: player_state.life,
            your_mana_pool: player_state.mana_pool.clone(),
            your_library_size: player_state.library_order.len(),
            your_library_cards,
            your_mulligan_count: player_state.mulligan_count,
            opponents,
            battlefield,
            graveyards,
            stack,
            exile,
            step: state.step,
            active_player: state.active_player,
            priority_player: state.priority_player,
            turn_number: state.turn_number,
            display_log: state.game_log.iter()
                .filter(|e| e.level >= crate::state::LogLevel::Info)
                .map(|e| e.message.clone())
                .collect(),
            full_log: state.game_log.iter()
                .map(|e| e.message.clone())
                .collect(),
            revealed_names,
        }
    }
}

fn card_view(state: &GameState, obj: &crate::state::GameObject, registry: &CardRegistry) -> CardView {
    let data = registry.card_data(obj.card_id);
    // CR 208.2: characteristic-defining abilities work in all zones, so use
    // effective_power/toughness which consults dynamic_pt and continuous effects.
    let power = state.effective_power(obj.id, registry).or(obj.power);
    let toughness = state.effective_toughness(obj.id, registry).or(obj.toughness);
    CardView {
        object_id: obj.id,
        card_id: obj.card_id,
        name: data.as_ref().map_or_else(|| "Unknown".into(), |d| d.name.clone()),
        cost: data.as_ref().and_then(|d| d.cost.clone()),
        card_types: data.as_ref().map(|d| d.card_types.clone()).unwrap_or_default(),
        power,
        toughness,
        oracle_text: data.as_ref().map(|d| d.oracle_text.clone()).unwrap_or_default(),
        owner: obj.owner,
        flashback_cost: data.and_then(|d| d.flashback_cost),
    }
}
