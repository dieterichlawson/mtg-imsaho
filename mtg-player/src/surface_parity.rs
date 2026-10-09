//! One decision, two menus: the rows the CLI and the LLM seat offer for
//! one `LegalActions` stand for the same offers.
//!
//! Each surface collapses `legal.actions` into one row per way to cast and
//! one per activatable ability, and each dedupes on a key of its own. The
//! keys drifted from the engine's at different times — the LLM seat's in
//! #589, the CLI's in #610 — and each drift dropped a legal option from one
//! surface while the other still offered it. Nothing failed: both halves
//! of `LegalActions` agree with each other, so `--check-invariants`, which
//! is engine-internal, cannot see a row a surface never rendered (the
//! 2026-09-29 playtest report, "Method notes"). The keys are one function
//! now (`crate::cast_offer_key`, `crate::ability_offer_key`); this is the
//! check that holds what each surface *renders* against the offer it
//! stands for, over the boards the fuzzer reaches, so the next drift —
//! in a key, a label, a dedupe — fails here instead of in a playtest.
//!
//! In-crate rather than under `tests/` because both menu builders and
//! both `DisplayEntry` types are crate-private, and the fixtures of the
//! regression tests at the bottom live in the two surfaces' own test
//! modules.

use std::collections::BTreeSet;

use mtg_engine::actions::Action;
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine::{self, Decklist, GameConfig, LegalActions};
use mtg_engine::ids::PlayerId;
use mtg_engine::view::GameView;

use crate::cli::{CliPlayer, DisplayEntry as CliEntry};
use crate::llm::{ActionRow, DisplayEntry as LlmEntry, LlmPlayer};
use crate::random::RandomPlayer;
use crate::{ability_offer_key, cast_offer_key, AbilityOfferKey, CastOfferKey, Player};

/// What a display entry on either surface points at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Entry {
    /// Index into `LegalActions::actions`.
    Direct(usize),
    /// Index into `LegalActions::castable_spells`.
    Cast(usize),
    /// Index into `LegalActions::activatable_abilities`.
    Ability(usize),
}

impl From<CliEntry> for Entry {
    fn from(e: CliEntry) -> Self {
        match e {
            CliEntry::Direct(i) => Entry::Direct(i),
            CliEntry::Cast(i) => Entry::Cast(i),
            CliEntry::Ability(i) => Entry::Ability(i),
        }
    }
}

impl From<LlmEntry> for Entry {
    fn from(e: LlmEntry) -> Self {
        match e {
            LlmEntry::Direct(i) => Entry::Direct(i),
            LlmEntry::Cast(i) => Entry::Cast(i),
            LlmEntry::Ability(i) => Entry::Ability(i),
        }
    }
}

/// What an entry stands for, in the engine's own keys.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Offer {
    Cast(CastOfferKey),
    Ability(AbilityOfferKey),
    /// Anything else, which no surface collapses: the action itself.
    Other(usize),
}

fn offer_of_action(legal: &LegalActions, i: usize) -> Offer {
    match &legal.actions[i] {
        Action::CastSpell { object_id, alternative_cost, .. } =>
            Offer::Cast(cast_offer_key(*object_id, alternative_cost.as_ref())),
        Action::ActivateAbility { object_id, ability_index, source_card_id, .. } =>
            Offer::Ability(ability_offer_key(*object_id, *ability_index, *source_card_id)),
        _ => Offer::Other(i),
    }
}

/// The offer an entry resolves to — property (c): every entry points into
/// one of the three lists, at something that is there.
fn offer_of(legal: &LegalActions, entry: Entry, surface: &str, at: &str) -> Offer {
    match entry {
        Entry::Direct(i) => {
            assert!(i < legal.actions.len(),
                "{at}: {surface} row points at action {i} of {}", legal.actions.len());
            offer_of_action(legal, i)
        }
        Entry::Cast(i) => {
            let cs = legal.castable_spells.get(i).unwrap_or_else(|| panic!(
                "{at}: {surface} row points at castable spell {i} of {}", legal.castable_spells.len()));
            Offer::Cast(cast_offer_key(cs.object_id, cs.alternative_cost.as_ref()))
        }
        Entry::Ability(i) => {
            let ab = legal.activatable_abilities.get(i).unwrap_or_else(|| panic!(
                "{at}: {surface} row points at activatable ability {i} of {}",
                legal.activatable_abilities.len()));
            Offer::Ability(ability_offer_key(ab.object_id, ab.ability_index, ab.source_card_id))
        }
    }
}

/// One surface's menu for one decision: the rows as they read, and what
/// each display index stands for.
struct Surface {
    name: &'static str,
    /// Every row, as the reader sees it. A `Copies` row is one row here.
    rows: Vec<String>,
    /// One per display index, in order, with its offer.
    entries: Vec<(Entry, Offer)>,
}

impl Surface {
    /// The CLI's menu, rendered the way `choose_action` prints it: rows
    /// that read alike are told apart by the ids of the objects they name
    /// (`menu_row_texts`, #257), so that is the text held to property (b).
    fn cli(view: &GameView, legal: &LegalActions, at: &str) -> Surface {
        let (entries, labels) = CliPlayer::build_action_menu(view, legal);
        let rows = CliPlayer::menu_row_texts(&labels);
        assert_eq!(rows.len(), entries.len(), "{at}: the CLI menu has a row per entry");
        let entries = entries.into_iter()
            .map(|e| (Entry::from(e), offer_of(legal, e.into(), "CLI", at)))
            .collect();
        Surface { name: "CLI", rows, entries }
    }

    /// The LLM seat's list. A `Copies` row takes one display index per id
    /// it names (#461), so its entries are the ids' abilities in order, and
    /// the check here is that the row's ids are the entries' objects.
    fn llm(view: &GameView, legal: &LegalActions, at: &str) -> Surface {
        let (entries, action_rows) = LlmPlayer::build_action_rows(view, legal);
        let mut rows = Vec::new();
        let mut next = 0;
        for row in &action_rows {
            match row {
                ActionRow::One(label) => {
                    rows.push(label.clone());
                    next += 1;
                }
                ActionRow::Copies { label, ids } => {
                    let members: Vec<String> = ids.iter().map(|id| format!("#{}", id.0)).collect();
                    rows.push(format!("{label} — one per copy: {}", members.join(", ")));
                    for id in ids {
                        let entry = entries.get(next).unwrap_or_else(|| panic!(
                            "{at}: the LLM row {label:?} names {} copies but the list has only {} entries in all",
                            ids.len(), entries.len()));
                        let LlmEntry::Ability(ab_idx) = entry else {
                            panic!("{at}: a copies row's index {next} is not an ability: {entry:?}");
                        };
                        assert_eq!(legal.activatable_abilities[*ab_idx].object_id, *id,
                            "{at}: the LLM row {label:?} says index {next} is #{} and it is not", id.0);
                        next += 1;
                    }
                }
            }
        }
        assert_eq!(next, entries.len(),
            "{at}: the LLM rows account for {next} display indices and there are {} entries",
            entries.len());
        let entries = entries.into_iter()
            .map(|e| (Entry::from(e), offer_of(legal, e.into(), "LLM", at)))
            .collect();
        Surface { name: "LLM", rows, entries }
    }

    fn cast_keys(&self) -> Vec<CastOfferKey> {
        self.entries.iter().filter_map(|(_, o)| match o {
            Offer::Cast(k) => Some(k.clone()),
            _ => None,
        }).collect()
    }

    fn ability_keys(&self) -> Vec<AbilityOfferKey> {
        self.entries.iter().filter_map(|(_, o)| match o {
            Offer::Ability(k) => Some(*k),
            _ => None,
        }).collect()
    }

    fn other_offers(&self) -> BTreeSet<usize> {
        self.entries.iter().filter_map(|(_, o)| match o {
            Offer::Other(i) => Some(*i),
            _ => None,
        }).collect()
    }

    fn describe(&self) -> String {
        use std::fmt::Write;
        let mut s = format!("{} rows:\n", self.name);
        for (i, row) in self.rows.iter().enumerate() {
            let _ = writeln!(s, "  {i}: {row}");
        }
        s
    }
}

/// The engine's own offers: the distinct cast keys, the distinct
/// activation keys, and the indices of everything else.
struct EngineOffers {
    casts: BTreeSet<CastOfferKey>,
    abilities: BTreeSet<AbilityOfferKey>,
    others: BTreeSet<usize>,
}

fn engine_offers(legal: &LegalActions) -> EngineOffers {
    let mut offers = EngineOffers {
        casts: BTreeSet::new(), abilities: BTreeSet::new(), others: BTreeSet::new(),
    };
    for i in 0..legal.actions.len() {
        match offer_of_action(legal, i) {
            Offer::Cast(k) => { offers.casts.insert(k); }
            Offer::Ability(k) => { offers.abilities.insert(k); }
            Offer::Other(i) => { offers.others.insert(i); }
        }
    }
    offers
}

/// Property (a) for one surface's casts.
///
/// Both surfaces deliberately collapse casts whose rendered label is
/// byte-identical — two Grizzly Bears in hand are one row — so a cast key
/// may be missing from the rows, but only when a kept row is the same
/// card cast the same way: same name, same cost, same zone, same verb.
/// A key missing for any other reason is the #610 shape.
fn casts_match(surface: &Surface, engine: &EngineOffers, legal: &LegalActions, at: &str, both: &str) {
    let kept = surface.cast_keys();
    let mut seen = BTreeSet::new();
    for k in &kept {
        assert!(seen.insert(k.clone()),
            "{at}: the {} offers cast {k:?} twice\n{both}", surface.name);
        assert!(engine.casts.contains(k),
            "{at}: the {} offers cast {k:?}, which the engine does not\n{both}", surface.name);
    }
    let castable = |key: &CastOfferKey| legal.castable_spells.iter()
        .find(|cs| cast_offer_key(cs.object_id, cs.alternative_cost.as_ref()) == *key)
        .unwrap_or_else(|| panic!("{at}: no castable_spells entry for {key:?}\n{both}"));
    for dropped in engine.casts.difference(&seen) {
        let d = castable(dropped);
        let twin = kept.iter().map(castable).any(|k|
            k.name == d.name && k.alternative_cost == d.alternative_cost
                && k.is_flashback == d.is_flashback && k.from_graveyard == d.from_graveyard);
        assert!(twin,
            "{at}: the {} has no row for cast {dropped:?} ({}{}), and no row it kept is the \
             same card cast the same way — a legal option the player cannot choose\n{both}",
            surface.name, d.name,
            crate::cast_cost_note(d).map(|n| format!(", {n}")).unwrap_or_default());
    }
}

/// Property (a) for one surface's activations: no surface collapses
/// these, so the rows' keys are exactly the engine's.
fn abilities_match(surface: &Surface, engine: &EngineOffers, at: &str, both: &str) {
    let keys = surface.ability_keys();
    let distinct: BTreeSet<AbilityOfferKey> = keys.iter().copied().collect();
    assert_eq!(distinct.len(), keys.len(),
        "{at}: the {} offers an activation twice: {keys:?}\n{both}", surface.name);
    let missing: Vec<&AbilityOfferKey> = engine.abilities.difference(&distinct).collect();
    let extra: Vec<&AbilityOfferKey> = distinct.difference(&engine.abilities).collect();
    assert!(missing.is_empty() && extra.is_empty(),
        "{at}: the {} and the engine disagree on the activations offered: \
         missing from the rows {missing:?}, in the rows and not the engine {extra:?}\n{both}",
        surface.name);
}

/// Property (b): no two rows read the same.
fn rows_distinct(surface: &Surface, at: &str, both: &str) {
    let mut seen = BTreeSet::new();
    for row in &surface.rows {
        assert!(seen.insert(row),
            "{at}: two {} rows read exactly alike: {row:?}\n{both}", surface.name);
    }
}

/// Build both menus for one decision and hold them to properties (a),
/// (b) and (c). Returns the two menus so a regression test can count rows.
fn check_parity(view: &GameView, legal: &LegalActions, at: &str) -> (Surface, Surface) {
    let cli = Surface::cli(view, legal, at);
    let llm = Surface::llm(view, legal, at);
    let both = format!("{}{}", cli.describe(), llm.describe());
    let engine = engine_offers(legal);

    // (c) continued: an entry is one offer, and no offer is two entries of
    // the same kind (a cast offered twice is caught in casts_match).
    for s in [&cli, &llm] {
        let mut seen = Vec::new();
        for (e, _) in &s.entries {
            assert!(!seen.contains(e), "{at}: the {} lists entry {e:?} twice\n{both}", s.name);
            seen.push(*e);
        }
    }

    rows_distinct(&cli, at, &both);
    rows_distinct(&llm, at, &both);

    casts_match(&cli, &engine, legal, at, &both);
    casts_match(&llm, &engine, legal, at, &both);
    abilities_match(&cli, &engine, at, &both);
    abilities_match(&llm, &engine, at, &both);

    // Everything that is neither: the surfaces show the engine's list.
    assert_eq!(cli.other_offers(), engine.others,
        "{at}: the CLI's plain rows are not the engine's plain actions\n{both}");
    assert_eq!(llm.other_offers(), engine.others,
        "{at}: the LLM's plain rows are not the engine's plain actions\n{both}");

    // And the two surfaces agree with each other: the same casts kept,
    // after their own label collapses, and the same activations.
    let cli_casts: BTreeSet<CastOfferKey> = cli.cast_keys().into_iter().collect();
    let llm_casts: BTreeSet<CastOfferKey> = llm.cast_keys().into_iter().collect();
    assert_eq!(cli_casts, llm_casts,
        "{at}: the CLI and the LLM seat keep different cast rows\n{both}");
    assert_eq!(cli.ability_keys().len(), llm.ability_keys().len(),
        "{at}: the CLI and the LLM seat offer different numbers of activations\n{both}");

    (cli, llm)
}

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

/// A priority decision with a real choice in it: no combat, resolution or
/// set prompt (those are not menus of offers), not the mulligan, and
/// something on offer besides passing and conceding.
fn is_a_priority_menu(legal: &LegalActions) -> bool {
    legal.combat_prompt.is_none()
        && legal.resolution_prompt.is_none()
        && legal.set_prompt.is_none()
        && !legal.actions.iter().any(|a| matches!(a, Action::MulliganKeep))
        && legal.actions.iter().any(|a| !matches!(a, Action::PassPriority | Action::Concede))
}

/// Seeded random-vs-random games over the coverage decks, driven the way
/// `tests/gui_protocol.rs` drives them, with both menus built and compared
/// at every priority decision before the random seat answers it. Returns
/// how many decisions were compared.
fn compare_over_games(seeds: impl Iterator<Item = u64>) -> u32 {
    let registry = CardRegistry::with_all_cards();
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../decks/coverage/");
    let decks = ["ub-coverage.txt", "rg-coverage.txt", "wb-coverage.txt", "ug-coverage.txt"];
    let mut compared = 0u32;
    for seed in seeds {
        let pick = usize::try_from(seed).expect("a seed fits");
        let d1 = coverage_deck(&format!("{root}{}", decks[pick % decks.len()]));
        let d2 = coverage_deck(&format!("{root}{}", decks[(pick + 1) % decks.len()]));
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
            if decisions > 4000 {
                return Action::AbandonGame;
            }
            let view = GameView::for_player(game_state, acting, &registry);
            if is_a_priority_menu(legal) {
                compared += 1;
                check_parity(&view, legal, &format!("seed {seed}, decision {decisions}, seat {}", acting.0));
            }
            let seat = &mut seats[acting.0 as usize];
            match &legal.combat_prompt {
                Some(p) => seat.choose_combat(p),
                None => seat.choose_action(&view, legal),
            }
        });
    }
    compared
}

#[test]
fn both_surfaces_offer_what_the_engine_offers_in_random_games() {
    // Thirty-two games is a few thousand priority menus in well under half
    // a minute of debug build; a floor so the sweep cannot quietly shrink.
    let compared = compare_over_games(1..=32);
    assert!(compared >= 3000,
        "only {compared} priority menus compared in thirty-two games — the sweep has stopped covering");
}

/// The #610 board: one Geistflame in the graveyard carrying two flashback
/// costs (Past in Flames' granted one and the printed one), the mana
/// already floating so no tap plan tells the rows apart. The CLI keyed on
/// `alternative_cost.is_some()` and showed one row; the LLM seat showed two.
#[test]
fn two_flashback_costs_on_one_card_are_two_rows_on_both_surfaces() {
    use crate::cli::tests::{flashback_of, legal, view};
    use mtg_engine::ids::ObjectId;
    use mtg_engine::types::{Color, ManaCost, ManaSymbol, Step};

    let granted = ManaCost::new(vec![ManaSymbol::Colored(Color::Red)]);
    let printed = ManaCost::new(vec![ManaSymbol::Generic(3), ManaSymbol::Colored(Color::Red)]);
    let cast = |alt: &ManaCost| Action::CastSpell {
        object_id: ObjectId(30), targets: vec![], sacrifice: None,
        exile_count: None, exile_ids: vec![],
        alternative_cost: Some(alt.clone()), tap_plan: vec![],
    };
    let v = view(Step::PrecombatMain, 19, true);
    let mut offer = legal(vec![Action::PassPriority, cast(&granted), cast(&printed), Action::Concede]);
    offer.castable_spells = vec![
        flashback_of(30, "Geistflame", granted, 0),
        flashback_of(30, "Geistflame", printed, 0),
    ];

    let (cli, llm) = check_parity(&v, &offer, "#610 board");
    assert_eq!(cli.cast_keys().len(), 2, "{}", cli.describe());
    assert_eq!(llm.cast_keys().len(), 2, "{}", llm.describe());
}

/// The LLM seat's priority menu carries one row the CLI's never will:
/// `Pass until something happens`, the seat's own "go". It is not an
/// engine action — nothing in `LegalActions` stands for it — so it is not
/// a `DisplayEntry` and the properties above, which hold each surface's
/// rows against the engine's offers, are stated over `build_action_rows`
/// and never see it. That exclusion is deliberate: the page has the same
/// thing as the `f` key and the CLI as its auto-pass mode, a keystroke
/// each, and a row on one surface with a key on the other is parity of
/// the decision, not of the menu text. What this test pins is the shape
/// of what is excluded — exactly one row, right after Pass, the engine's
/// rows after it in their order and unchanged — so a change that made it
/// several rows, or moved it, or let it reach the parity sweep, fails
/// here with the reason written down.
#[test]
fn the_pass_until_row_is_the_llm_seats_own_and_outside_the_parity_sweep() {
    use crate::cli::tests::{legal, view};
    use crate::llm::{MenuEntry, PASS_UNTIL_ROW};
    use mtg_engine::ids::ObjectId;
    use mtg_engine::types::Step;

    let v = view(Step::PrecombatMain, 3, true);
    let offer = legal(vec![
        Action::PassPriority,
        Action::PlayLand { object_id: ObjectId(21) },
        Action::Concede,
    ]);
    // The sweep's own comparison holds on this offer with the row absent
    // from both surfaces.
    let (cli, llm) = check_parity(&v, &offer, "pass-until board");
    assert!(cli.rows.iter().all(|r| !r.contains("Pass until")), "{}", cli.describe());
    assert!(llm.rows.iter().all(|r| !r.contains("Pass until")), "{}", llm.describe());

    let (engine_entries, engine_rows) = LlmPlayer::build_action_rows(&v, &offer);
    let (menu, rows) = LlmPlayer::priority_menu(&v, &offer, true);
    assert_eq!(rows.len(), engine_rows.len() + 1, "one row more than the engine's");
    assert_eq!(menu.len(), engine_entries.len() + 1, "one index more than the engine's");
    assert_eq!(rows[0].label(), "Pass");
    assert_eq!(rows[1].label(), PASS_UNTIL_ROW, "right after Pass");
    assert_eq!(menu[1], MenuEntry::PassUntil);
    assert_eq!(menu.iter().filter(|e| **e == MenuEntry::PassUntil).count(), 1, "exactly one");
    let engine_after: Vec<LlmEntry> = menu.iter().filter_map(|e| match e {
        MenuEntry::Engine(d) => Some(*d),
        MenuEntry::PassUntil => None,
    }).collect();
    assert_eq!(engine_after, engine_entries, "the engine's entries, in their order");
    let labels_after: Vec<&str> = rows.iter().enumerate()
        .filter(|(i, _)| *i != 1).map(|(_, r)| r.label()).collect();
    let engine_labels: Vec<&str> = engine_rows.iter().map(ActionRow::label).collect();
    assert_eq!(labels_after, engine_labels, "the engine's rows, unchanged");

    // Left out, the menu is the engine's rows exactly.
    let (menu, rows) = LlmPlayer::priority_menu(&v, &offer, false);
    assert_eq!(rows.len(), engine_rows.len());
    assert!(menu.iter().all(|e| matches!(e, MenuEntry::Engine(_))));
}

/// The #589 board: three Ulvenwald Mystics with a native index-0 ability,
/// one of them enchanted with an Aura granting a different index-0
/// ability. The LLM seat keyed on `(object, index)` and lost the granted
/// row; the CLI, keyed on the engine's triple, showed all four.
#[test]
fn a_granted_ability_at_a_native_index_is_a_row_on_both_surfaces() {
    use crate::llm::tests::{empty_view, granted_ability_offer};

    let v = empty_view();
    let offer = granted_ability_offer(&[2, 4, 6], &[2], "{B}: Regenerate");

    let (cli, llm) = check_parity(&v, &offer, "#589 board");
    assert_eq!(cli.ability_keys().len(), 4, "{}", cli.describe());
    assert_eq!(llm.ability_keys().len(), 4, "{}", llm.describe());
    // The LLM seat groups the three native copies as one row and keeps the
    // granted ability as its own: Pass, the copies row, the granted row,
    // Concede. The CLI prints one row per copy.
    assert_eq!(llm.rows.len(), 4, "{}", llm.describe());
    assert_eq!(cli.rows.len(), 6, "{}", cli.describe());
}
