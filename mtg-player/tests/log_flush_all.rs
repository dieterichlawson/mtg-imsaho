//! On a fatal, every worker's held records reach the log, in their order
//! (issue #658). Its own binary because the log is process-global.

#[test]
fn a_fatal_flush_writes_every_workers_block_in_rank_order() {
    let path = std::env::temp_dir().join(format!("mtg-flush-all-{}.log", std::process::id()));
    let _ = std::fs::remove_file(&path);
    mtg_player::game_log::init(path.to_str().unwrap()).unwrap();

    // Two workers hold their records — an Error one included — and never
    // get to hand them back, as when another worker's fatal ends the run.
    let (ready_tx, ready_rx) = std::sync::mpsc::channel();
    let (go_tx, go_rx) = std::sync::mpsc::channel::<()>();
    let go_rx = std::sync::Arc::new(std::sync::Mutex::new(go_rx));
    let mut workers = Vec::new();
    for rank in [1u64, 0] {
        let ready_tx = ready_tx.clone();
        let go_rx = std::sync::Arc::clone(&go_rx);
        workers.push(std::thread::spawn(move || {
            mtg_player::game_log::buffer_ranked(rank);
            mtg_player::game_log::write(file!(), line!(), "PROMPT", &format!("worker {rank}"));
            mtg_player::game_log::write_at(mtg_player::game_log::LogLevel::Error, file!(), line!(),
                "MALFORMED", &format!("worker {rank} answered badly"));
            ready_tx.send(()).unwrap();
            let _ = go_rx.lock().unwrap().recv();
        }));
    }
    ready_rx.recv().unwrap();
    ready_rx.recv().unwrap();
    let before = std::fs::read_to_string(&path).unwrap_or_default();
    assert!(!before.contains("answered badly"), "an Error record is held with its scope, not written through");

    mtg_player::game_log::flush_all();
    let after = std::fs::read_to_string(&path).unwrap();
    let pos = |needle: &str| after.find(needle).unwrap_or_else(|| panic!("{needle} missing:\n{after}"));
    assert!(pos("worker 0") < pos("worker 0 answered badly"));
    assert!(pos("worker 0 answered badly") < pos("worker 1"), "rank 0's block first, whole:\n{after}");
    drop(go_tx);
    for w in workers {
        w.join().unwrap();
    }
    let _ = std::fs::remove_file(&path);
}
