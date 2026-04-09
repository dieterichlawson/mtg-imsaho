use crate::ids::{ObjectId, PlayerId, CardId};
use crate::state::GameState;
use crate::cards::CardRegistry;
use crate::types::*;

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
    pub keywords: Vec<Keyword>,
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
}

impl GameView {
    /// Build a view of the game state for a specific player.
    pub fn for_player(state: &GameState, player: PlayerId, registry: &CardRegistry) -> Self {
        let player_state = state.get_player(player);

        // Your hand: you can see all cards.
        let your_hand = state.objects_in_zone(Zone::Hand, player)
            .iter()
            .map(|obj| card_view(obj, registry))
            .collect();

        // Your library cards (you know what's in your deck, not the order).
        let your_library_cards = player_state.library_order.iter()
            .filter_map(|&obj_id| state.get_object(obj_id))
            .map(|obj| card_view(obj, registry))
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
                    .cloned()
                    .collect();
                PermanentView {
                    object_id: obj.id,
                    card_id: obj.card_id,
                    name: registry.card_data(obj.card_id)
                        .map(|d| d.name)
                        .unwrap_or_else(|| obj.name.clone()),
                    card_types: registry.card_data(obj.card_id)
                        .map(|d| d.card_types)
                        .unwrap_or_else(|| obj.card_types.clone()),
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
                    keywords,
                }
            })
            .collect();

        // Graveyards: public zones, all visible.
        let graveyards = state.players.iter()
            .map(|p| {
                let cards = state.objects_in_zone(Zone::Graveyard, p.id)
                    .iter()
                    .map(|obj| card_view(obj, registry))
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
                                .map(|d| d.name)
                                .unwrap_or_else(|| "Unknown".into()),
                            controller: obj.controller,
                            targets: obj.targets.clone(),
                        })
                    }
                    crate::state::StackEntry::Trigger(trigger) => {
                        Some(StackItemView {
                            object_id: ObjectId(0), // triggers don't have an object ID
                            card_id: CardId(0),
                            name: trigger.display_name(registry),
                            controller: trigger.controller(),
                            targets: vec![],
                        })
                    }
                }
            })
            .collect();

        // Exile.
        let exile = state.all_objects_in_zone(Zone::Exile)
            .iter()
            .map(|obj| card_view(obj, registry))
            .collect();

        GameView {
            you: player,
            your_hand,
            your_life: player_state.life,
            your_mana_pool: player_state.mana_pool.clone(),
            your_library_size: player_state.library_order.len(),
            your_library_cards,
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
        }
    }
}

fn card_view(obj: &crate::state::GameObject, registry: &CardRegistry) -> CardView {
    let data = registry.card_data(obj.card_id);
    CardView {
        object_id: obj.id,
        card_id: obj.card_id,
        name: data.as_ref().map(|d| d.name.clone()).unwrap_or_else(|| "Unknown".into()),
        cost: data.as_ref().and_then(|d| d.cost.clone()),
        card_types: data.as_ref().map(|d| d.card_types.clone()).unwrap_or_default(),
        power: obj.power,
        toughness: obj.toughness,
        oracle_text: data.as_ref().map(|d| d.oracle_text.clone()).unwrap_or_default(),
        owner: obj.owner,
        flashback_cost: data.map(|d| d.flashback_cost).unwrap_or(None),
    }
}
