//! Every card's printed characteristics are its Oracle record's.
//!
//! For each card in the registry, each face's `CardData` is compared field by
//! field with that face's record in `data/oracle_cache.json` (Scryfall's
//! Oracle data): name, mana cost, colour indicator, supertypes, card types,
//! subtypes, power and toughness, rules text (reminder text aside), the
//! keyword abilities the engine models that the text prints on a line of
//! their own, and the flashback cost the text prints (CR 109.3, 202.1, 204,
//! 205, 207.2a, 208, 702.34a, 712.8).
//!
//! Both sides are finite and fixed, and every card is compared, so this is
//! a complete check of every card's characteristics, not a sample: a card
//! whose `CardData` disagrees with its Oracle record in any of these fields
//! is named, with the field. What a card's abilities *do* is proved in
//! `mtg-engine/src/proofs/cards`.

mod common;
use common::*;
use mtg_engine::cards::CardData;
use mtg_engine::types::{Color, Keyword, ManaCost, ManaSymbol};

/// One face of a card as its Oracle record prints it.
struct Face {
    name: String,
    mana_cost: String,
    type_line: String,
    oracle_text: String,
    power: Option<String>,
    toughness: Option<String>,
    color_indicator: Vec<String>,
}

fn face(v: &serde_json::Value, name: &str) -> Face {
    let s = |k: &str| v[k].as_str().map(str::to_string);
    Face {
        name: name.to_string(),
        mana_cost: s("mana_cost").unwrap_or_default(),
        type_line: s("type_line").unwrap_or_default(),
        oracle_text: s("oracle_text").unwrap_or_default(),
        power: s("power"),
        toughness: s("toughness"),
        color_indicator: v["color_indicator"].as_array().into_iter().flatten()
            .filter_map(|c| c.as_str().map(str::to_string))
            .collect(),
    }
}

/// Each card's front face, and its back face if it has one.
fn oracle() -> Vec<(Face, Option<Face>)> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../data/oracle_cache.json");
    let json: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path).expect("readable"))
        .expect("the oracle cache is JSON");
    json["cards"].as_object().expect("a cards map").iter()
        .map(|(name, c)| {
            let back = c.get("back_face").filter(|b| b.is_object())
                .map(|b| face(b, b["name"].as_str().unwrap_or_default()));
            (face(c, name), back)
        })
        .collect()
}

/// A mana cost as counts: generic, colourless, X, and each colour (CR
/// 107.4, 202.1). Two costs are the same cost when these are the same,
/// whatever order or grouping their symbols are written in.
fn counted(symbols: &[ManaSymbol]) -> (u32, u32, u32, [u32; 5]) {
    let mut out = (0, 0, 0, [0; 5]);
    for s in symbols {
        match s {
            ManaSymbol::Generic(n) => out.0 += n,
            ManaSymbol::Colorless(n) => out.1 += n,
            ManaSymbol::X => out.2 += 1,
            ManaSymbol::Colored(c) => out.3[Color::ALL.iter().position(|k| k == c).expect("a colour")] += 1,
        }
    }
    out
}

/// A printed cost, "{2}{B}{B}", as symbols; None if a symbol is not one the
/// engine has.
fn printed_cost(text: &str) -> Option<Vec<ManaSymbol>> {
    text.split_inclusive('}')
        .map(|sym| {
            let inner = sym.strip_prefix('{')?.strip_suffix('}')?;
            Some(match inner {
                "W" => ManaSymbol::Colored(Color::White),
                "U" => ManaSymbol::Colored(Color::Blue),
                "B" => ManaSymbol::Colored(Color::Black),
                "R" => ManaSymbol::Colored(Color::Red),
                "G" => ManaSymbol::Colored(Color::Green),
                "C" => ManaSymbol::Colorless(1),
                "X" => ManaSymbol::X,
                n => ManaSymbol::Generic(n.parse().ok()?),
            })
        })
        .collect()
}

fn cost_of(cost: Option<&ManaCost>) -> (u32, u32, u32, [u32; 5]) {
    counted(cost.map_or(&[][..], |c| &c.symbols[..]))
}

/// The text with reminder text, in parentheses, taken out (CR 207.2a).
fn without_reminders(text: &str) -> String {
    let mut out = String::new();
    let mut depth = 0;
    for ch in text.chars() {
        match ch {
            '(' => depth += 1,
            ')' => depth -= 1,
            _ if depth == 0 => out.push(ch),
            _ => {}
        }
    }
    out
}

/// The keyword abilities the engine models that `text` prints on a line of
/// their own ("Flying", "First strike, vigilance"); protection rides on such
/// a line without being one of the engine's keywords (CR 702.16).
fn printed_keywords(text: &str) -> Vec<Keyword> {
    let mut out = Vec::new();
    for line in without_reminders(text).lines() {
        let words: Vec<&str> = line.split(',').map(str::trim)
            .filter(|w| !w.to_lowercase().starts_with("protection from "))
            .collect();
        let all: Option<Vec<Keyword>> = words.iter()
            .map(|w| Keyword::ALL.into_iter().find(|k| k.label().eq_ignore_ascii_case(w)))
            .collect();
        if let Some(ks) = all {
            out.extend(ks);
        }
    }
    out.sort_by_key(|k| k.label());
    out.dedup();
    out
}

/// What disagrees between `data` and the Oracle `face`, field by field.
fn disagreements(data: &CardData, face: &Face, front: bool) -> Vec<String> {
    let mut out = Vec::new();
    let mut check = |field: &str, ok: bool, engine: String, oracle: String| {
        if !ok {
            out.push(format!("{field}: the engine has {engine}, the Oracle {oracle}"));
        }
    };
    check("name", data.name == face.name, data.name.clone(), face.name.clone());
    match printed_cost(&face.mana_cost) {
        Some(symbols) => check("mana cost", cost_of(data.cost.as_ref()) == counted(&symbols),
            format!("{:?}", data.cost), face.mana_cost.clone()),
        None => check("mana cost", false, format!("{:?}", data.cost), format!("{} (a symbol the engine lacks)", face.mana_cost)),
    }
    let (types, subtypes) = face.type_line.split_once(" — ").unwrap_or((&face.type_line, ""));
    let mut printed_types: Vec<String> = types.split_whitespace().map(str::to_string).collect();
    let mut engine_types: Vec<String> = data.supertypes.iter().map(|t| t.label().to_string())
        .chain(data.card_types.iter().map(|t| t.label().to_string()))
        .collect();
    printed_types.sort();
    engine_types.sort();
    check("types", engine_types == printed_types, format!("{engine_types:?}"), format!("{printed_types:?}"));
    let printed_subtypes: Vec<String> = subtypes.split_whitespace().map(str::to_string).collect();
    check("subtypes", data.subtypes == printed_subtypes, format!("{:?}", data.subtypes), format!("{printed_subtypes:?}"));
    // A characteristic-defining power or toughness, "*", is 0 on the card
    // and its ability says what it is (CR 208.2a).
    let number = |p: &Option<String>| p.as_deref().map(|s| if s == "*" { 0 } else { s.parse().expect("a number") });
    check("power", data.power == number(&face.power), format!("{:?}", data.power), format!("{:?}", face.power));
    check("toughness", data.toughness == number(&face.toughness), format!("{:?}", data.toughness), format!("{:?}", face.toughness));
    let indicator: Vec<String> = data.color_indicator.iter()
        .map(|c| match c {
            Color::White => "W",
            Color::Blue => "U",
            Color::Black => "B",
            Color::Red => "R",
            Color::Green => "G",
        }.to_string())
        .collect();
    check("colour indicator", indicator == face.color_indicator, format!("{indicator:?}"), format!("{:?}", face.color_indicator));
    // Reminder text is not rules text (CR 207.2a), so it is set aside on
    // both sides; a basic land's is the mana ability its type gives it
    // (305.6), which the engine prints as text.
    let rules = |t: &str| without_reminders(t).lines().map(str::trim).filter(|l| !l.is_empty()).collect::<Vec<_>>().join("\n");
    let unbracketed = face.oracle_text.replace(['(', ')'], "");
    check("rules text", rules(&data.oracle_text) == rules(&face.oracle_text) || data.oracle_text == unbracketed,
        format!("{:?}", data.oracle_text), format!("{:?}", face.oracle_text));
    let mut keywords = data.keywords.clone();
    keywords.sort_by_key(|k| k.label());
    keywords.dedup();
    let printed = printed_keywords(&face.oracle_text);
    check("keywords", keywords == printed, format!("{keywords:?}"), format!("{printed:?}"));
    if front {
        let flashback = without_reminders(&face.oracle_text).lines()
            .find_map(|l| l.trim().strip_prefix("Flashback ").map(str::to_string));
        let engine = cost_of(data.flashback_cost.as_ref());
        let oracle = flashback.as_deref().and_then(printed_cost).map(|s| counted(&s));
        check("flashback cost", data.flashback_cost.is_some() == flashback.is_some() && oracle.is_none_or(|o| o == engine),
            format!("{:?}", data.flashback_cost), format!("{flashback:?}"));
    }
    out
}

#[test]
fn every_card_prints_what_its_oracle_record_prints() {
    let registry = registry();
    let oracle = oracle();
    let mut failures = Vec::new();
    let mut compared = 0;
    let mut names: Vec<&str> = registry.all_names();
    names.sort_unstable();
    for name in names {
        let id = registry.get_id_by_name(name).expect("a registered name");
        let behavior = registry.get(id).expect("a registered card");
        let Some((front, back)) = oracle.iter().find(|(f, _)| f.name == name) else {
            failures.push(format!("{name}: no Oracle record"));
            continue;
        };
        compared += 1;
        for why in disagreements(&behavior.card_data(), front, true) {
            failures.push(format!("{name}: {why}"));
        }
        match (behavior.back_face_data(), back) {
            (Some(data), Some(face)) => {
                for why in disagreements(&data, face, false) {
                    failures.push(format!("{name}'s back face: {why}"));
                }
            }
            (None, None) => {}
            (data, face) => failures.push(format!("{name}: a back face in the engine {}, in the Oracle {}",
                data.is_some(), face.is_some())),
        }
    }
    assert!(compared >= 268, "only {compared} cards compared");
    assert!(failures.is_empty(), "{} disagreements with the Oracle:\n  {}", failures.len(), failures.join("\n  "));
}
