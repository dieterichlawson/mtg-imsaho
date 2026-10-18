//! A browser seat lets its port go when it is dropped.

use std::net::{TcpListener, TcpStream};

/// #853: the accept loop ran for the life of the process, so a hosted
/// table's finished match kept its game port and a later round found
/// "no free port" and forfeited the people's match.
#[test]
fn a_dropped_gui_seat_releases_its_port() {
    std::env::set_var(mtg_player::gui::WEB_DIR_ENV,
        concat!(env!("CARGO_MANIFEST_DIR"), "/../mtg-gui"));
    let seat = mtg_player::gui::GuiPlayer::new("gui", Some(0)).expect("the seat binds a port");
    let port: u16 = seat.url.trim_end_matches('/').rsplit(':').next()
        .and_then(|p| p.parse().ok()).expect("the url names its port");
    assert!(TcpListener::bind(("127.0.0.1", port)).is_err(), "the seat holds its port while it lives");
    // A page still attached when the match ends does not keep it either.
    let _page = TcpStream::connect(("127.0.0.1", port)).expect("the page connects");
    drop(seat);
    TcpListener::bind(("127.0.0.1", port))
        .unwrap_or_else(|e| panic!("port {port} is still held after the seat was dropped: {e}"));
}
