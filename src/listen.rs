use std::net::SocketAddr;
use std::rc::Rc;

use error_chain::ChainedError;
use futures::{Async, Future, IntoFuture, Poll, Sink, Stream};
use futures::unsync::mpsc::unbounded;
use tokio_core::net::TcpStream;
use tokio_core::reactor::Handle;
use tokio_tls::{TlsAcceptorExt, TlsStream};
use websocket::async::server::{IntoWs, Upgrade};
use websocket::async::Client;
use websocket::{OwnedMessage, WebSocketError};

use errors::*;
use message::*;
use shared_data::SharedData;

struct AuthorizeStream<S, E>
where
    S: Stream<Item = OwnedMessage, Error = E>,
{
    inner: S,
    data: Rc<SharedData>,
    addr: SocketAddr,
    authorized: bool,
}

impl<S, E> AuthorizeStream<S, E>
where
    S: Stream<Item = OwnedMessage, Error = E>,
{
    fn new(data: Rc<SharedData>, addr: SocketAddr, stream: S) -> Self {
        Self {
            inner: stream,
            data,
            addr,
            authorized: false,
        }
    }
}

impl<S, E> Stream for AuthorizeStream<S, E>
where
    S: Stream<Item = OwnedMessage, Error = E>,
{
    type Item = OwnedMessage;
    type Error = E;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        if self.authorized {
            return self.inner.poll();
        }

        match self.inner.poll() {
            Ok(Async::Ready(Some(OwnedMessage::Text(message)))) => {
                if message == self.data.config.listen_password {
                    self.authorized = true;

                    // Kind of a hack.
                    let listen_connections = self.data.listen_connections.borrow();
                    let tx = listen_connections.get(&self.addr).unwrap();
                    Message::ControlPassword(&self.data.control_password.borrow()).send(tx);

                    self.inner.poll()
                } else {
                    println!("Listen connection from {}: wrong password.", self.addr);
                    Ok(Async::Ready(Some(OwnedMessage::Close(None))))
                }
            }

            other => other,
        }
    }
}

fn handle_client(data: &Rc<SharedData>,
                 addr: SocketAddr,
                 client: Client<TlsStream<TcpStream>>)
                 -> impl IntoFuture<Item = (), Error = WebSocketError> {
    let (writer, reader) = client.split();

    let (tx, rx) = unbounded::<String>();
    data.listen_connections.borrow_mut().insert(addr, tx);

    let rx_messages = rx.map(|s| OwnedMessage::Text(s))
                        .map_err(|_| -> WebSocketError {
                                     unreachable!();
                                 });

    let authorized = AuthorizeStream::new(data.clone(), addr, reader);

    let reader = {
        let data = data.clone();
        authorized.filter_map(move |m| {
            println!("Listener {} sent: {:?}", addr, m);
            match m {
                OwnedMessage::Ping(p) => Some(OwnedMessage::Pong(p)),
                OwnedMessage::Close(c) => Some(OwnedMessage::Close(c)),
                OwnedMessage::Text(t) => {
                    forward_to_listeners(&data, t);
                    None
                }
                _ => None,
            }
        })
    };

    let merged = rx_messages.select(reader);

    let handle_writer = merged.take_while(|m| Ok(!m.is_close()))
                              .forward(writer)
                              .and_then(|(_, writer)| writer.send(OwnedMessage::Close(None)));

    let data = data.clone();
    handle_writer.then(move |x| {
                           data.listen_connections.borrow_mut().remove(&addr);

                           x.map(|_| ())
                       })
}

fn handle_websocket(data: &Rc<SharedData>,
                    upgrade: Upgrade<TlsStream<TcpStream>>,
                    addr: SocketAddr)
                    -> Box<Future<Item = (), Error = Error>> {
    if !upgrade.protocols().iter().any(|x| x == "rust-websocket") {
        let handle_conn =
            upgrade.reject()
                   .map_err(|err| {
                                Error::with_chain(err, "I/O error rejecting an invalid connection")
                            })
                   .map(move |_| println!("Rejected an invalid listen connection from {}", addr));

        return Box::new(handle_conn);
    }

    let data = data.clone();
    let handle_conn = upgrade.use_protocol("rust-websocket")
                             .accept()
                             .and_then(move |(client, _)| handle_client(&data, addr, client));

    let handle_conn = handle_conn.map(|_| ()).map_err(|err| {
        Error::with_chain(err, "Websocket error handling connection")
    });

    Box::new(handle_conn)
}

pub fn serve(handle: &Handle, data: &Rc<SharedData>, tcp: TcpStream, addr: SocketAddr) {
    println!("Incoming listen connection from {}", addr);

    let convert_into_tls = data.tls_acceptor
                               .accept_async(tcp)
                               .map_err(|err| Error::with_chain(err, "Error accepting TLS"));

    let convert_into_ws = convert_into_tls.and_then(|stream| {
        stream.into_ws()
              .map_err(|(_, _, _, err)| Error::with_chain(err, "Invalid websocket connection"))
    });

    let data = data.clone();
    let connection =
        convert_into_ws.and_then(move |upgrade| handle_websocket(&data, upgrade, addr))
                       .map_err(move |err| {
                                    println!("Error on listen connection {}: {}",
                                             addr,
                                             err.display())
                                });

    handle.spawn(connection);
}
