//! A friend across the internet reaches the whole table through one port.
//!
//! The game pages used to be links to ports of their own (`--game-ports`,
//! 8801-8899), so a host had to forward a range of ports to play with
//! anyone off the LAN. The links now go through the lobby's port, at
//! `/game/<port>/<key>/`, and the pages themselves listen on loopback; the lobby
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

/// `head` sent in one write to `port`, and everything that comes back
/// until the server closes or goes quiet for two seconds.
fn raw_head(port: u16, head: &str) -> Vec<u8> {
    use std::io::{Read, Write};
    let mut s = std::net::TcpStream::connect(("127.0.0.1", port)).expect("the lobby's port");
    s.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
    s.write_all(head.as_bytes()).unwrap();
    let mut out = Vec::new();
    let mut chunk = [0u8; 65_536];
    while let Ok(n) = s.read(&mut chunk) {
        if n == 0 {
            break;
        }
        out.extend_from_slice(&chunk[..n]);
    }
    out
}

#[test]
fn the_game_page_and_its_socket_come_through_the_lobbys_port() {
    if !lobby_support::python3_available() {
        eprintln!("skipping: no python3 to run the stub seat with");
        return;
    }
    let server = Server::start("single-port", "human,ai", &["--pick-seconds", "1", "--build-seconds", "2"]);
    let url = match_url(&server);
    // A path, on whatever host the seat reached the lobby by: the
    // advertised address is not the tunnel's or the tailnet's (#969).
    let prefix = "/game/";
    assert!(url.starts_with(prefix) && url.ends_with('/'), "the link is a path through the lobby: {url}");
    let (game_port, key) = url[prefix.len()..].trim_end_matches('/').split_once('/').expect("/game/<port>/<key>/");
    let game_port: u16 = game_port.parse().expect("a port in the link");
    assert!(key.len() >= 16, "the link carries the page's key: {url}");

    // The page, its script, and its socket, all on the lobby's port.
    let page_path = format!("/game/{game_port}/{key}/");
    let (status, body) = server.http_get(&page_path);
    assert_eq!(status, 200, "{body}");
    assert!(body.contains("dist/main.js"), "the game page: {body}");
    let (status, _) = server.http_get(&format!("{page_path}dist/main.js"));
    assert_eq!(status, 200);
    let (status, _) = server.http_get(&format!("/game/{game_port}"));
    assert_eq!(status, 301, "a bare /game/<port> is sent to the page's own directory");
    let (status, _) = server.http_get(&format!("/game/{game_port}/{key}"));
    assert_eq!(status, 301, "a link without its last slash is sent to the page's own directory");

    // Without the key, or with another, the live page is not there: a
    // stranger who guesses the port reads no hand and answers nothing (#872).
    for stranger in [format!("/game/{game_port}/"), format!("/game/{game_port}/dist/main.js"),
                     format!("/game/{game_port}/{}/", "0".repeat(key.len()))] {
        let (status, body) = server.http_get(&stranger);
        assert_eq!(status, 404, "{stranger} is refused: {body}");
    }
    assert!(WsClient::try_connect(&format!("ws://127.0.0.1:{}/game/{game_port}/ws", server.port)).is_none(),
        "the page's socket without the key is refused");

    let mut page = WsClient::connect(&format!("ws://127.0.0.1:{}{page_path}ws", server.port));
    page.send(serde_json::json!({"type": "hello"}));
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        if page.next_message(Duration::from_millis(500)).is_some() {
            break;
        }
        assert!(Instant::now() < deadline, "the game page's socket said nothing through the lobby");
    }

    // A browser holding tens of KB of cookies for the host (#935, #936).
    // The heads were read to 16 KiB: an `Upgrade` after a long Cookie was
    // not seen and the socket was answered 404, and the unread rest of a
    // page fetch's head reset the connection, cutting render.js (81 KB)
    // at 64 KiB through the proxy. Past 64 KiB a head is refused 431.
    let lobby = server.port;
    let lobby_ws = format!("/ws?seat=0&key={}", server.key(0));
    for (path, cookie) in [(format!("{page_path}ws"), 16_500), (format!("{page_path}ws"), 40_000), (lobby_ws.clone(), 40_000)] {
        let reply = raw_head(lobby, &format!("GET {path} HTTP/1.1\r\nHost: x\r\nSec-WebSocket-Version: 13\r\n\
            Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nConnection: keep-alive, Upgrade\r\n\
            Cookie: a={}\r\nUpgrade: websocket\r\n\r\n", "v".repeat(cookie)));
        assert!(reply.starts_with(b"HTTP/1.1 101"), "{path} with a {cookie}-byte cookie: {}",
            String::from_utf8_lossy(&reply[..reply.len().min(200)]));
    }
    let script = std::fs::read(lobby_support::repo_root().join("mtg-gui/dist/render.js")).expect("the page's script");
    assert!(script.len() > 65_536, "a script longer than one 64 KiB read");
    for _ in 0..5 {
        let reply = raw_head(lobby, &format!("GET {page_path}dist/render.js HTTP/1.1\r\nHost: x\r\n\
            Cookie: a={}\r\nConnection: close\r\n\r\n", "v".repeat(40_000)));
        let at = reply.windows(4).position(|w| w == b"\r\n\r\n").expect("a response head") + 4;
        assert!(reply.starts_with(b"HTTP/1.1 200"), "{}", String::from_utf8_lossy(&reply[..at]));
        assert_eq!(reply.len() - at, script.len(), "the whole script comes through the proxy");
    }
    // A head of bare line feeds is a WebSocket through the proxy too: its
    // request line was looked for by CRLF, and the game page was sent the
    // line alone and answered 408 five seconds later.
    let reply = raw_head(lobby, &format!("GET {page_path}ws HTTP/1.1\nHost: x\nSec-WebSocket-Version: 13\n\
        Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\nConnection: Upgrade\nUpgrade: websocket\n\n"));
    assert!(reply.starts_with(b"HTTP/1.1 101"), "an LF head through the proxy: {}", String::from_utf8_lossy(&reply[..reply.len().min(200)]));
    let reply = raw_head(lobby, &format!("GET {page_path} HTTP/1.1\r\nHost: x\r\nCookie: a={}\r\n\r\n", "v".repeat(70_000)));
    assert!(reply.starts_with(b"HTTP/1.1 431"), "{}", String::from_utf8_lossy(&reply[..reply.len().min(200)]));

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
    let (port, key) = url.trim_start_matches("http://127.0.0.1:").trim_end_matches('/').split_once('/').expect("<port>/<key>/");
    let port: u16 = port.parse().unwrap();
    assert_ne!(port, server.port, "a port of its own");
    // On its own port too, the page answers only under its key (#872).
    assert!(key.len() >= 16, "{url}");
    let get = |path: &str| {
        use std::io::{Read, Write};
        let mut s = std::net::TcpStream::connect(("127.0.0.1", port)).expect("the page's port");
        write!(s, "GET {path} HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n").unwrap();
        let mut out = String::new();
        let _ = s.read_to_string(&mut out);
        out
    };
    assert!(get(&format!("/{key}/")).starts_with("HTTP/1.1 200"), "the keyed page is served");
    assert!(get("/").starts_with("HTTP/1.1 404"), "the bare port serves nothing");
    assert!(WsClient::try_connect(&format!("ws://127.0.0.1:{port}/ws")).is_none(), "nor its socket");
}
