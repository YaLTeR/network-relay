use std::cell::RefCell;
use std::collections::HashMap;
use std::net::SocketAddr;

use futures::unsync::mpsc::UnboundedSender;
use futures::unsync::oneshot;
use native_tls::TlsAcceptor;

use config::Config;

pub struct SharedData {
    pub config: Config,

    // A HashMap to store the connected listeners.
    pub listen_connections: RefCell<HashMap<SocketAddr, UnboundedSender<String>>>,

    // The password for the next control connection.
    pub control_password: RefCell<String>,

    // Oneshot sender for terminating the current control connection.
    pub current_control_tx: RefCell<Option<oneshot::Sender<()>>>,

    // A TLS acceptor.
    pub tls_acceptor: Option<TlsAcceptor>,
}
