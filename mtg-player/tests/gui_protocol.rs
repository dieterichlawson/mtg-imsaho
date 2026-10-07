//! The GUI seat's protocol: the engine's own types as JSON.
//!
//! The page (`mtg-gui/src/prompts.ts`) answers by the *kind* of prompt in
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
    let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../mtg-gui/src/prompts.ts"))
        .expect("mtg-gui/src/prompts.ts is part of the repository");
    let mut kinds = BTreeSet::new();
    for line in src.lines() {
        let l = line.trim();
        if let Some(rest) = l.strip_prefix("case \"") {
            if let Some(name) = rest.split('"').next() {
                // Any kind name. This used to admit only `Choose*` and
                // three others by name, so a kind called anything else
                // (AssignCombatDamage, #637) could never count as handled
                // however many arms the page gave it.
                if name.chars().next().is_some_and(char::is_uppercase) {
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
            "the engine asks `{name}` and mtg-gui/src/prompts.ts has no arm for it: \
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
         Add a `case` for each in mtg-gui/src/prompts.ts (a list is the safe default), \
         then add it here if a list is right.");
}

/// The X-funding cases the page is held to.
///
/// The page builds its own `Action`, so it carries its own copy of
/// `funding::allocate_for_x` — the engine cannot be called from the browser.
/// A copy is what #404 and #561 are about: a fix that went into one request
/// path and not the other. So the engine writes the answers down here and
/// `mtg-gui/tests/xfunding.js` holds the page to them, which fails whichever
/// side moves alone.
///
/// Regenerate with `MTG_UPDATE_X_FUNDING_CASES=1 cargo test -p mtg-player
/// --test gui_protocol` and commit the file with the change that moved it.
#[test]
fn the_page_is_given_the_engines_x_funding_answers() {
    use mtg_engine::funding::{self, FundingCategory, FundingGroup, FundingOptions};
    use mtg_engine::ids::ObjectId;
    use mtg_engine::types::ManaType;

    fn group(name: &str, category: FundingCategory, per_tap: u32, count: u64) -> FundingGroup {
        FundingGroup {
            name: name.into(),
            category,
            mana_per_tap: per_tap,
            source_ids: (0..count).map(|i| ObjectId(200 + i)).collect(),
            colors_produced: vec![],
        }
    }

    // The shapes that made the two surfaces disagree: a quantum bigger than
    // one (#593, #594), two quanta that need the later group alone, a pool
    // that must be kept rather than spent, a cost reduction (CR 601.2f), and
    // a group that produces nothing.
    let boards: Vec<(&str, Vec<FundingGroup>)> = vec![
        ("one-land", vec![group("Mountain", FundingCategory::Lands, 1, 1)]),
        ("one-rock", vec![group("Sol Ring", FundingCategory::Rocks, 2, 1)]),
        ("three-rocks", vec![group("Sol Ring", FundingCategory::Rocks, 2, 3)]),
        ("land-and-rock", vec![
            group("Mountain", FundingCategory::Lands, 1, 1),
            group("Sol Ring", FundingCategory::Rocks, 2, 1)]),
        ("rock-and-dorks", vec![
            group("Sol Ring", FundingCategory::Rocks, 2, 1),
            group("Llanowar Elves", FundingCategory::Dorks, 1, 2)]),
        ("coprime-rocks", vec![
            group("Worn Powerstone", FundingCategory::Rocks, 4, 2),
            group("Gilded Lotus", FundingCategory::Rocks, 3, 1)]),
        ("a-group-making-nothing", vec![
            group("Nothing", FundingCategory::Rocks, 0, 2),
            group("Sol Ring", FundingCategory::Rocks, 2, 2)]),
    ];

    let mut cases = Vec::new();
    for (name, groups) in boards {
        for (pool_red, pool_white) in [(0, 0), (1, 0), (2, 1)] {
            for x_discount in [0, 2] {
                let mut pool = std::collections::BTreeMap::new();
                if pool_red > 0 {
                    pool.insert(ManaType::Red, pool_red);
                }
                if pool_white > 0 {
                    pool.insert(ManaType::White, pool_white);
                }
                let max_x = pool_red + pool_white
                    + groups.iter().map(FundingGroup::max_contribution).sum::<u32>();
                let options = FundingOptions { pool, groups: groups.clone(), max_x, x_discount };
                let allocations: Vec<serde_json::Value> = (0..=options.max_announceable_x())
                    .map(|x| {
                        let (response, shortfall) = funding::allocate_for_x(&options, x);
                        serde_json::json!({
                            "x": x,
                            "pool": response.pool,
                            "taps": response.taps,
                            "shortfall": shortfall,
                        })
                    })
                    .collect();
                cases.push(serde_json::json!({
                    "board": format!("{name}/pool{pool_red}-{pool_white}/discount{x_discount}"),
                    "options": options,
                    "fundable": funding::fundable_x_values(&options),
                    "allocations": allocations,
                }));
            }
        }
    }
    // One compact case per line: a 40-case fixture pretty-printed is 80 KB of
    // one-number lines, and a diff that names the board that moved is worth
    // more than indentation.
    let mut fresh = String::from(
        "{\"generated_by\":\"mtg-player/tests/gui_protocol.rs::the_page_is_given_the_engines_x_funding_answers\",\
          \"read_by\":\"mtg-gui/tests/xfunding.js\"}\n",
    );
    for case in &cases {
        fresh.push_str(&serde_json::to_string(case).expect("serialize"));
        fresh.push('\n');
    }

    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../mtg-gui/tests/x-funding-cases.jsonl");
    if std::env::var_os("MTG_UPDATE_X_FUNDING_CASES").is_some() {
        std::fs::write(path, &fresh).expect("write the fixture");
        return;
    }
    let committed = std::fs::read_to_string(path).unwrap_or_default();
    assert_eq!(
        committed, fresh,
        "the engine's X-funding answers have moved and mtg-gui/tests/x-funding-cases.jsonl \
         has not. The page carries its own allocator, so a change here is a change there: \
         port it into mtg-gui/src/prompts.ts, run `MTG_UPDATE_X_FUNDING_CASES=1 cargo test \
         -p mtg-player --test gui_protocol`, rebuild the page (cd mtg-gui && npx tsc -p .) \
         and commit all three."
    );
}

/// Read the property names of one `export interface` out of the page's
/// protocol declarations.
fn page_interface_fields(src: &str, name: &str) -> BTreeSet<String> {
    let start = src.find(&format!("export interface {name} {{"))
        .unwrap_or_else(|| panic!("mtg-gui/src/protocol.ts declares no {name}"));
    let body = &src[start..];
    let end = body.find("\n}").unwrap_or_else(|| panic!("{name} never ends"));
    body[..end].lines()
        .filter_map(|line| {
            let l = line.trim();
            // `name: Type;` at two spaces of indent. Doc comments and the
            // interface's own header line have no bare `name:` prefix.
            if !line.starts_with("  ") || l.starts_with('*') || l.starts_with("/*") || l.starts_with("//") {
                return None;
            }
            let (field, _) = l.split_once(':')?;
            let field = field.trim().trim_end_matches('?');
            field.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
                .then(|| field.to_string())
        })
        .collect()
}

/// Every field of the view the page is sent is a field the page declares.
///
/// The page is handed `GameView` as it is — nothing on the Rust side is
/// per-prompt — so `mtg-gui/src/protocol.ts` is a second, hand-written copy
/// of the view's shape, and TypeScript cannot know it has gone stale. A
/// field renamed or added on the engine side reads as `undefined` in the
/// browser, which is falsy: the badge simply never paints, the fact simply
/// never appears in the inspector, and nothing anywhere fails. That is how
/// #604 stayed open — the page was reading a field whose *meaning* had moved
/// on — and it is what the "one decision, four surfaces" rule exists to
/// stop.
///
/// The engine's names are taken from the serialized JSON rather than the
/// struct, because that is what crosses the socket.
#[test]
fn every_view_field_the_page_is_sent_is_one_the_page_declares() {
    let registry = CardRegistry::with_all_cards();
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../decks/coverage/");
    let config = GameConfig {
        player_names: vec!["a".into(), "b".into()],
        decklists: vec![coverage_deck(&format!("{root}wb-coverage.txt")),
                        coverage_deck(&format!("{root}rg-coverage.txt"))],
        starting_life: 20,
        starting_player: Some(PlayerId(0)),
        rng_seed: Some(7),
    };
    let state = engine::setup_game(&config, &registry);
    let view = GameView::for_player(&state, PlayerId(0), &registry);
    let json = serde_json::to_value(&view).expect("the view serializes");

    let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../mtg-gui/src/protocol.ts"))
        .expect("the page's protocol declarations");

    // The top level, and the two object shapes a board is made of. A
    // battlefield is empty at the start of a game, so the permanent's fields
    // come from the engine's own view of a staged board below.
    let mut checked = 0;
    for (interface, sent) in [
        ("GameView", json.as_object().expect("the view is an object").clone()),
        ("CardView", json["your_hand"].as_array().and_then(|a| a.first())
            .and_then(|c| c.as_object()).expect("a hand card").clone()),
    ] {
        let declared = page_interface_fields(&src, interface);
        let missing: Vec<&String> = sent.keys().filter(|k| !declared.contains(*k)).collect();
        assert!(missing.is_empty(),
            "{interface}: the engine sends fields mtg-gui/src/protocol.ts does not declare: \
             {missing:?}. The page reads a field it does not declare as `undefined`, which is \
             falsy and silent — add them there and to whatever reads them in render.ts.");
        checked += sent.len();
    }

    // A permanent, from a state with something on the battlefield.
    let mut staged = state;
    let card_id = registry.get_id_by_name("Walking Corpse").expect("a creature in the pool");
    staged.create_object(card_id, PlayerId(0), mtg_engine::types::Zone::Battlefield, Some(2), Some(2));
    let staged_view = GameView::for_player(&staged, PlayerId(0), &registry);
    let perm = serde_json::to_value(&staged_view.battlefield[0]).expect("a permanent serializes");
    let declared = page_interface_fields(&src, "PermanentView");
    let sent = perm.as_object().expect("a permanent is an object");
    let missing: Vec<&String> = sent.keys().filter(|k| !declared.contains(*k)).collect();
    assert!(missing.is_empty(),
        "PermanentView: the engine sends fields mtg-gui/src/protocol.ts does not declare: \
         {missing:?}. The page reads a field it does not declare as `undefined`, which is \
         falsy and silent — add them there and to whatever reads them in render.ts.");
    checked += sent.len();
    assert!(checked >= 60, "only {checked} fields compared — this guard has stopped covering");
}

/// The inspector names a counter as the engine does (#723): the page keeps
/// a table from the serde key the view sends to `CounterType::label`, and
/// printed the key itself before it had one ("2 PlusOnePlusOne counters").
/// The match below has no wildcard, so a new counter kind fails to compile
/// here until the page is given its label.
#[test]
fn the_page_labels_every_counter_kind_as_the_engine_does() {
    use mtg_engine::types::CounterType;
    fn every(c: CounterType) -> CounterType {
        match c {
            CounterType::PlusOnePlusOne | CounterType::MinusOneMinusOne | CounterType::Loyalty
            | CounterType::Slime | CounterType::Study | CounterType::Hatchling => c,
        }
    }
    let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../mtg-gui/src/render.ts"))
        .expect("the page's renderer");
    for c in [CounterType::PlusOnePlusOne, CounterType::MinusOneMinusOne, CounterType::Loyalty,
              CounterType::Slime, CounterType::Study, CounterType::Hatchling].map(every) {
        let key = serde_json::to_value(c).unwrap();
        let entry = format!("{}: \"{}\"", key.as_str().unwrap(), c.label());
        assert!(src.contains(&entry), "render.ts's COUNTER_LABEL lacks `{entry}`");
    }
}
