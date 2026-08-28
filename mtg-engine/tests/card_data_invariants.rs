//! Cross-cutting invariants over every card in the registry.
//!
//! These replace the per-card `*_card_data` / `*_has_correct_stats` tests that
//! used to sit at the top of each card file. Those read a `CardData` literal
//! and asserted its fields straight back — `power: Some(1)` in the card,
//! `assert_eq!(data.power, Some(1))` in the test. A restatement cannot fail
//! unless somebody edits the card, and then it fails without telling anyone
//! anything they did not already know from the diff. The card file is the
//! source of truth for what a card says; there is no second, independent
//! source here to check it against.
//!
//! What *is* worth asserting is consistency — the relationships between the
//! fields that a typo or a half-finished card breaks, checked across all of
//! them at once so a new card is covered the moment it is registered.

mod common;
use common::*;
use mtg_engine::cards::{CardData, CardRegistry};
use mtg_engine::types::{CardType, Color, Keyword, Step, Supertype};
use std::collections::HashSet;

/// Every card in the registry, by name.
fn all_cards(reg: &CardRegistry) -> Vec<CardData> {
    let mut names: Vec<String> = reg.all_names().iter().map(|s| (*s).to_string()).collect();
    names.sort();
    names
        .iter()
        .map(|n| {
            let id = reg
                .get_id_by_name(n)
                .unwrap_or_else(|| panic!("{n} is in all_names but has no id"));
            reg.card_data(id).unwrap_or_else(|| panic!("{n} has no card data"))
        })
        .collect()
}

/// Guard against a vacuous invariant: an assertion that no card in the set
/// exercises passes for the wrong reason. Each test below states how many
/// cards it actually looked at.
fn assert_covers(n: usize, floor: usize, what: &str) {
    assert!(n >= floor, "only {n} card(s) {what} — this invariant has stopped covering anything");
}

/// Report every offender at once — one failing card should not hide the rest.
fn assert_none(offenders: &[String], what: &str) {
    assert!(
        offenders.is_empty(),
        "{} card(s) {what}:\n  {}",
        offenders.len(),
        offenders.join("\n  ")
    );
}

#[test]
fn every_card_round_trips_through_its_name() {
    let reg = registry();
    let mut offenders = Vec::new();
    for name in reg.all_names() {
        let Some(id) = reg.get_id_by_name(name) else {
            offenders.push(format!("{name}: not findable by its own name"));
            continue;
        };
        match reg.card_data(id) {
            None => offenders.push(format!("{name}: no card data")),
            Some(d) if d.name != name => {
                offenders.push(format!("{name}: registered under a different name ({})", d.name));
            }
            Some(_) => {}
        }
    }
    assert_none(&offenders, "do not round-trip through the registry");
}

#[test]
fn card_names_are_unique() {
    let reg = registry();
    let mut seen = HashSet::new();
    let dupes: Vec<String> = reg
        .all_names()
        .iter()
        .filter(|n| !seen.insert((*n).to_string()))
        .map(|n| (*n).to_string())
        .collect();
    assert_none(&dupes, "are registered twice");
}

#[test]
fn a_card_has_power_and_toughness_exactly_when_it_is_a_creature() {
    let reg = registry();
    let mut offenders = Vec::new();
    let mut creatures = 0;
    for d in all_cards(&reg) {
        let creature = d.card_types.contains(&CardType::Creature);
        let has_pt = d.power.is_some() && d.toughness.is_some();
        if creature && !has_pt {
            offenders.push(format!("{}: creature with power {:?} / toughness {:?}", d.name, d.power, d.toughness));
        }
        if !creature && (d.power.is_some() || d.toughness.is_some()) {
            offenders.push(format!("{}: not a creature but has P/T {:?}/{:?}", d.name, d.power, d.toughness));
        }
        if creature {
            creatures += 1;
        }
    }
    assert_covers(creatures, 100, "are creatures");
    assert_none(&offenders, "disagree about being a creature");
}

#[test]
fn lands_have_no_mana_cost_and_everything_else_has_one() {
    let reg = registry();
    let mut offenders = Vec::new();
    let mut lands = 0;
    for d in all_cards(&reg) {
        let land = d.card_types.contains(&CardType::Land);
        match (land, d.cost.is_some()) {
            (true, true) => offenders.push(format!("{}: a land with a mana cost", d.name)),
            (false, false) => offenders.push(format!("{}: a nonland with no mana cost", d.name)),
            _ => {}
        }
        if land {
            lands += 1;
        }
    }
    assert_covers(lands, 10, "are lands");
    assert_none(&offenders, "have the wrong kind of mana cost");
}

#[test]
fn subtypes_imply_their_card_type() {
    let reg = registry();
    // (subtype, the card type it can only appear on)
    const REQUIRED: &[(&str, CardType)] = &[
        ("Equipment", CardType::Artifact),
        ("Aura", CardType::Enchantment),
        ("Curse", CardType::Enchantment),
    ];
    let mut offenders = Vec::new();
    let mut matched = 0;
    for d in all_cards(&reg) {
        for (sub, ty) in REQUIRED {
            if d.subtypes.iter().any(|s| s == sub) {
                matched += 1;
                if !d.card_types.contains(ty) {
                    offenders.push(format!("{}: {sub} but not {ty:?} ({:?})", d.name, d.card_types));
                }
            }
        }
        // A Curse is a kind of Aura (CR 205.3h) and must say so, or the
        // attachment code that looks for Auras will not see it.
        if d.subtypes.iter().any(|s| s == "Curse") && !d.subtypes.iter().any(|s| s == "Aura") {
            offenders.push(format!("{}: a Curse that is not also an Aura", d.name));
        }
    }
    assert_covers(matched, 20, "carry one of these subtypes");
    assert_none(&offenders, "carry a subtype their card type cannot have");
}

#[test]
fn basic_and_legendary_land_on_the_right_card_types() {
    let reg = registry();
    let mut offenders = Vec::new();
    let mut legendary = 0;
    for d in all_cards(&reg) {
        if d.supertypes.contains(&Supertype::Basic) && !d.card_types.contains(&CardType::Land) {
            offenders.push(format!("{}: Basic but not a land", d.name));
        }
        // CR 205.4a: only permanents (and, in other formats, instants and
        // sorceries we do not have) are legendary.
        if d.supertypes.contains(&Supertype::Legendary) {
            legendary += 1;
            if !d.card_types.iter().any(CardType::is_permanent) {
                offenders.push(format!("{}: Legendary but not a permanent", d.name));
            }
        }
    }
    assert_covers(legendary, 5, "are legendary");
    assert_none(&offenders, "carry a supertype their card type cannot have");
}

#[test]
fn flashback_is_only_on_instants_and_sorceries_and_says_so() {
    let reg = registry();
    let mut offenders = Vec::new();
    let mut with_flashback = 0;
    for d in all_cards(&reg) {
        let Some(cost) = &d.flashback_cost else { continue };
        with_flashback += 1;
        if !d.card_types.iter().any(|t| matches!(t, CardType::Instant | CardType::Sorcery)) {
            offenders.push(format!("{}: flashback on a {:?}", d.name, d.card_types));
        }
        if !d.oracle_text.to_lowercase().contains("flashback") {
            offenders.push(format!("{}: has a flashback cost but its text never mentions it", d.name));
        }
        // CR 702.33a: flashback is an alternative cost, so there has to be one
        // to pay. A free flashback is the "no mana cost" bug in disguise.
        if cost.mana_value() == 0 && cost.symbols.is_empty() {
            offenders.push(format!("{}: flashback for nothing", d.name));
        }
    }
    assert_covers(with_flashback, 10, "have flashback");
    assert_none(&offenders, "declare flashback inconsistently");
}

/// The word a keyword is printed as, for checking it against the oracle text.
fn keyword_word(k: Keyword) -> &'static str {
    match k {
        Keyword::Flying => "flying",
        Keyword::FirstStrike => "first strike",
        Keyword::DoubleStrike => "double strike",
        Keyword::Trample => "trample",
        Keyword::Deathtouch => "deathtouch",
        Keyword::Lifelink => "lifelink",
        Keyword::Vigilance => "vigilance",
        Keyword::Flash => "flash",
        Keyword::Reach => "reach",
        Keyword::Haste => "haste",
        Keyword::Defender => "defender",
        Keyword::Hexproof => "hexproof",
        Keyword::Intimidate => "intimidate",
        Keyword::Menace => "menace",
        Keyword::Indestructible => "indestructible",
    }
}

#[test]
fn every_declared_keyword_is_printed_on_the_card() {
    let reg = registry();
    let mut offenders = Vec::new();
    let mut declared = 0;
    for d in all_cards(&reg) {
        let text = d.oracle_text.to_lowercase();
        for k in &d.keywords {
            declared += 1;
            if !text.contains(keyword_word(*k)) {
                offenders.push(format!("{}: declares {k:?}, which its text never prints", d.name));
            }
        }
    }
    assert_covers(declared, 50, "declare a keyword");
    assert_none(&offenders, "declare a keyword their oracle text does not print");
}

#[test]
fn no_card_declares_the_same_thing_twice() {
    let reg = registry();
    let mut offenders = Vec::new();
    for d in all_cards(&reg) {
        let mut seen = HashSet::new();
        for k in &d.keywords {
            if !seen.insert(*k) {
                offenders.push(format!("{}: keyword {k:?} twice", d.name));
            }
        }
        let mut seen = HashSet::new();
        for s in &d.subtypes {
            if !seen.insert(s.clone()) {
                offenders.push(format!("{}: subtype {s} twice", d.name));
            }
        }
        let mut seen = HashSet::new();
        for t in &d.card_types {
            if !seen.insert(*t) {
                offenders.push(format!("{}: card type {t:?} twice", d.name));
            }
        }
    }
    assert_none(&offenders, "declare something twice");
}

#[test]
fn every_card_has_a_name_a_type_and_rules_text() {
    let reg = registry();
    let mut offenders = Vec::new();
    for d in all_cards(&reg) {
        if d.name.trim().is_empty() {
            offenders.push("<unnamed card>".to_string());
        }
        if d.card_types.is_empty() {
            offenders.push(format!("{}: no card type", d.name));
        }
        // A vanilla creature is the only thing allowed to say nothing.
        let vanilla = d.card_types == vec![CardType::Creature] && d.keywords.is_empty();
        if d.oracle_text.trim().is_empty() && !vanilla {
            offenders.push(format!("{}: no oracle text", d.name));
        }
    }
    assert_none(&offenders, "are missing something every card has");
}

#[test]
fn every_triggered_ability_describes_itself() {
    let reg = registry();
    let mut offenders = Vec::new();
    let mut triggers = 0;
    for d in all_cards(&reg) {
        for a in &d.triggered_abilities {
            triggers += 1;
            if a.description.trim().is_empty() {
                offenders.push(format!("{}: a {:?} trigger with no description", d.name, a.kind));
            }
        }
    }
    assert_covers(triggers, 80, "declare a triggered ability");
    assert_none(&offenders, "have an undescribed triggered ability");
}

/// A triggered ability that targets must say so.
///
/// The engine chooses a trigger's targets as it goes on the stack (CR 603.3d).
/// If the ability's `target_requirement` is `None` the engine pushes it
/// untargeted, and the card is left to pick something at resolution — which is
/// both the wrong time and invisible to the "no legal target, no trigger" rule
/// (CR 603.3c). Was four hand-listed cards; the declaration itself says which
/// abilities target, so ask all of them.
#[test]
fn a_triggered_ability_whose_text_targets_declares_a_target_requirement() {
    let reg = registry();
    let mut targeting = 0;
    let mut offenders = Vec::new();

    for d in all_cards(&reg) {
        for ability in &d.triggered_abilities {
            let text = ability.description.to_lowercase();
            // "target" in the ability's own description is the declaration that
            // it targets. "that creature" / "enchanted player" do not target.
            if !text.contains("target") {
                continue;
            }
            targeting += 1;
            if ability.target_requirement.is_none() {
                offenders.push(format!(
                    "{}: its {:?} ability says {:?} but declares no target_requirement",
                    d.name, ability.kind, ability.description));
            }
        }
    }
    assert_covers(targeting, 8, "declare a targeting trigger");
    assert_none(&offenders, "have a targeting trigger that does not declare its target");
}

/// A card's declared trigger kinds must match what its text says it watches.
///
/// Creepy Doll had a per-card test asserting it declares
/// `DealsCombatDamageToCreature` and NOT `Blocks` / `BecomesBlocked` — a real
/// constraint, written out for one card. The oracle text says which event a
/// trigger watches, so the constraint generalises: if the text says "deals
/// combat damage to a creature", the declaration has to say so too.
#[test]
fn a_triggers_declared_kind_matches_what_its_text_watches() {
    use mtg_engine::cards::TriggerKind;

    // (phrase in the ability's own description, the kind it must be declared as)
    const SAYS: &[(&str, TriggerKind)] = &[
        ("deals combat damage to a creature", TriggerKind::DealsCombatDamageToCreature),
        ("at the beginning of your upkeep", TriggerKind::Upkeep),
        ("at the beginning of each upkeep", TriggerKind::Upkeep),
        ("at the beginning of your end step", TriggerKind::EndStep),
        ("when this creature dies", TriggerKind::SelfDies),
        ("when this creature enters", TriggerKind::EntersBattlefield),
        ("when this permanent enters", TriggerKind::EntersBattlefield),
    ];

    let reg = registry();
    let mut matched = 0;
    let mut offenders = Vec::new();

    for d in all_cards(&reg) {
        let text = d.oracle_text.to_lowercase();
        for (phrase, kind) in SAYS {
            if !text.contains(phrase) {
                continue;
            }
            matched += 1;
            let front = d.triggered_abilities.iter().any(|a| a.kind == *kind);
            let back = reg
                .get_id_by_name(&d.name)
                .and_then(|id| reg.get(id))
                .and_then(|b| b.back_face_data())
                .is_some_and(|back| back.triggered_abilities.iter().any(|a| a.kind == *kind));
            if !front && !back {
                offenders.push(format!(
                    "{}: text says {phrase:?} but no {kind:?} trigger is declared",
                    d.name));
            }
        }
    }
    assert_covers(matched, 25, "have text naming one of these trigger events");
    assert_none(&offenders, "declare a trigger kind that does not match their text");
}

/// Every card that has a back face declares it.
///
/// `data/oracle_cache.json` is the independent source here — it is fetched from
/// Scryfall, not written alongside the card — so this is a real cross-check
/// rather than a restatement.
///
/// A card that skips `back_face_data()` and models its second face by branching
/// on `is_transformed` still *behaves* like the back face, which is why this is
/// easy to miss: what breaks is every characteristics read. `face_data` falls
/// through to the front face, so `name_of` gives the front face's name and the
/// oracle text stays the front face's — reaching the legend rule (CR 704.5j),
/// the log, and anything matching on names. Garruk Relentless was written that
/// way, with the back face's name hand-written into `obj.name` on transform,
/// which covered the displays that read the cache and nothing that read the
/// card.
#[test]
fn every_card_with_a_back_face_declares_it() {
    let raw = std::fs::read_to_string("../data/oracle_cache.json")
        .expect("oracle cache is checked in at data/oracle_cache.json");

    // Cards are keyed at four-space indent; a back face is a six-space
    // `"back_face": {` whose first entry is that face's name.
    let mut expected: Vec<(String, String)> = Vec::new();
    let mut current: Option<String> = None;
    let mut lines = raw.lines().peekable();
    while let Some(line) = lines.next() {
        if let Some(rest) = line.strip_prefix("    \"") {
            if let Some(end) = rest.find("\": {") {
                current = Some(rest[..end].to_string());
            }
        }
        if line.trim_start().starts_with("\"back_face\": {") {
            if let (Some(front), Some(name_line)) = (current.clone(), lines.peek()) {
                let t = name_line.trim();
                if let Some(rest) = t.strip_prefix("\"name\": \"") {
                    if let Some(end) = rest.find('"') {
                        expected.push((front, rest[..end].to_string()));
                    }
                }
            }
        }
    }

    let reg = registry();
    let mut offenders = Vec::new();
    let mut checked = 0;
    for (front, back_name) in &expected {
        let Some(behavior) = reg.get_id_by_name(front).and_then(|id| reg.get(id)) else {
            continue; // not implemented in this pool
        };
        checked += 1;
        match behavior.back_face_data() {
            None => offenders.push(format!(
                "{front}: Scryfall gives it a back face ({back_name}) but the card \
                 declares no back_face_data()")),
            Some(back) if back.name != *back_name => offenders.push(format!(
                "{front}: declares back face {:?}, Scryfall says {back_name:?}", back.name)),
            Some(_) => {}
        }
    }

    assert_covers(checked, 15, "have a back face in the oracle cache");
    assert_none(&offenders, "declare the back face the oracle cache gives them");
}

/// CR 111.4: "If the spell or ability doesn't specify the name of the token,
/// its name is the same as its subtype(s) plus the word 'Token.'"
///
/// No card in this set names a token, so every token it makes should be named
/// that way. This mattered because five cards make a 1/1 white flying Spirit
/// and they did not agree what to call it — four said `Spirit`, Moorland Haunt
/// said `Spirit Token`. Two cards match creatures *by name* (Sever the
/// Bloodline, Evil Twin's granted ability), so a Sever aimed at one kind of
/// Spirit token would have missed the other.
#[test]
fn tokens_are_named_after_their_subtypes() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let cases: &[(&str, &[&str])] = &[
        ("Spirit Token", &["Spirit"]),
        ("Zombie Token", &["Zombie"]),
        ("Wolf Token", &["Wolf"]),
        ("Human Soldier Token", &["Human", "Soldier"]),
    ];

    for (expected, subtypes) in cases {
        let ids = state.create_token_with_subtypes(
            "", P0, 1, 1, vec![Color::White], vec![CardType::Creature], vec![],
            subtypes.iter().map(|s| (*s).to_string()).collect(), &reg);
        assert_eq!(state.get_object(ids[0]).unwrap().name, *expected,
            "a token with subtypes {subtypes:?} is named {expected}");
    }

    // A token the effect *does* name keeps that name (CR 111.4's other half).
    let ids = state.create_token_with_subtypes(
        "Boo", P0, 1, 1, vec![Color::Red], vec![CardType::Creature], vec![],
        vec!["Hamster".into()], &reg);
    assert_eq!(state.get_object(ids[0]).unwrap().name, "Boo",
        "a named token keeps its given name");
}

/// No card hardcodes a token name that the engine would derive anyway — that
/// duplication is what let the set disagree with itself about Spirit tokens.
#[test]
fn no_card_hardcodes_a_derivable_token_name() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/cards");
    let mut offenders = Vec::new();
    let mut files = Vec::new();
    let mut stack = vec![src.clone()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap().flatten() {
            let p = entry.path();
            if p.is_dir() { stack.push(p); }
            else if p.extension().is_some_and(|e| e == "rs") { files.push(p); }
        }
    }
    files.sort();
    for path in files {
        let text = std::fs::read_to_string(&path).unwrap();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        for (i, window) in text.split("create_token_with_subtypes(").skip(1).enumerate() {
            let head: String = window.chars().take(60).collect();
            let first_arg = head.trim_start().trim_start_matches('\n').trim_start();
            if first_arg.starts_with('"') && !first_arg.starts_with("\"\"") {
                offenders.push(format!("{name}: token call #{} passes a literal name: {}",
                    i + 1, first_arg.lines().next().unwrap_or("")));
            }
        }
    }
    assert!(offenders.is_empty(),
        "{} card(s) name a token the engine derives from its subtypes:\n  {}\n\n\
         Pass \"\" and let CR 111.4 name it.",
        offenders.len(), offenders.join("\n  "));
}

/// `GameState::objects` is a `HashMap`, and its iteration order is seeded per
/// process. Card code that scans it directly gets a different order on every
/// run of the same game, which shows up in three ways: a list of options
/// offered to a player by position (Curse of the Pierced Heart's planeswalkers,
/// Divine Reckoning's creatures), a `find`/`any` that stops at the first match,
/// and a log that reports the same board in a different order.
///
/// The reach is the whole crate, because every layer of it has an
/// order-sensitive scan: which creature the player is offered first, which of
/// two simultaneous triggers goes on the stack first, which of two creatures
/// dying together is reported first, which state trigger fires when several
/// are ready at once. Everything goes through the accessors that sort by id —
/// `objects_in_zone`, `all_objects_in_zone`, `objects_in_id_order`.
///
/// `state.rs` is exempt: it is where those accessors are built, and where the
/// genuinely order-free walks live (summing continuous effects over every
/// source reaches the same total in any order).
#[test]
fn nothing_iterates_the_object_map_in_map_order() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    // The whole crate, except `state.rs` — that is where the sorted accessors
    // live, and where the order-free walks (summing continuous effects) belong.
    let mut stack = vec![root.join("src")];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap().flatten() {
            let p = entry.path();
            if p.is_dir() { stack.push(p); }
            else if p.extension().is_some_and(|e| e == "rs") { files.push(p); }
        }
    }
    files.sort();

    let mut offenders = Vec::new();
    for path in files {
        let text = std::fs::read_to_string(&path).unwrap();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        if name == "state.rs" {
            continue;
        }
        for (n, line) in text.lines().enumerate() {
            let code = line.trim_start();
            if code.starts_with("//") || code.starts_with("///") {
                continue; // the accessors' own doc comments name the pattern
            }
            if code.contains("objects.values()")
                || code.contains("objects.iter()")
                || code.contains("objects.keys()")
            {
                offenders.push(format!("{name}:{}: {}", n + 1, code));
            }
        }
    }
    assert!(offenders.is_empty(),
        "{} site(s) iterate the raw object map, whose order is \
         seeded per process:\n  {}\n\n\
         Use objects_in_zone / all_objects_in_zone / objects_in_id_order, \
         which sort by id.",
        offenders.len(), offenders.join("\n  "));
}

/// "X transforms into Y" is written once, by `helpers::apply_transform`, which
/// is where the flip happens and where both names are known.
///
/// Nineteen cards used to write it themselves around that call. They said it
/// on the paths where `apply_transform` refuses to flip (a token copy of a
/// double-faced card, CR 111.7), several hardcoded both face names so a rename
/// would leave the log lying, and one said only "Transforms into Stalking
/// Vampire" without naming the permanent at all.
#[test]
fn no_card_announces_its_own_transform() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/cards");
    let mut files = Vec::new();
    let mut stack = vec![src];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap().flatten() {
            let p = entry.path();
            if p.is_dir() { stack.push(p); }
            else if p.extension().is_some_and(|e| e == "rs") { files.push(p); }
        }
    }
    files.sort();

    let mut offenders = Vec::new();
    for path in files {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        if name == "helpers.rs" {
            continue; // this is the one place it is written
        }
        for (n, line) in std::fs::read_to_string(&path).unwrap().lines().enumerate() {
            let code = line.trim_start();
            if code.starts_with("//") {
                continue;
            }
            if code.contains("transforms into") || code.contains("transforms back") {
                offenders.push(format!("{name}:{}: {}", n + 1, code));
            }
        }
    }
    assert!(offenders.is_empty(),
        "{} card(s) announce their own transform:\n  {}\n\n\
         `helpers::apply_transform` logs it, and only when the flip happens.",
        offenders.len(), offenders.join("\n  "));
}

/// An ability's "you" is its source's controller, and CR 608.2g says that is
/// the *last known* controller once the source has left the battlefield —
/// which `helpers::controller_of` answers.
///
/// Reading `o.controller` off the source instead is wrong in exactly the case
/// cards keep commenting about (CR 113.7a, "the ability still resolves if the
/// source is destroyed in response"), because leaving the battlefield resets
/// `controller` to `owner`. Curse of the Pierced Heart handed the choice to
/// the owner; Curiosity offered the draw to the owner. The `PlayerId(0)`
/// fallback these sites carried also silently named a real player.
///
/// Comparisons of one object's controller against another's are a different
/// question and are not what this looks for.
#[test]
fn no_card_reads_its_sources_controller_by_hand() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/cards");
    let mut files = Vec::new();
    let mut stack = vec![src];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap().flatten() {
            let p = entry.path();
            if p.is_dir() { stack.push(p); }
            else if p.extension().is_some_and(|e| e == "rs") { files.push(p); }
        }
    }
    files.sort();

    let mut offenders = Vec::new();
    for path in files {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        if name == "helpers.rs" {
            continue; // where controller_of lives
        }
        for (n, line) in std::fs::read_to_string(&path).unwrap().lines().enumerate() {
            let code = line.trim_start();
            if code.starts_with("//") {
                continue;
            }
            // The two idioms that mean "the controller of this ability's
            // source": a PlayerId(0) fallback, or an unwrap.
            let hand_rolled = code.contains("|o| o.controller")
                && (code.contains("PlayerId(0)") || code.contains(".unwrap()"));
            if hand_rolled {
                offenders.push(format!("{name}:{}: {}", n + 1, code));
            }
        }
    }
    assert!(offenders.is_empty(),
        "{} site(s) read a source's controller by hand:\n  {}\n\n\
         Use `helpers::controller_of`, which answers CR 608.2g.",
        offenders.len(), offenders.join("\n  "));
}

/// Counters are game objects' state, not a card's private bookkeeping, so
/// every card reaches them through the engine: `state.add_counters`,
/// `state.remove_counters`, `state.get_counter_count`.
///
/// Four cards used to reach into `obj.counters` directly, and the shortcuts
/// were not free. `add_counters` refuses to put a counter on a permanent that
/// has left the battlefield (CR 121.1) — a hand-rolled `entry().or_insert(0)`
/// does not, and it also leaves a zero-valued entry behind where the pipeline
/// drops the key. Worse, a card that removes a counter by hand at resolution
/// is almost always removing it on the wrong side of the priority window: if
/// the removal is a cost it belongs in `ActivatedAbilityDef::counter_cost`,
/// which the engine pays on activation and checks for payability first
/// (CR 601.2h, CR 602.2b). Mikaeus, the Lunarch did exactly that.
///
/// `enters_with_counters` builds a list of counters for an ETB replacement
/// effect rather than touching an object, so `e.counters` is not this.
#[test]
fn no_card_reaches_into_the_counter_map_by_hand() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/cards");
    let mut files = Vec::new();
    let mut stack = vec![src];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap().flatten() {
            let p = entry.path();
            if p.is_dir() { stack.push(p); }
            else if p.extension().is_some_and(|e| e == "rs") { files.push(p); }
        }
    }
    files.sort();

    let ops = ["get(", "entry(", "remove(", "insert(", "contains_key(", "get_mut("];
    let mut offenders = Vec::new();
    for path in files {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        for (n, line) in std::fs::read_to_string(&path).unwrap().lines().enumerate() {
            let code = line.trim_start();
            if code.starts_with("//") || code.starts_with("///") {
                continue;
            }
            let Some(rest) = code.split_once(".counters.").map(|(_, r)| r) else { continue };
            if ops.iter().any(|op| rest.starts_with(op)) {
                offenders.push(format!("{name}:{}: {}", n + 1, code));
            }
        }
    }
    assert!(offenders.is_empty(),
        "{} site(s) reach into an object's counter map by hand:\n  {}\n\n\
         Use `state.add_counters` / `state.remove_counters` / \
         `state.get_counter_count`, and declare a counter that is part of an \
         activation cost as `ActivatedAbilityDef::counter_cost`.",
        offenders.len(), offenders.join("\n  "));
}

/// "Who controls my source" and "is my source still on the battlefield" are
/// two questions, and cards kept asking them with one expression:
///
/// ```ignore
/// let controller = match state.get_object(self_id) {
///     Some(o) if o.zone == Zone::Battlefield => o.controller,
///     _ => return,
/// };
/// ```
///
/// It reads as the first and behaves as the second, so an ability whose effect
/// has nothing to do with its source silently did nothing when the source was
/// removed in response to it — against CR 113.7a, which is the whole point of
/// an ability existing on the stack independently of the object it came from.
/// Hamlet Captain stopped pumping the rest of the team; Ghoulraiser stopped
/// returning a Zombie, so removal in response ate the card advantage as well
/// as the body. And once the source *has* left, `o.controller` is reset to
/// `o.owner`, so the read is wrong on its own terms too (CR 608.2g).
///
/// Ask them separately: `helpers::controller_of` for the first,
/// `helpers::still_on_battlefield` for the second. Most effects need neither
/// guard — `add_counters` and `apply_transform` already decline on a permanent
/// that is not there.
///
/// The methods listed in `FUNCTIONS_ON_THE_BATTLEFIELD` are exempt, because
/// for them the battlefield check is the correct question rather than a
/// smuggled one: a static ability, a replacement effect and the list of
/// abilities a permanent offers all function only while the permanent is on
/// the battlefield (CR 113.6), unlike an ability already on the stack.
#[test]
fn no_card_conflates_its_controller_with_still_being_on_the_battlefield() {
    /// Hooks that answer "what is true of this permanent right now", not
    /// "resolve this ability".
    const FUNCTIONS_ON_THE_BATTLEFIELD: &[&str] = &[
        "replace_event", "activated_abilities", "continuous_effects",
        "is_valid_target", "dynamic_pt", "should_trigger",
        "should_trigger_on_blocks", "should_trigger_on_becomes_blocked",
        "state_trigger_condition", "should_transform",
    ];

    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/cards");
    let mut files = Vec::new();
    let mut stack = vec![src];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap().flatten() {
            let p = entry.path();
            if p.is_dir() { stack.push(p); }
            else if p.extension().is_some_and(|e| e == "rs") { files.push(p); }
        }
    }
    files.sort();

    let mut offenders = Vec::new();
    let mut scanned = 0usize;
    for path in files {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        if name == "helpers.rs" {
            continue; // where both helpers live, and the doc comment shows the idiom
        }
        let text = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        let mut current_fn = String::new();
        for (n, line) in lines.iter().enumerate() {
            let code = line.trim_start();
            if let Some(rest) = code.strip_prefix("fn ") {
                current_fn = rest.split('(').next().unwrap_or("").to_string();
            }
            if code.starts_with("//") {
                continue;
            }
            if !code.contains("o.zone == Zone::Battlefield") || !code.contains("=>") {
                continue;
            }
            scanned += 1;
            if FUNCTIONS_ON_THE_BATTLEFIELD.contains(&current_fn.as_str()) {
                continue;
            }
            // Whose object is being matched on? Only the ability's own source
            // counts; a guard on a *target* is CR 608.2b and belongs there.
            let subject_is_source = lines[n.saturating_sub(1)].contains("state.get_object(self_id)")
                || lines[n.saturating_sub(1)].contains("state.get_object(object_id)");
            if !subject_is_source {
                continue;
            }
            // Comparing one object's controller against another's is a
            // different question, so only a bare `o.controller` counts.
            let yields_controller = code.match_indices("o.controller").any(|(i, _)| {
                let after = code[i + "o.controller".len()..].trim_start();
                let before = code[..i].trim_end();
                !after.starts_with("==") && !before.ends_with("==")
            });
            if yields_controller {
                offenders.push(format!("{name}:{}: {}", n + 1, code));
            }
        }
    }
    assert!(scanned >= 5,
        "only {scanned} battlefield-guarded match arm(s) in src/cards — this invariant has stopped covering anything");
    assert!(offenders.is_empty(),
        "{} site(s) read a controller through a battlefield guard:\n  {}\n\n\
         Split them: `helpers::controller_of` for who \"you\" is (CR 608.2g), \
         and `helpers::still_on_battlefield` only if the effect genuinely \
         needs the permanent to be there.",
        offenders.len(), offenders.join("\n  "));
}

/// A card resolving one of its own abilities must not read `o.controller` off
/// its source.
///
/// Two rules say so, and they agree. CR 608.2g: an ability that resolves after
/// its source has left the battlefield uses the source's *last known*
/// controller — and leaving the battlefield resets `controller` to `owner`, so
/// the field being read is not that. CR 602.2a: an *activated* ability's
/// controller is the player who activated it, which is not the source's
/// controller either if someone took the permanent in response.
/// `helpers::controller_of` answers the first, `helpers::ability_controller`
/// the second (and falls through to the first).
///
/// Twenty-five sites read the raw field. Most carried a comment saying exactly
/// the rule they were breaking — "triggered ability resolves even if source
/// has left the battlefield", "'your' means last-known controller, not owner"
/// — above a `match` that returned the owner. Moldgraf Monstrosity's is a dies
/// trigger, so its source is *always* in the graveyard by the time it reads
/// the field, and "return two creature cards from your graveyard" looked in
/// the owner's. And every one of them paired the read with `None => return`,
/// throwing the whole effect away if the source had gone, against CR 113.7a.
///
/// Exempt are the hooks that answer "what is true of this permanent right
/// now" rather than resolving anything: a static or replacement effect and a
/// trigger *condition* are evaluated while the source is on the battlefield
/// (CR 113.6), where the two answers coincide, and the enters-tapped check on
/// the dual lands runs on the land as it enters.
#[test]
fn no_card_reads_its_controller_off_its_own_source_while_resolving() {
    const FUNCTIONS_ON_THE_BATTLEFIELD: &[&str] = &[
        "replace_event", "activated_abilities", "continuous_effects",
        "is_valid_target", "dynamic_pt", "should_trigger",
        "should_trigger_on_blocks", "should_trigger_on_becomes_blocked",
        "should_trigger_on_spell_cast", "state_trigger_condition",
        "should_transform", "pay_activation_cost", "mana_abilities",
        "card_data", "back_face_data", "step_trigger_scope", "loyalty_abilities",
        // The dual lands' "unless you control a Mountain or a Plains" check,
        // run as the land itself enters (CR 614.1c).
        "controller_has_matching_land",
    ];

    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/cards");
    let mut files = Vec::new();
    let mut stack = vec![src];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap().flatten() {
            let p = entry.path();
            if p.is_dir() { stack.push(p); }
            else if p.extension().is_some_and(|e| e == "rs") { files.push(p); }
        }
    }
    files.sort();

    let mut offenders = Vec::new();
    let mut scanned = 0usize;
    for path in files {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        if name == "helpers.rs" {
            continue; // where both helpers live
        }
        let text = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        let mut current_fn = String::new();
        for (n, line) in lines.iter().enumerate() {
            let code = line.trim_start();
            if let Some(rest) = code.strip_prefix("fn ") {
                current_fn = rest.split('(').next().unwrap_or("").to_string();
            }
            if code.starts_with("//") || !code.contains("o.controller") {
                continue;
            }
            scanned += 1;
            if FUNCTIONS_ON_THE_BATTLEFIELD.contains(&current_fn.as_str()) {
                continue;
            }
            // Whose controller? Only the ability's own source is this rule; a
            // read off a *target* is that target's business, and Ghost Quarter
            // makes one.
            let prev = lines[n.saturating_sub(1)];
            let subject_is_source = prev.contains("state.get_object(object_id)")
                || prev.contains("state.get_object(self_id)");
            // A comparison against a controller already in hand is a different
            // question again — Olivia Voldaren checks one against the recorded
            // activator.
            let is_comparison = code.contains("o.controller ==") || code.contains("== o.controller");
            if subject_is_source && !is_comparison {
                offenders.push(format!("{name}:{}: fn {current_fn}: {code}", n + 1));
            }
        }
    }
    assert!(scanned >= 20,
        "only {scanned} controller read(s) in src/cards — this invariant has \
         stopped covering anything");
    assert!(offenders.is_empty(),
        "{} site(s) read the source's own `controller` field while resolving:\n  {}\n\n\
         Use `helpers::ability_controller` for an activated ability (CR 602.2a) \
         and `helpers::controller_of` everywhere else (CR 608.2g). Neither \
         needs a `None => return`, which CR 113.7a forbids anyway.",
        offenders.len(), offenders.join("\n  "));
}

/// Equip is one rules action (CR 702.6b) and it was written out eleven times,
/// once per Equipment in the set: four identical lines to set `attached_to`,
/// and above them a byte-identical `is_valid_target` in ten of them.
///
/// The duplication was not free. The engine's CR 608.2b re-check runs
/// `is_target_legal` plus the card's own `is_valid_target`, and for
/// `CreatureWithFilter` the former only re-runs the *filter* — it accepts a
/// target in the Stack zone and asks nothing about creature-ness. So the
/// legality check at the moment of attaching was each card's to remember, with
/// no shared place for it or for CR 301.5c (an Equipment that is also a
/// creature does not become attached) to live.
///
/// `helpers::resolve_equip` is that place. Auras have had `helpers::resolve_aura`
/// all along; this is its counterpart.
#[test]
fn no_equipment_attaches_itself_by_hand() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/cards");
    let mut files = Vec::new();
    let mut stack = vec![src];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap().flatten() {
            let p = entry.path();
            if p.is_dir() { stack.push(p); }
            else if p.extension().is_some_and(|e| e == "rs") { files.push(p); }
        }
    }
    files.sort();

    let mut offenders = Vec::new();
    for path in files {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        if name == "helpers.rs" {
            continue; // where resolve_aura and resolve_equip live
        }
        for (n, line) in std::fs::read_to_string(&path).unwrap().lines().enumerate() {
            let code = line.trim_start();
            if code.starts_with("//") {
                continue;
            }
            // Player attachment is a different question: Bitterheart Witch
            // puts a Curse onto the battlefield attached to a player straight
            // out of a library, which is CR 303.4h rather than a curse spell
            // resolving, and has its own "can this player be enchanted" check.
            if code.contains("attached_to = Some") {
                offenders.push(format!("{name}:{}: {}", n + 1, code));
            }
        }
    }
    assert!(offenders.is_empty(),
        "{} card(s) attach a permanent to a creature by hand:\n  {}\n\n\
         Use `helpers::resolve_equip` (CR 702.6b) or `helpers::resolve_aura`, \
         which check the target is still legal where the attachment happens.",
        offenders.len(), offenders.join("\n  "));
}

/// Paying a mana cost is the engine's: `pay_cost_with_sources` (or
/// `plan_autotap_for_cost` and `execute_tap_plan_and_pay`, which are the same
/// thing in two steps). Both tap lands for the mana, which CR 601.2g requires
/// and a player expects — "you may pay {1}" with an empty pool and four
/// untapped Plains has to be payable.
///
/// Mentor of the Meek walked the mana pool by hand instead — colorless first,
/// then WUBRG — spending a floating unit if it found one and quietly doing
/// nothing if it did not. Saying "yes" with lands untapped paid nothing and
/// drew nothing. Screeching Bat, the set's other "you may pay", has always
/// gone through the engine.
#[test]
fn no_card_spends_mana_out_of_the_pool_by_hand() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/cards");
    let mut files = Vec::new();
    let mut stack = vec![src];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap().flatten() {
            let p = entry.path();
            if p.is_dir() { stack.push(p); }
            else if p.extension().is_some_and(|e| e == "rs") { files.push(p); }
        }
    }
    files.sort();

    let mut offenders = Vec::new();
    for path in files {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        for (n, line) in std::fs::read_to_string(&path).unwrap().lines().enumerate() {
            let code = line.trim_start();
            if code.starts_with("//") {
                continue;
            }
            // Reaching past the pool's API into its map. `mana_pool.add` is
            // how a mana ability produces mana and is not this; reading the
            // pool (`mana_pool.get`) to decide whether to offer something is
            // not this either.
            if code.contains("mana_pool.mana") {
                offenders.push(format!("{name}:{}: {}", n + 1, code));
            }
        }
    }
    assert!(offenders.is_empty(),
        "{} site(s) reach into a mana pool's map by hand:\n  {}\n\n\
         Use `engine::pay_cost_with_sources`, which taps lands for the mana \
         (CR 601.2g) rather than only spending what happens to be floating.",
        offenders.len(), offenders.join("\n  "));
}

/// Creating a regeneration shield is `state.add_regeneration_shield`, which
/// refuses a permanent that is not on the battlefield (CR 701.15 — the shield
/// replaces a destruction, and a permanent that has left is a different object
/// that cannot be destroyed).
///
/// Four cards wrote `obj.regeneration_shields += 1` by hand with no such
/// check, and the cleanup step only clears unused shields from permanents on
/// the battlefield — so a creature destroyed in response to its own
/// "{B}: Regenerate this creature" kept the shield through the graveyard and
/// came back from a reanimation with a free regeneration.
#[test]
fn no_card_creates_a_regeneration_shield_by_hand() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/cards");
    let mut files = Vec::new();
    let mut stack = vec![src];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap().flatten() {
            let p = entry.path();
            if p.is_dir() { stack.push(p); }
            else if p.extension().is_some_and(|e| e == "rs") { files.push(p); }
        }
    }
    files.sort();

    let mut offenders = Vec::new();
    for path in files {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        for (n, line) in std::fs::read_to_string(&path).unwrap().lines().enumerate() {
            let code = line.trim_start();
            if code.starts_with("//") {
                continue;
            }
            if code.contains("regeneration_shields") {
                offenders.push(format!("{name}:{}: {}", n + 1, code));
            }
        }
    }
    assert!(offenders.is_empty(),
        "{} card(s) touch `regeneration_shields` directly:\n  {}\n\n\
         Use `state.add_regeneration_shield`, which refuses a permanent that \
         is no longer on the battlefield.",
        offenders.len(), offenders.join("\n  "));
}

/// `try_destroy` returns whether the permanent actually died, and a card that
/// writes "X destroyed Y" without looking at that answer is writing a line
/// that can be false. Ghost Quarter's ruling says so in as many words: the
/// land's controller searches "even if that land wasn't destroyed... because
/// the land has indestructible or because it was regenerated" — and the log
/// announced a destruction that had not happened.
///
/// `destruction::try_destroy_by` takes the source's name and writes one true
/// line, the same shape as `mill_cards` taking a source. The pipeline's own
/// lines say what happened; this one says who tried.
#[test]
fn no_card_announces_a_destruction_it_did_not_check() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/cards");
    let mut files = Vec::new();
    let mut stack = vec![src];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap().flatten() {
            let p = entry.path();
            if p.is_dir() { stack.push(p); }
            else if p.extension().is_some_and(|e| e == "rs") { files.push(p); }
        }
    }
    files.sort();

    let mut offenders = Vec::new();
    let mut scanned = 0usize;
    for path in files {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let text = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        for (n, line) in lines.iter().enumerate() {
            let code = line.trim_start();
            if code.starts_with("//") || !code.contains("try_destroy") {
                continue;
            }
            scanned += 1;
            // The result is used when it is bound, matched, or compared —
            // `try_destroy_by` does that inside itself.
            let uses_result = code.contains("try_destroy_by")
                || code.contains("let ") || code.contains("match ")
                || code.contains("==") || code.contains("if ");
            if uses_result {
                continue;
            }
            // A bare call is fine on its own; it is only a problem when the
            // card then narrates a destruction it never confirmed.
            let window: String = lines[n..].iter().take(6).copied().collect::<Vec<_>>().join(" ");
            if window.contains("state.log") && window.contains("destroyed") {
                offenders.push(format!("{name}:{}: {}", n + 1, code));
            }
        }
    }
    assert!(scanned >= 5,
        "only {scanned} destruction call(s) in src/cards — this invariant has stopped covering anything");
    assert!(offenders.is_empty(),
        "{} card(s) log a destruction without checking whether it happened:\n  {}\n\n\
         Use `destruction::try_destroy_by`, which names the source and writes \
         the line the result justifies.",
        offenders.len(), offenders.join("\n  "));
}

/// Strip parenthesised reminder text and collapse the leftover whitespace.
///
/// Reminder text is printed on the card but says nothing the rules do not
/// already say, and the set is inconsistent about carrying it — Scryfall gives
/// Gatstaf Howler's intimidate with reminder text and the code writes it
/// without. That difference is not drift worth failing a build over; a changed
/// *rule* is.
fn without_reminder_text(s: &str) -> String {
    let mut out = String::new();
    let mut depth = 0usize;
    for c in s.chars() {
        match c {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Every card's oracle text — both faces — says what Scryfall says.
///
/// `data/oracle_cache.json` is fetched, not written alongside the card, so this
/// is a real cross-check. Cards are errata'd, and a card whose text has drifted
/// is a card being audited against the wrong words: seven back faces still read
/// "transform Ironfang" long after the front faces were updated to "transform
/// this creature", and Ulvenwald Primordials still regenerated itself by name.
/// Nothing behavioural depended on those strings, which is exactly why they sat
/// there — the text is what a reader, a log line, and an audit compare against.
#[test]
fn oracle_text_says_what_scryfall_says() {
    let raw = std::fs::read_to_string("../data/oracle_cache.json")
        .expect("oracle cache is checked in at data/oracle_cache.json");

    // The cache is pretty-printed: cards are keyed at four-space indent, a back
    // face is a nested object, and both carry an "oracle_text" line.
    let mut front: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut back: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut current: Option<String> = None;
    let mut in_back = false;
    for line in raw.lines() {
        if let Some(rest) = line.strip_prefix("    \"") {
            if let Some(end) = rest.find("\": {") {
                current = Some(rest[..end].to_string());
                in_back = false;
            }
        }
        let t = line.trim_start();
        if t.starts_with("\"back_face\": {") {
            in_back = true;
        }
        if let Some(rest) = t.strip_prefix("\"oracle_text\": \"") {
            // The value ends at the closing quote, and exactly one — text that
            // itself ends in an escaped quote (`... this creature.\""`) loses
            // its last character to a greedy trim.
            let raw_text = rest.trim_end().strip_suffix(',').unwrap_or(rest.trim_end());
            let raw_text = raw_text.strip_suffix('"').unwrap_or(raw_text);
            let text = raw_text
                .replace("\\n", "\n")
                .replace("\\\"", "\"")
                .replace("\\u2014", "\u{2014}")
                .replace("\\u2019", "\u{2019}");
            if let Some(name) = current.clone() {
                if in_back { back.insert(name, text); } else { front.insert(name, text); }
            }
        }
    }
    assert!(front.len() > 200, "parsed only {} front texts from the cache", front.len());

    // A basic land's printed text *is* its reminder text — Scryfall gives
    // "({T}: Add {U}.)" and nothing else, because the mana ability is intrinsic
    // (CR 305.6) rather than printed. The cards state it as the ability it is.
    const INTRINSIC_MANA: &[&str] = &["Plains", "Island", "Swamp", "Mountain", "Forest"];

    let reg = registry();
    let mut offenders = Vec::new();
    let mut checked = 0;
    for name in reg.all_names() {
        if INTRINSIC_MANA.contains(&name) {
            continue;
        }
        let Some(id) = reg.get_id_by_name(name) else { continue };
        let Some(data) = reg.card_data(id) else { continue };
        if let Some(want) = front.get(name) {
            checked += 1;
            if without_reminder_text(want) != without_reminder_text(&data.oracle_text) {
                offenders.push(format!(
                    "{name}\n    Scryfall: {:?}\n    card    : {:?}", want, data.oracle_text));
            }
        }
        if let (Some(want), Some(face)) = (back.get(name), reg.get(id).and_then(|b| b.back_face_data())) {
            checked += 1;
            if without_reminder_text(want) != without_reminder_text(&face.oracle_text) {
                offenders.push(format!(
                    "{name} // {}\n    Scryfall: {:?}\n    card    : {:?}",
                    face.name, want, face.oracle_text));
            }
        }
    }
    assert!(checked > 200, "only cross-checked {checked} faces");
    assert!(offenders.is_empty(),
        "{} card face(s) state oracle text the fetched cache disagrees with:\n\n{}\n",
        offenders.len(), offenders.join("\n\n"));
}

/// "at random" is a rules instruction, not flavour: CR 104.3c and the shuffle
/// rules treat a random choice as genuinely random, and a player may not be
/// able to know or influence it. An implementation that picks the first
/// eligible object satisfies every ordinary test for the card — the tests set
/// up one candidate, so a fixed choice and a random one are indistinguishable
/// — and only shows up as a game that always makes the same "random" pick.
///
/// So: if a card's own oracle text says "at random", its file has to reach for
/// an RNG somewhere. That is a weak check on its own, but it is the one that
/// catches the silent case, and the per-card tests check the distribution.
#[test]
fn a_card_that_says_at_random_actually_randomizes() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/cards");
    let mut files = Vec::new();
    let mut stack = vec![src];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap().flatten() {
            let p = entry.path();
            if p.is_dir() { stack.push(p); }
            else if p.extension().is_some_and(|e| e == "rs") { files.push(p); }
        }
    }
    files.sort();

    let mut offenders = Vec::new();
    for path in files {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let text = std::fs::read_to_string(&path).unwrap();
        // The oracle_text field is itself checked against Scryfall by the card
        // data guards, so keying off it does not launder a wrong assumption.
        let says_at_random = text.lines().any(|l| {
            let c = l.trim_start();
            !c.starts_with("//") && c.contains("oracle_text") && c.contains("at random")
        });
        if !says_at_random {
            continue;
        }
        // `choose_at_random` is where five of these cards' RNG went when it was
        // centralised (CR 104.3, one place to seed); calling it is reaching for
        // an RNG just as much as `shuffle(` was.
        let randomizes = ["choose_at_random", "flip_coin",
                          "shuffle(", "choose(", "choose_multiple(", "gen_range", "gen_bool"]
            .iter()
            .any(|needle| text.lines().any(|l| {
                let c = l.trim_start();
                !c.starts_with("//") && c.contains(needle)
            }));
        if !randomizes {
            offenders.push(name);
        }
    }

    assert!(offenders.is_empty(),
        "{} card(s) say \"at random\" in their oracle text but never reach for \
         an RNG:\n  {}\n\n\
         A fixed choice is not a random one, and no single-candidate test can \
         tell the two apart.",
        offenders.len(), offenders.join("\n  "));
}

/// Countering goes through the pipeline, not through `state.stack` directly.
///
/// CR 701.5a makes countering two steps — remove the spell from the stack, and
/// put it into its owner's graveyard (or exile, if it was cast with flashback)
/// — and they must not come apart. Four places wrote both out by hand, and one
/// of them, Dissipate, had already reached its destination without the second:
/// `stack.retain(..)` then `move_object(Exile)`. Exile is the right zone for
/// that card, so it was not a bug, but it is how the next card to want a
/// destination gets one wrong.
///
/// `helpers::counter_spell` / `counter_spell_exiling` own both steps now. This
/// scans card files only; `helpers.rs` is where the retain legitimately lives.
#[test]
fn no_card_removes_a_spell_from_the_stack_itself() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/cards");
    let mut files = Vec::new();
    let mut stack = vec![src];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap().flatten() {
            let p = entry.path();
            if p.is_dir() { stack.push(p); }
            else if p.extension().is_some_and(|e| e == "rs") { files.push(p); }
        }
    }
    files.sort();

    let mut offenders = Vec::new();
    for path in files {
        if path.file_name().is_some_and(|n| n == "helpers.rs") {
            continue;
        }
        let text = std::fs::read_to_string(&path).unwrap();
        if text.lines().map(str::trim_start).filter(|l| !l.starts_with("//"))
            .any(|l| l.contains("stack.retain"))
        {
            offenders.push(path.file_name().unwrap().to_string_lossy().to_string());
        }
    }

    assert!(offenders.is_empty(),
        "{} card(s) edit `state.stack` directly:\n  {}\n\n\
         Countering is `helpers::counter_spell` (CR 701.5a), which removes the \
         stack entry AND disposes of the card — including exiling a spell cast \
         with flashback. Use `counter_spell_exiling` for a card that replaces \
         the destination.",
        offenders.len(), offenders.join("\n  "));
}

/// No card re-decides whether its own permanent is untapped and on the
/// battlefield before offering an activated ability.
///
/// `legal_actions` enumerates battlefield permanents its player controls and
/// rejects a `requires_tap` ability on a tapped one, and applies the
/// summoning-sickness rule (CR 302.6) that no card copy ever did. Nine cards
/// used to open `activated_abilities` with their own version anyway. Being
/// redundant is the mild half; the sharp half is that the guard is written
/// per *card*, not per ability, so the first of those cards to gain a second
/// ability without `{T}` in its cost would have had it silently hidden while
/// the permanent was tapped.
///
/// A card may still read `tapped` legitimately — Skirsdag High Priest's cost
/// is "Tap two untapped creatures you control", so it filters on `!o.tapped`
/// over *other* creatures. The anti-pattern is reading the source's own tapped
/// state, which in all ten copies was `obj.tapped`, off the conventional
/// `let Some(obj) = state.get_object(object_id)` binding, so that is what this
/// matches. Keying on `object_id` appearing anywhere on the line instead
/// flagged Skirsdag, whose filter names it precisely to exclude the source.
///
/// So this is a tripwire for the shape that existed, not a proof that no card
/// can express the idea some other way. A card that reached for the source's
/// tapped state through a different binding would slip past it.
#[test]
fn no_card_re_decides_the_tap_cost_rules_in_activated_abilities() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/cards");
    let mut files = Vec::new();
    let mut stack = vec![src];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap().flatten() {
            let p = entry.path();
            if p.is_dir() { stack.push(p); }
            else if p.extension().is_some_and(|e| e == "rs") { files.push(p); }
        }
    }
    files.sort();

    let mut offenders = Vec::new();
    for path in files {
        let text = std::fs::read_to_string(&path).unwrap();
        let Some(start) = text.find("fn activated_abilities") else { continue };
        // The body runs to the next item at the same indentation.
        let rest = &text[start..];
        let end = rest.find("\n    fn ").map_or(rest.len(), |i| i + 1);
        let body = &rest[..end];
        let hit = body.lines()
            .map(str::trim_start)
            .filter(|l| !l.starts_with("//"))
            .any(|l| l.contains("obj.tapped"));
        if hit {
            offenders.push(path.file_name().unwrap().to_string_lossy().to_string());
        }
    }

    assert!(offenders.is_empty(),
        "{} card(s) inspect `tapped` in `activated_abilities`:\n  {}\n\n\
         Whether a `{{T}}` cost can be paid is `legal_actions`' decision, and it \
         already makes it for every ability with `requires_tap` — including the \
         summoning-sickness half a card-level guard leaves out. A guard written \
         once per card also hides abilities that have no `{{T}}` in their cost.",
        offenders.len(), offenders.join("\n  "));
}

/// `is_valid_target` is only ever consulted for a target the card actually
/// takes. On a card with no target requirement anywhere it is dead code that
/// reads as a restriction — Manor Gargoyle carried one saying "any creature on
/// the battlefield", which is not a rule this card has and would have been the
/// wrong answer the moment it gained a targeted ability.
///
/// A card declares its targets in one of three ways: `target_requirement:
/// Some(..)` on a triggered or activated ability, a `fn target_requirement`
/// override for a spell, or a bare `TargetRequirement::` mention. If none of
/// them appears, nothing can reach `is_valid_target`.
///
/// Except that a card can also get a targeted ability from a shared helper —
/// the eleven Equipment cards get equip, which targets a creature you control,
/// from `helpers::equip_ability`. Scanning the card file alone called every one
/// of them dead the moment that duplication was factored out, so the scan
/// follows one hop: any `helpers.rs` function whose body mentions
/// `TargetRequirement::` hands its targets to whoever calls it.
#[test]
fn no_card_defines_is_valid_target_without_taking_a_target() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/cards");
    let mut files = Vec::new();
    let mut stack = vec![src];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap().flatten() {
            let p = entry.path();
            if p.is_dir() { stack.push(p); }
            else if p.extension().is_some_and(|e| e == "rs") { files.push(p); }
        }
    }
    files.sort();

    // Helper functions that themselves declare a target requirement. A card
    // that calls one takes targets just as surely as one that spells the
    // requirement out.
    let helpers_src = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/cards/helpers.rs")).unwrap();
    let mut helper_bodies: Vec<(String, String)> = Vec::new();
    for line in helpers_src.lines() {
        if let Some(rest) = line.trim_start().strip_prefix("pub fn ") {
            if let Some(name) = rest.split(['(', '<']).next() {
                helper_bodies.push((name.to_string(), String::new()));
            }
        }
        if let Some(last) = helper_bodies.last_mut() {
            last.1.push_str(line);
            last.1.push('\n');
        }
    }
    // A helper that only calls another targeting helper is targeting too —
    // `equip_for_generic` names no requirement of its own, it defers to
    // `equip_ability`. Grow the set until it stops growing.
    let mut targeting_helpers: Vec<String> = Vec::new();
    loop {
        let before = targeting_helpers.len();
        for (name, body) in &helper_bodies {
            if targeting_helpers.contains(name) {
                continue;
            }
            if body.contains("TargetRequirement::")
                || targeting_helpers.iter().any(|h| body.contains(&format!("{h}(")))
            {
                targeting_helpers.push(name.clone());
            }
        }
        if targeting_helpers.len() == before {
            break;
        }
    }
    assert!(!targeting_helpers.is_empty(),
        "found no targeting helpers — the scan below would then be the old \
         card-file-only one, silently");

    let mut offenders = Vec::new();
    for path in files {
        if path.file_name().is_some_and(|n| n == "helpers.rs") {
            continue;
        }
        let text = std::fs::read_to_string(&path).unwrap();
        let code: Vec<&str> = text.lines()
            .map(str::trim_start)
            .filter(|l| !l.starts_with("//"))
            .collect();
        let defines = code.iter().any(|l| l.contains("fn is_valid_target"));
        if !defines {
            continue;
        }
        let takes_targets = code.iter().any(|l| {
            l.contains("target_requirement: Some")
                || l.contains("fn target_requirement")
                || l.contains("TargetRequirement::")
                || targeting_helpers.iter().any(|h| l.contains(&format!("{h}(")))
        });
        if !takes_targets {
            offenders.push(path.file_name().unwrap().to_string_lossy().to_string());
        }
    }

    assert!(offenders.is_empty(),
        "{} card(s) define `is_valid_target` but never take a target:\n  {}\n\n\
         Nothing calls it, so it is dead code that reads as a restriction the \
         card does not have. Delete it.",
        offenders.len(), offenders.join("\n  "));
}

/// Pull one string field per card (and per back face) out of the checked-in
/// oracle cache. Shared by the type-line and mana-cost cross-checks below,
/// which need the same line-based walk `oracle_text_says_what_scryfall_says`
/// does — the cache is pretty-printed JSON and this crate has no JSON parser.
fn cache_field(raw: &str, field: &str) -> (
    std::collections::HashMap<String, String>,
    std::collections::HashMap<String, String>,
) {
    let mut front = std::collections::HashMap::new();
    let mut back = std::collections::HashMap::new();
    let key = format!("\"{field}\": ");
    let mut current: Option<String> = None;
    let mut in_back = false;
    for line in raw.lines() {
        if let Some(rest) = line.strip_prefix("    \"") {
            if let Some(end) = rest.find("\": {") {
                current = Some(rest[..end].to_string());
                in_back = false;
            }
        }
        let t = line.trim_start();
        if t.starts_with("\"back_face\": {") {
            in_back = true;
        }
        if let Some(rest) = t.strip_prefix(key.as_str()) {
            let v = rest.trim_end().strip_suffix(',').unwrap_or(rest.trim_end());
            if v == "null" {
                continue;
            }
            let v = v.trim_matches('"')
                .replace("\\u2014", "\u{2014}")
                .replace("\\u2019", "\u{2019}");
            if let Some(name) = current.clone() {
                if in_back { back.insert(name, v); } else { front.insert(name, v); }
            }
        }
    }
    (front, back)
}

/// Every card's type line — supertypes, card types and subtypes — says what
/// Scryfall says, on both faces.
///
/// `oracle_text_says_what_scryfall_says` already cross-checks the rules text
/// against the same checked-in cache, and `every_card_with_a_back_face_declares_it`
/// the back face's name. The type line was the half nobody compared, and it is
/// the half the engine reads: `subtypes` decides what Slayer of the Wicked can
/// destroy, what Elite Inquisitor has protection from, and which creatures the
/// set's dozen Human-matters cards see. Selfless Cathar is a Human **Cleric**
/// and its file called it a Human Soldier in the comment; nothing would have
/// caught the same slip in the field.
#[test]
fn type_lines_say_what_scryfall_says() {
    let raw = std::fs::read_to_string("../data/oracle_cache.json")
        .expect("oracle cache is checked in at data/oracle_cache.json");
    let (front, back) = cache_field(&raw, "type_line");
    assert!(front.len() > 200, "parsed only {} type lines from the cache", front.len());

    /// Split "Legendary Creature — Human Rogue" into the words before the
    /// dash and the words after it.
    fn split(type_line: &str) -> (Vec<String>, Vec<String>) {
        let (left, right) = match type_line.split_once('\u{2014}') {
            Some((l, r)) => (l, r),
            None => (type_line, ""),
        };
        (left.split_whitespace().map(str::to_string).collect(),
         right.split_whitespace().map(str::to_string).collect())
    }

    fn describe(data: &mtg_engine::cards::CardData) -> (Vec<String>, Vec<String>) {
        let mut left: Vec<String> = data.supertypes.iter().map(|s| format!("{s:?}")).collect();
        left.extend(data.card_types.iter().map(|t| format!("{t:?}")));
        (left, data.subtypes.clone())
    }

    let reg = registry();
    let mut offenders = Vec::new();
    let mut checked = 0;
    let compare = |name: &str, want: &str, data: &mtg_engine::cards::CardData, offenders: &mut Vec<String>| {
        let (want_left, want_right) = split(want);
        let (got_left, got_right) = describe(data);
        // Order is not part of a type line's meaning; membership is.
        let sorted = |mut v: Vec<String>| { v.sort(); v };
        if sorted(want_left.clone()) != sorted(got_left.clone())
            || sorted(want_right.clone()) != sorted(got_right.clone()) {
            offenders.push(format!(
                "{name}\n    Scryfall: {want:?}\n    card    : {:?} \u{2014} {:?}", got_left, got_right));
        }
    };
    for name in reg.all_names() {
        let Some(id) = reg.get_id_by_name(name) else { continue };
        let Some(data) = reg.card_data(id) else { continue };
        if let Some(want) = front.get(name) {
            checked += 1;
            compare(name, want, &data, &mut offenders);
        }
        if let (Some(want), Some(face)) = (back.get(name), reg.get(id).and_then(|b| b.back_face_data())) {
            checked += 1;
            let label = format!("{name} // {}", face.name);
            compare(&label, want, &face, &mut offenders);
        }
    }

    assert_covers(checked, 200, "have a type line in the oracle cache");
    assert_none(&offenders, "declare the type line the oracle cache gives them");
}

/// Every card's mana cost and printed power/toughness say what Scryfall says.
///
/// The rest of the printed characteristics, against the same checked-in cache.
/// A wrong cost is not a cosmetic slip: it is what `cost_to_cast` starts from,
/// what `mana_value` reports, and what every "creature card with mana value N"
/// reads.
#[test]
fn mana_costs_and_printed_pt_say_what_scryfall_says() {
    let raw = std::fs::read_to_string("../data/oracle_cache.json")
        .expect("oracle cache is checked in at data/oracle_cache.json");
    let (costs, _) = cache_field(&raw, "mana_cost");
    let (powers, _) = cache_field(&raw, "power");
    let (toughnesses, _) = cache_field(&raw, "toughness");
    assert!(costs.len() > 200, "parsed only {} mana costs from the cache", costs.len());

    let reg = registry();
    let mut offenders = Vec::new();
    let mut checked = 0;
    for name in reg.all_names() {
        let Some(id) = reg.get_id_by_name(name) else { continue };
        let Some(data) = reg.card_data(id) else { continue };

        if let Some(want) = costs.get(name) {
            checked += 1;
            // A land has no mana cost; Scryfall spells that "".
            let got = data.cost.as_ref().map_or(String::new(), ToString::to_string);
            if want != &got {
                offenders.push(format!("{name}: Scryfall cost {want:?}, card {got:?}"));
            }
        }
        // "*" is a characteristic-defining ability, not a printed number —
        // Boneyard Wurm and Mindshrieker carry it, and the number comes from
        // `dynamic_pt` rather than from `CardData`.
        for (field, cache, got) in [
            ("power", &powers, data.power),
            ("toughness", &toughnesses, data.toughness),
        ] {
            let Some(want) = cache.get(name) else { continue };
            if want.contains('*') {
                continue;
            }
            let Ok(want_n) = want.parse::<i32>() else { continue };
            checked += 1;
            if got != Some(want_n) {
                offenders.push(format!("{name}: Scryfall {field} {want_n}, card {got:?}"));
            }
        }
    }

    assert_covers(checked, 200, "have a mana cost or printed P/T in the oracle cache");
    assert_none(&offenders, "declare the mana cost and P/T the oracle cache gives them");
}

/// Every activated ability's declared cost is the cost its oracle text prints.
///
/// CR 602.1: an activated ability is written "cost: effect". The cost half is
/// right there in the text the `oracle_text_says_what_scryfall_says` invariant
/// already pins to Scryfall, and nothing compared it with the
/// `ActivatedAbilityDef` the engine actually charges. Dropping the {G} from
/// Darkthicket Wolf's "{2}{G}: This creature gets +2/+2" and charging {3}
/// instead passed the whole suite: a card's colour requirement is only ever
/// exercised by a test that happens to pay for it in the right colours.
///
/// Deliberately narrow, so that what it does check it checks exactly: only
/// cards where the text prints exactly one activated ability and the card
/// declares exactly one. A card with two abilities gives no way to say which
/// line belongs to which without guessing, and a guess here would be a test
/// that passes for the wrong reason.
#[test]
fn activated_ability_costs_are_the_costs_the_oracle_text_prints() {
    /// The mana part of a printed cost, or `None` if the prefix is not a cost
    /// at all (so "Flying" or a reminder-text colon is skipped) or contains a
    /// cost this cannot render (sacrifice, discard, counters, life).
    ///
    /// Returns (mana cost as printed, whether the cost includes {T}).
    fn printed_cost(prefix: &str) -> Option<(String, bool)> {
        let mut mana = String::new();
        let mut taps = false;
        for part in prefix.split(',') {
            let part = part.trim();
            if part == "{T}" {
                taps = true;
                continue;
            }
            // A mana-symbol run: "{2}{G}", "{X}{R}{G}".
            if part.starts_with('{') && part.ends_with('}')
                && part.chars().filter(|c| *c == '{').count()
                    == part.chars().filter(|c| *c == '}').count()
            {
                mana.push_str(part);
                continue;
            }
            return None; // "Sacrifice this creature", "Discard a card", ...
        }
        Some((mana, taps))
    }

    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let mut offenders = Vec::new();
    let mut checked = 0;
    for name in reg.all_names() {
        let Some(id) = reg.get_id_by_name(name) else { continue };
        let Some(data) = reg.card_data(id) else { continue };
        let Some(behavior) = reg.get(id) else { continue };

        let printed: Vec<(String, bool)> = data.oracle_text.lines()
            .filter_map(|line| line.split_once(": ").and_then(|(p, _)| printed_cost(p)))
            .collect();
        if printed.len() != 1 {
            continue;
        }
        let object = state.create_object(id, P0, mtg_engine::types::Zone::Battlefield, data.power, data.toughness);
        let abilities = behavior.activated_abilities(&state, object, &reg);
        if abilities.len() != 1 {
            continue;
        }
        checked += 1;
        let (want_mana, want_tap) = &printed[0];
        let got_mana = abilities[0].cost.to_string();
        if &got_mana != want_mana {
            offenders.push(format!(
                "{name}: oracle prints cost {want_mana:?}, ability charges {got_mana:?}"));
        }
        if abilities[0].requires_tap != *want_tap {
            offenders.push(format!(
                "{name}: oracle cost {} {{T}}, ability requires_tap = {}",
                if *want_tap { "includes" } else { "does not include" },
                abilities[0].requires_tap));
        }

        // The two printed restrictions, which are flags rather than costs.
        // These are plain text-to-flag checks, so they hold for a card with
        // one ability without any of the matching problem above.
        for (phrase, got, what) in [
            ("Activate only once each turn", abilities[0].once_per_turn, "once_per_turn"),
            ("Activate only as a sorcery", abilities[0].sorcery_speed_only, "sorcery_speed_only"),
        ] {
            let want = data.oracle_text.contains(phrase);
            if want != got {
                offenders.push(format!(
                    "{name}: oracle text {} {phrase:?}, ability has {what} = {got}",
                    if want { "prints" } else { "does not print" }));
            }
        }
    }

    assert_covers(checked, 15, "print exactly one activated ability");
    assert_none(&offenders, "charge the cost their oracle text prints");
}

/// Every keyword ability a card is printed with is a keyword the card
/// declares, and every keyword it declares is printed.
///
/// The last of the printed characteristics the checked-in oracle cache can
/// settle, after the rules text, the back face, the type line and the mana
/// cost. A missing `Keyword::Trample` is invisible in every test that does not
/// stage the blocked-creature case it changes, and a spurious one is invisible
/// in every test that does not stage its absence: keywords are only ever
/// exercised by the combat or targeting scenario that happens to need them.
///
/// Only the keywords `Keyword` models are compared. Scryfall's list also
/// carries keyword *actions* (Mill, Fight, Proliferate) and mechanics the
/// engine models elsewhere — Flashback as `flashback_cost`, Protection and
/// Enchant as continuous effects, Transform as `back_face_data` — and reading
/// those as missing keywords would be reading the cache wrong, not the card.
#[test]
fn keywords_say_what_scryfall_says() {
    use mtg_engine::types::Keyword;

    /// The Scryfall spelling of every keyword the engine models as one.
    const MODELLED: &[(&str, Keyword)] = &[
        ("Flying", Keyword::Flying),
        ("First strike", Keyword::FirstStrike),
        ("Double strike", Keyword::DoubleStrike),
        ("Trample", Keyword::Trample),
        ("Deathtouch", Keyword::Deathtouch),
        ("Lifelink", Keyword::Lifelink),
        ("Vigilance", Keyword::Vigilance),
        ("Flash", Keyword::Flash),
        ("Reach", Keyword::Reach),
        ("Haste", Keyword::Haste),
        ("Defender", Keyword::Defender),
        ("Hexproof", Keyword::Hexproof),
        ("Intimidate", Keyword::Intimidate),
        ("Menace", Keyword::Menace),
        ("Indestructible", Keyword::Indestructible),
    ];

    let raw = std::fs::read_to_string("../data/oracle_cache.json")
        .expect("oracle cache is checked in at data/oracle_cache.json");

    // Scryfall's `keywords` is a card-level array covering both faces, and it
    // is pretty-printed across several lines.
    let mut listed: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    let mut current: Option<String> = None;
    let mut collecting: Option<Vec<String>> = None;
    for line in raw.lines() {
        if let Some(rest) = line.strip_prefix("    \"") {
            if let Some(end) = rest.find("\": {") {
                current = Some(rest[..end].to_string());
            }
        }
        let t = line.trim();
        if t.starts_with("\"keywords\": [") {
            collecting = Some(Vec::new());
            continue;
        }
        if let Some(items) = collecting.as_mut() {
            if t.starts_with(']') {
                if let Some(name) = current.clone() {
                    // A card-level list and a back face's list both belong to
                    // the same card.
                    listed.entry(name).or_default().extend(std::mem::take(items));
                }
                collecting = None;
                continue;
            }
            let item = t.trim_end_matches(',').trim_matches('"');
            if !item.is_empty() {
                items.push(item.to_string());
            }
        }
    }
    assert!(listed.len() > 100, "parsed only {} keyword lists from the cache", listed.len());

    let reg = registry();
    let mut offenders = Vec::new();
    let mut checked = 0;
    for name in reg.all_names() {
        let Some(id) = reg.get_id_by_name(name) else { continue };
        let Some(data) = reg.card_data(id) else { continue };
        let Some(behavior) = reg.get(id) else { continue };
        let Some(want) = listed.get(name) else { continue };
        checked += 1;

        // Either face may be the one carrying it — Scryfall's list does not
        // say which, and Villagers of Estwald's back face is the flier.
        let back = behavior.back_face_data();
        for (spelling, keyword) in MODELLED {
            let printed = want.iter().any(|k| k == spelling);
            let declared = data.keywords.contains(keyword)
                || back.as_ref().is_some_and(|d| d.keywords.contains(keyword));
            if printed != declared {
                offenders.push(format!(
                    "{name}: Scryfall {} {spelling}, card {} it",
                    if printed { "prints" } else { "does not print" },
                    if declared { "declares" } else { "does not declare" }));
            }
        }
    }

    assert_covers(checked, 100, "have a keyword list in the oracle cache");
    assert_none(&offenders, "declare the keywords the oracle cache gives them");
}

/// Every flashback cost is the cost the card prints.
///
/// "Flashback {5}{B}{B}" is a printed cost like any other, and the one the
/// engine charges lives in `CardData::flashback_cost` where nothing compared
/// it with the text. Turning Sever the Bloodline's flashback into {4}{B}{B}
/// passed the whole suite: a flashback test pays what the card asks and then
/// checks the card was cast and exiled, so it cannot notice the ask changing.
#[test]
fn flashback_costs_are_the_costs_the_oracle_text_prints() {
    let reg = registry();
    let mut offenders = Vec::new();
    let mut checked = 0;
    for name in reg.all_names() {
        let Some(id) = reg.get_id_by_name(name) else { continue };
        let Some(data) = reg.card_data(id) else { continue };

        // "Flashback {5}{B}{B} (You may cast this card...)" — the cost runs to
        // the reminder text or the end of the line.
        let printed = data.oracle_text.lines()
            .find_map(|line| line.trim().strip_prefix("Flashback "))
            .map(|rest| {
                rest.split_whitespace()
                    .take_while(|w| w.starts_with('{'))
                    .collect::<String>()
            });
        match (&printed, &data.flashback_cost) {
            (None, None) => {}
            (Some(want), Some(got)) => {
                checked += 1;
                if &got.to_string() != want {
                    offenders.push(format!(
                        "{name}: prints flashback {want:?}, charges {:?}", got.to_string()));
                }
            }
            (Some(want), None) => offenders.push(format!(
                "{name}: prints flashback {want:?} but declares no flashback_cost")),
            (None, Some(got)) => offenders.push(format!(
                "{name}: declares flashback_cost {:?} but prints no Flashback", got.to_string())),
        }
    }

    assert_covers(checked, 20, "print a flashback cost");
    assert_none(&offenders, "charge the flashback cost their oracle text prints");
}

/// A spell whose text says "any target" declares `TargetRequirement::AnyTarget`,
/// and one that declares it says so.
///
/// CR 115.4a: "any target" means any creature, player, planeswalker or battle.
/// `damage_helper.rs::every_any_target_spell_can_point_at_a_planeswalker`
/// sweeps the registry for cards declaring `AnyTarget` and checks each one
/// offers a planeswalker — but a card that *stopped* declaring it simply drops
/// out of the sweep, and the floor of three that guards the sweep is still met
/// by the others. Narrowing Devil's Play to "target creature" passed the whole
/// suite for exactly that reason: the sweep is derived from the declaration it
/// is meant to check.
///
/// This is the other half, and it reads the oracle text — which
/// `oracle_text_says_what_scryfall_says` pins to Scryfall — so the two cannot
/// drift together.
#[test]
fn any_target_in_the_text_means_any_target_in_the_requirement() {
    use mtg_engine::cards::TargetRequirement;

    let reg = registry();
    let mut offenders = Vec::new();
    let mut checked = 0;
    for name in reg.all_names() {
        let Some(id) = reg.get_id_by_name(name) else { continue };
        let Some(data) = reg.card_data(id) else { continue };
        let Some(behavior) = reg.get(id) else { continue };
        // Only spells: "any target" also appears in activated and triggered
        // abilities (Stensia Bloodhall, Pitchburn Devils), whose requirements
        // live on the ability rather than on `target_requirement`.
        if data.card_types.iter().all(|t|
            !matches!(t, mtg_engine::types::CardType::Instant | mtg_engine::types::CardType::Sorcery)) {
            continue;
        }
        checked += 1;

        let printed = data.oracle_text.contains("any target");
        let declared = matches!(behavior.target_requirement(), TargetRequirement::AnyTarget);
        if printed != declared {
            offenders.push(format!(
                "{name}: text {} \"any target\", requirement {} AnyTarget",
                if printed { "says" } else { "does not say" },
                if declared { "is" } else { "is not" }));
        }
    }

    assert_covers(checked, 40, "are instants or sorceries");
    assert_none(&offenders, "declare AnyTarget exactly when their text says \"any target\"");
}
