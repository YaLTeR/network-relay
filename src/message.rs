use std::net::SocketAddr;

use futures::unsync::mpsc::UnboundedSender;

use shared_data::SharedData;

#[derive(Debug)]
pub enum Message<'a> {
    ControlPassword(&'a str),
}

const CONTROL_PASSWORD: &str = "control_password ";

impl<'a> Message<'a> {
    pub fn send(self, tx: &UnboundedSender<String>) {
        tx.send(match self {
                    Message::ControlPassword(password) => {
                        format!("{}{}", CONTROL_PASSWORD, password)
                    }
                })
          .unwrap();
    }
}

fn is_special_message(string: &str) -> bool {
    string.starts_with(CONTROL_PASSWORD)
}

pub fn is_listener_only_message(string: &str) -> bool {
    string.starts_with("host ")
}

// Forward the line to all listeners.
pub fn forward_to_listeners(data: &SharedData, line: String) {
    // Don't forward special messages to prevent abuse.
    if is_special_message(&line) {
        return;
    }

    for tx in data.listen_connections.borrow().values() {
        tx.send(line.clone()).unwrap();
    }
}

// Forward the line to all listeners, except the one with the passed address.
pub fn forward_to_listeners_except(data: &SharedData, addr: SocketAddr, line: String) {
    // Don't forward special messages to prevent abuse.
    if is_special_message(&line) {
        return;
    }

    for (_, tx) in data.listen_connections
                       .borrow()
                       .iter()
                       .filter(|&(&k, _)| k != addr)
    {
        tx.send(line.clone()).unwrap();
    }
}
