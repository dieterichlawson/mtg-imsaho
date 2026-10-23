//! The table's snapshot, written by every thread at once (#917).
//!
//! Every draft seat's thread and every match thread writes the snapshot
//! after its change, and they shared one `.tmp`: a write that found it
//! renamed away by another said "the snapshot cannot be written" for a
//! write that had not failed, and a thread that took its snapshot first
//! could rename last, leaving the file older than the table.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use mtg_draft::pack::BoosterPack;
use mtg_draft::set_data::SetData;
use mtg_draft_runner::card_lines::CardLines;
use mtg_draft_runner::draft_log::DraftLogger;
use mtg_draft_runner::lobby::{Lobby, LobbyConfig, SeatKind};
use mtg_draft_runner::server::{ServerConfig, Shared};
use mtg_engine::cards::CardRegistry;

fn repo_root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
}

fn table(save: PathBuf) -> Arc<Shared> {
    let mut set_data = SetData::load(&repo_root().join("data/sets/isd.json")).unwrap();
    let registry = Arc::new(CardRegistry::with_all_cards());
    set_data.filter_implemented(&registry);
    let mut names = set_data.all_card_names();
    names.sort();
    let mut it = names.into_iter().cycle();
    let packs: Vec<Vec<BoosterPack>> = (0..2).map(|_| (0..3).map(|_| {
        let cards: Vec<String> = it.by_ref().take(14).collect();
        BoosterPack { commons: cards[..12].to_vec(), uncommons: vec![], rare: cards[12].clone(), dfc: cards[13].clone(), foil: None }
    }).collect()).collect();
    let lines = CardLines::new(&set_data.all_card_names(), &set_data.rarities(), &registry);
    let config = LobbyConfig {
        set_code: "isd".into(), set_name: set_data.set_name.clone(),
        seats: vec![SeatKind::Human, SeatKind::Human], best_of: 3, seed: 1, guide_path: None,
        pick_seconds: None, build_seconds: None, sideboard_seconds: None, out_dir: None,
    };
    let lobby = Lobby::new(config, &packs, &set_data, Arc::clone(&registry), &lines, DraftLogger::silent()).unwrap();
    let server = ServerConfig {
        bind: "127.0.0.1".into(), port: 0, advertise: "127.0.0.1".into(), game_ports: (0, 0),
        web_dir: repo_root().join("mtg-gui"), quiet: true, out_dir: None, guide: None,
        save: Some(save), direct_game_ports: false,
    };
    Shared::new(lobby, server, registry, lines, String::new(), set_data.set_name.clone())
}

#[test]
fn threads_writing_the_snapshot_at_once_neither_fail_nor_leave_an_older_one() {
    let dir = std::env::temp_dir().join(format!("mtg-snapshot-race-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let save = dir.join("table.save.json");
    let shared = table(save.clone());

    let writers: Vec<_> = (0..8).map(|seat| {
        let shared = Arc::clone(&shared);
        std::thread::spawn(move || {
            (0..40).filter_map(|n| {
                // A change, then its write, as a seat's thread makes them.
                shared.lobby().set_name(seat % 2, &format!("seat{seat}-{n}"));
                shared.write_snapshot().err()
            }).collect::<Vec<String>>()
        })
    }).collect();
    let failures: Vec<String> = writers.into_iter().flat_map(|w| w.join().unwrap()).collect();
    assert!(failures.is_empty(), "no write failed: {failures:?}");

    // The last write took the table as it is now.
    let on_disk: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&save).unwrap()).unwrap();
    let now = serde_json::to_value(shared.lobby().snapshot()).unwrap();
    assert_eq!(on_disk, now, "the file is the table as it stands");
    let _ = std::fs::remove_dir_all(&dir);
}
