//! Hold a key (any keyboard, e.g. Wooting) -> Logitech G HUB G-Shift on while held.
//!
//! Uses G HUB agent's local websocket API (ws://127.0.0.1:9010), the same one the
//! G HUB UI uses: SET /input/gshift {on, source: HOST}.
#![windows_subsystem = "windows"] // no console window

use std::io::ErrorKind;
use std::thread::sleep;
use std::time::Duration;
use tungstenite::client::IntoClientRequest;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{Error, Message};

const VK: i32 = 0x7C; // F13: map the Wooting key to F13 in Wootility (change VK to use another key)

#[link(name = "user32")]
unsafe extern "system" {
    fn GetAsyncKeyState(vk: i32) -> i16;
}

fn msg(on: bool) -> String {
    format!(
        r#"{{"msgId":"","verb":"SET","path":"/input/gshift","payload":{{"@type":"type.googleapis.com/logi.protocol.devices.GShift","on":{on},"source":"HOST"}}}}"#
    )
}

fn run() -> Result<(), Error> {
    let mut req = "ws://127.0.0.1:9010".into_client_request()?;
    req.headers_mut().insert("Sec-WebSocket-Protocol", "json".parse().unwrap());
    req.headers_mut().insert("Origin", "file://".parse().unwrap());
    let (mut ws, _) = tungstenite::connect(req)?;
    if let MaybeTlsStream::Plain(s) = ws.get_mut() {
        s.set_read_timeout(Some(Duration::from_millis(1)))?;
    }
    let mut last = false;
    loop {
        let now = unsafe { GetAsyncKeyState(VK) } as u16 & 0x8000 != 0;
        if now != last {
            ws.send(Message::text(msg(now)))?;
            last = now;
        }
        // agent pushes broadcasts; read them so the buffer never fills
        match ws.read() {
            Ok(_) => {}
            Err(Error::Io(e)) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {}
            Err(e) => return Err(e),
        }
        sleep(Duration::from_millis(4)); // shortcut: polling, ~5 ms worst-case latency; use a LL hook if that's ever noticeable
    }
}

fn main() {
    loop {
        let _ = run(); // G HUB restarts/updates drop the socket: reconnect
        sleep(Duration::from_secs(2));
    }
}
