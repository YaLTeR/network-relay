use futures::unsync::mpsc::UnboundedSender;

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

pub fn is_special_message(string: &str) -> bool {
    string.starts_with(CONTROL_PASSWORD)
}
