use std::net::SocketAddr;
use std::ops::Deref;
use std::rc::Rc;

use error_chain::ChainedError;
use futures::{Future, IntoFuture, Sink, Stream};
use futures::unsync::oneshot;
use tokio_core::net::TcpStream;
use tokio_core::reactor::Handle;
use tokio_io::AsyncRead;

use errors::*;
use line_codec::LineCodec;
use message::*;
use shared_data::SharedData;
use super::generate_password;

// There may be only one control connection at any given time. As soon as a new control
// connection is established and authorized with the correct password, the old one is
// forgotten (and a new password is generated).
fn authorize<S>(data: &SharedData,
                first_line: Option<String>,
                reader: S)
                -> impl IntoFuture<Item = (S, oneshot::Receiver<()>), Error = Error>
where
    S: Stream<Item = String, Error = Error>,
{
    if let Some(first_line) = first_line {
        // Check the password.
        ensure!(&first_line == data.control_password.borrow().deref(),
                "wrong password");

        // Change the control password.
        *data.control_password.borrow_mut() = generate_password();

        // Forward the new password to the listeners.
        for tx in data.listen_connections.borrow_mut().values() {
            Message::ControlPassword(&data.control_password.borrow()).send(tx);
        }

        // Disconnect the previous connection.
        let mut current_control_tx = data.current_control_tx.borrow_mut();
        if let Some(tx) = current_control_tx.take() {
            tx.send(()).unwrap();
        }

        // Store the new connection.
        let (tx, rx) = oneshot::channel();
        *current_control_tx = Some(tx);

        return Ok((reader, rx));
    }

    bail!("connection closed");
}

// Shut down the current control oneshot cleanly.
fn shutdown_control_oneshot(data: &SharedData) {
    data.current_control_tx
        .borrow_mut()
        .take()
        .unwrap()
        .send(())
        .unwrap();
}

pub fn serve(handle: &Handle, data: &Rc<SharedData>, tcp: TcpStream, addr: SocketAddr) {
    println!("Incoming control connection from {}", addr);

    let (writer, reader) = tcp.framed(LineCodec).split();

    let handle_auth = {
        let data = data.clone();

        reader.map_err(|err| Error::with_chain(err, "I/O error reading data"))
              .into_future()
              .map_err(|(err, _)| err)
              .and_then(move |(first_line, reader)| authorize(&data, first_line, reader))
    };

    let handle_conn = {
        let data = data.clone();
        let data_ = data.clone();

        handle_auth.and_then(move |(reader, rx)| {
            // Notify the controller they've been authorized.
            let handle_writer = writer.send("authorized".to_string()).map_err(
                |err| Error::with_chain(err, "I/O error writing data"),
            );

            let handle_reader = reader.for_each(move |line| {
                println!("Controller {} sent: {}", addr, line);

                if !is_listener_only_message(&line) {
                    forward_to_listeners(&data, line);
                }

                Ok(())
            })
                                      .then(move |result| {
                                                shutdown_control_oneshot(&data_);
                                                result
                                            });

            let handle = handle_writer.and_then(|_| handle_reader);

            // Exit if either the connection was closed, or we received a oneshot message
            // indicating there's a new connection and this one should terminate.
            rx.map_err(|_| unreachable!())
              .select(handle)
              .map(|_| ())
              .map_err(|(err, _)| err)
        })
    };

    let handle_conn = handle_conn.map_err(move |err| {
                                              println!("Error on control connection from {}: {}",
                                                       addr,
                                                       err.display());
                                          });

    handle.spawn(handle_conn);
}
