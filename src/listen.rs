use std::io;
use std::net::SocketAddr;
use std::rc::Rc;

use error_chain::ChainedError;
use futures::{Future, IntoFuture, Sink, Stream};
use futures::unsync::mpsc::unbounded;
use tokio_core::net::TcpStream;
use tokio_core::reactor::Handle;
use tokio_io::AsyncRead;

use errors::*;
use line_codec::LineCodec;
use message::Message;
use shared_data::SharedData;

fn authorize<S>(data: &SharedData,
                first_line: Option<String>,
                reader: S)
                -> impl IntoFuture<Item = S, Error = Error>
where
    S: Stream<Item = String, Error = Error>,
{
    if let Some(first_line) = first_line {
        // Check the password.
        ensure!(first_line == data.config.listen_password, "wrong password");

        return Ok(reader);
    }

    bail!("connection closed");
}

fn handle_listener<T, U>(data: &SharedData,
                         addr: SocketAddr,
                         reader: T,
                         writer: U)
                         -> impl IntoFuture<Item = (), Error = Error>
where
    T: Stream<Item = String, Error = Error>,
    U: Sink<SinkItem = String, SinkError = io::Error>,
{
    let (tx, rx) = unbounded();

    // Send the current control password.
    Message::ControlPassword(&data.control_password.borrow()).send(&tx);

    data.listen_connections.borrow_mut().insert(addr, tx);

    // The closure return type is needed for the type checker.
    let rx = rx.map_err(|_| -> io::Error {
                            unreachable!();
                        });

    let handle_writer = writer.send_all(rx)
                              .map(|_| ())
                              .map_err(|err| Error::with_chain(err, "I/O error writing data"));

    // Additionally terminate when the connection is closed from the listener.
    reader.for_each(|_| Ok(()))
          .map_err(|err| Error::with_chain(err, "I/O error reading data"))
          .select(handle_writer)
          .map_err(|(err, _)| err)
          .map(|_| ())
}

pub fn serve(handle: &Handle, data: &Rc<SharedData>, tcp: TcpStream, addr: SocketAddr) {
    println!("Incoming listen connection from {}", addr);

    let (writer, reader) = tcp.framed(LineCodec).split();

    let handle_auth = {
        let data = data.clone();

        reader.map_err(|err| Error::with_chain(err, "I/O error reading data"))
              .into_future()
              .map_err(|(err, _)| err)
              .and_then(move |(first_line, reader)| authorize(&data, first_line, reader))
    };

    let handle_writer = {
        let data = data.clone();

        handle_auth.and_then(move |reader| handle_listener(&data, addr, reader, writer))
    };

    let handle_conn = {
        let data = data.clone();

        handle_writer.then(move |x| {
            if let Err(err) = x {
                println!("Error on listen connection from {}: {}",
                         addr,
                         err.display());
            }

            data.listen_connections.borrow_mut().remove(&addr);
            Ok(())
        })
    };

    handle.spawn(handle_conn);
}
