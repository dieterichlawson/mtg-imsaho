//! A friend across the internet reaches the whole table through one port.
//!
//! The game pages used to be links to ports of their own (`--game-ports`,
//! 8801-8899), so a host had to forward a range of ports to play with
//! anyone off the LAN. The links now go through the lobby's port, at
//! `/game/<port>/`, and the pages themselves listen on loopback; the lobby
//! passes a page's fetches and its socket through, and reaches nothing on
//! the machine but this table's live game pages.
#![cfg(unix)]

mod lobby_support;

use std::time::{Duration, Instant};

use lobby_support::{Server, WsClient};

/// The human seat's match link, once its match has started.
fn match_url(server: &Server) -> String {
    let mut c0 = server.client(0);
    let deadline = Instant::now() + Duration::from_secs(240);
    loop {
        if let Some(v) = c0.next_view(Duration::from_millis(200)) {
            if let Some(url) = v["matches"].as_array().and_then(|m| m.iter().find_map(|m| m["url"].as_str())) {
                return url.to_string();
            }
        }
        assert!(Instant::now() < deadline, "the match never started:\n{}", server.stderr());
    }
}

#[test]
fn the_game_page_and_its_socket_come_through_the_lobbys_port() {
    if !lobby_support::python3_available() {
        eprintln!("skipping: no python3 to run the stub seat with");
        return;
    }
    let server = Server::start("single-port", "human,ai", &["--pick-seconds", "1", "--build-seconds", "2"]);
    let url = match_url(&server);
    let prefix = format!("http://127.0.0.1:{}/game/", server.port);
    assert!(url.starts_with(&prefix) && url.ends_with('/'), "the link is through the lobby: {url}");
    let game_port: u16 = url[prefix.len()..].trim_end_matches('/').parse().expect("a port in the link");

    // The page, its script, and its socket, all on the lobby's port.
    let (status, body) = server.http_get(&format!("/game/{game_port}/"));
    assert_eq!(status, 200, "{body}");
    assert!(body.contains("dist/main.js"), "the game page: {body}");
    let (status, _) = server.http_get(&format!("/game/{game_port}/dist/main.js"));
    assert_eq!(status, 200);
    let (status, _) = server.http_get(&format!("/game/{game_port}"));
    assert_eq!(status, 301, "a bare /game/<port> is sent to the page's own directory");
    let mut page = WsClient::connect(&format!("ws://127.0.0.1:{}/game/{game_port}/ws", server.port));
    page.send(serde_json::json!({"type": "hello"}));
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        if page.next_message(Duration::from_millis(500)).is_some() {
            break;
        }
        assert!(Instant::now() < deadline, "the game page's socket said nothing through the lobby");
    }

    // Nothing else on the machine: not the lobby itself, not a port no
    // page of this table is on.
    for port in [server.port, 22, game_port.wrapping_add(1)] {
        let (status, body) = server.http_get(&format!("/game/{port}/"));
        assert_eq!(status, 404, "/game/{port}/ is no game page: {body}");
    }
}

#[test]
fn direct_game_ports_link_each_page_on_its_own_port() {
    if !lobby_support::python3_available() {
        eprintln!("skipping: no python3 to run the stub seat with");
        return;
    }
    let server = Server::start("direct-ports", "human,ai",
        &["--pick-seconds", "1", "--build-seconds", "2", "--direct-game-ports"]);
    let url = match_url(&server);
    assert!(!url.contains("/game/") && url.starts_with("http://127.0.0.1:"), "{url}");
    let port: u16 = url.trim_start_matches("http://127.0.0.1:").trim_end_matches('/').parse().unwrap();
    assert_ne!(port, server.port, "a port of its own");
}
