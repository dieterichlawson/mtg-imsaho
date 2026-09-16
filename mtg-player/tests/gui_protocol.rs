//! The GUI seat's protocol: the engine's own types as JSON.
//!
//! The page (`mtg-gui/src/prompts.js`) answers by the *kind* of prompt in
//! `LegalActions`. These tests hold the two sides together:
//!
//! - every decision point a seeded random game reaches serializes, so the
//!   seat can always send it; and
//! - every prompt kind those games reach is one the page has an arm for,
//!   read from the page's own source, so a new `ResolutionChoiceKind`
//!   fails here until the page is taught it (the GUI's version of
//!   CLAUDE.md's "one decision, three surfaces").

use std::collections::BTreeSet;

use mtg_engine::cards::CardRegistry;
use mtg_engine::engine::{self, Decklist, GameConfig};
use mtg_engine::ids::PlayerId;
use mtg_engine::view::GameView;
use mtg_player::random::RandomPlayer;
use mtg_player::Player;

fn coverage_deck(path: &str) -> Decklist {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let entries = text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| {
            let (n, name) = l.split_once(' ')?;
            Some((name.trim().to_string(), n.parse::<u32>().ok()?))
        })
        .collect();
    Decklist { entries }
}

/// Play `seeds` games and return every prompt kind the seats were asked,
/// checking at each decision that the message the GUI seat would send
/// serializes.
fn prompt_kinds_reached(seeds: impl Iterator<Item = u64>) -> BTreeSet<String> {
    let registry = CardRegistry::with_all_cards();
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../decks/coverage/");
    let decks = ["ub-coverage.txt", "rg-coverage.txt", "wb-coverage.txt", "ug-coverage.txt"];
    let mut kinds = BTreeSet::new();
    for seed in seeds {
        let d1 = coverage_deck(&format!("{root}{}", decks[(seed as usize) % decks.len()]));
        let d2 = coverage_deck(&format!("{root}{}", decks[(seed as usize + 1) % decks.len()]));
        let config = GameConfig {
            player_names: vec!["a".into(), "b".into()],
            decklists: vec![d1, d2],
            starting_life: 20,
            starting_player: Some(PlayerId(0)),
            rng_seed: Some(seed),
        };
        let mut state = engine::setup_game(&config, &registry);
        let mut seats = [RandomPlayer::with_seed("a", seed + 1), RandomPlayer::with_seed("b", seed + 2)];
        let mut decisions = 0u32;
        engine::run_game_loop(&mut state, &registry, |game_state, acting, legal| {
            decisions += 1;
            let view = GameView::for_player(game_state, acting, &registry);
            // What the seat sends, exactly.
            let msg = serde_json::json!({
                "type": "decision", "seq": decisions, "seat": acting, "view": view,
                "legal": legal, "combat": legal.combat_prompt,
            });
            let text = serde_json::to_string(&msg).expect("a decision serializes");
            let back: serde_json::Value = serde_json::from_str(&text).expect("and reads back");
            if let Some(rp) = back["legal"]["resolution_prompt"].as_object() {
                for k in rp.keys() { kinds.insert(format!("resolution:{k}")); }
            }
            if back["legal"]["set_prompt"].is_object() { kinds.insert("set_prompt".into()); }
            if let Some(c) = back["combat"].as_object() {
                for k in c.keys() { kinds.insert(format!("combat:{k}")); }
            }
            if decisions > 4000 {
                return mtg_engine::actions::Action::AbandonGame;
            }
            let seat = &mut seats[acting.0 as usize];
            match &legal.combat_prompt {
                Some(p) => seat.choose_combat(p),
                None => seat.choose_action(&view, legal),
            }
        });
    }
    kinds
}

/// The prompt kinds the page has an arm for, read from its source.
fn kinds_the_page_handles() -> BTreeSet<String> {
    let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../mtg-gui/src/prompts.js"))
        .expect("mtg-gui/src/prompts.js is part of the repository");
    let mut kinds = BTreeSet::new();
    for line in src.lines() {
        let l = line.trim();
        if let Some(rest) = l.strip_prefix("case \"") {
            if let Some(name) = rest.split('"').next() {
                if name.starts_with("Choose") || name == "PayOrNot" || name == "YesNo" || name == "DividePermanentsIntoPiles" {
                    kinds.insert(name.to_string());
                }
            }
        }
    }
    kinds
}

#[test]
fn every_decision_point_serializes_and_is_a_kind_the_page_handles() {
    let reached = prompt_kinds_reached(1..=12);
    assert!(reached.contains("combat:ChooseAttackers"), "no combat in twelve games? {reached:?}");
    assert!(reached.contains("set_prompt") || reached.iter().any(|k| k.starts_with("resolution:")),
        "no prompt of any kind in twelve games? {reached:?}");
    let handled = kinds_the_page_handles();
    // The page's `default` arm takes any ResolveChoice list, but the kinds
    // with a board-side widget are named in its source. Everything reached
    // that is NOT named there is one of the list-shaped kinds, which is the
    // documented fallback; say which so a new kind is a visible choice.
    let list_shaped: BTreeSet<&str> = ["PayOrNot", "YesNo", "ChooseCardType", "ChooseDamageEffect",
        "ChoosePile", "ChooseCardName"].into_iter().collect();
    for k in &reached {
        let Some(name) = k.strip_prefix("resolution:") else { continue };
        assert!(handled.contains(name) || list_shaped.contains(name),
            "the engine asks `{name}` and mtg-gui/src/prompts.js has no arm for it: \
             add one (or add it to the list-shaped kinds here if a plain list is right)");
    }
}

#[test]
fn every_resolution_kind_the_engine_defines_is_known_to_the_page() {
    // The enum's variants, read from the engine's source the same way the
    // page's arms are read from its own: a variant added to one side and
    // not the other fails here without a card that reaches it.
    let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../mtg-engine/src/state.rs"))
        .expect("engine source");
    let start = src.find("pub enum ResolutionChoiceKind {").expect("the enum exists");
    let body = &src[start..];
    let end = body.find("\n}\n").expect("the enum ends");
    let mut variants = BTreeSet::new();
    for line in body[..end].lines() {
        let l = line.trim();
        // A variant line is `Name {` at four spaces of indent.
        if line.starts_with("    ") && !line.starts_with("     ") && !l.starts_with("//") && !l.starts_with('#') {
            if let Some(name) = l.split(['{', '(', ',']).next() {
                let name = name.trim();
                if !name.is_empty() && name.chars().next().is_some_and(char::is_uppercase) {
                    variants.insert(name.to_string());
                }
            }
        }
    }
    assert!(variants.len() >= 15, "found only {variants:?}");
    let handled = kinds_the_page_handles();
    let list_shaped = ["PayOrNot", "YesNo", "ChooseCardType", "ChooseDamageEffect", "ChoosePile", "ChooseCardName"];
    let unknown: Vec<_> = variants.iter()
        .filter(|v| !handled.contains(*v) && !list_shaped.contains(&v.as_str()))
        .collect();
    assert!(unknown.is_empty(),
        "ResolutionChoiceKind has variants the page does not name: {unknown:?}. \
         Add a `case` for each in mtg-gui/src/prompts.js (a list is the safe default), \
         then add it here if a list is right.");
}
