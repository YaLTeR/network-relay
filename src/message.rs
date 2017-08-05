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

// Forward the line to all listeners.
pub fn forward_to_listeners(data: &SharedData, line: String) {
    // Don't forward special messages to prevent abuse.
    if is_special_message(&line) {
        return;
    }

    for tx in data.listen_connections.borrow_mut().values() {
        tx.send(line.clone()).unwrap();
    }
}
