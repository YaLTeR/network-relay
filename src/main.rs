extern crate bytes;
#[macro_use]
extern crate error_chain;
extern crate futures;
extern crate native_tls;
extern crate rand;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_yaml;
extern crate tokio_core;
extern crate tokio_io;
extern crate tokio_tls;
extern crate websocket;

mod errors {
    error_chain!{}
}

mod config;
mod control;
mod line_codec;
mod listen;
mod message;
mod shared_data;

use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::rc::Rc;

use futures::{Future, Stream};
use native_tls::{Pkcs12, TlsAcceptor};
use rand::Rng;
use tokio_core::net::TcpListener;
use tokio_core::reactor::{Core, Handle};

use errors::*;
use config::Config;
use shared_data::SharedData;

const CONFIG_FILENAME: &str = "config.yaml";

fn generate_password() -> String {
    let rv = rand::thread_rng().gen_ascii_chars().take(16).collect();
    println!("New control password: {}", rv);
    rv
}

fn read_config() -> Result<Config> {
    let file = File::open(CONFIG_FILENAME)
        .chain_err(|| format!("could not open {}", CONFIG_FILENAME))?;
    serde_yaml::from_reader(file).chain_err(|| format!("could not parse {}", CONFIG_FILENAME))
}

fn bind_socket(handle: &Handle, port: u16) -> Result<TcpListener> {
    TcpListener::bind(&SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), port),
                      handle)
    .chain_err(|| format!("could not bind a socket to port {}", port))
}

fn set_up_tls_acceptor(config: &Config) -> Result<TlsAcceptor> {
    let identity_file = config.identity_file
                              .as_ref()
                              .ok_or("identity_file is abscent from the config")?;
    let identity_password = config.identity_password
                                  .as_ref()
                                  .ok_or("identity_password is abscent from the config")?;

    let mut file = File::open(identity_file)
        .chain_err(|| format!("could not open {}", identity_file))?;
    let mut pkcs12 = Vec::new();
    file.read_to_end(&mut pkcs12)
        .chain_err(|| format!("could not read {}", identity_file))?;

    let pkcs12 = Pkcs12::from_der(&pkcs12, identity_password)
        .chain_err(|| format!("could not parse {}", identity_file))?;
    let builder = TlsAcceptor::builder(pkcs12)
        .chain_err(|| "could not make a TlsAcceptorBuilder")?;
    let acceptor = builder.build().chain_err(|| "could not make a TlsAcceptor");

    acceptor
}

fn run() -> Result<()> {
    let config = read_config().chain_err(|| "error reading the config")?;

    let mut core = Core::new().chain_err(|| "could not create the event loop")?;
    let handle = core.handle();

    let tls_acceptor = if config.listen_tls {
        Some(set_up_tls_acceptor(&config)
                 .chain_err(|| "could not set up a TLS acceptor")?)
    } else {
        None
    };

    let control_tcp = bind_socket(&handle, config.control_port)
        .chain_err(|| "could not bind the control socket")?;
    let listen_tcp = bind_socket(&handle, config.listen_port)
        .chain_err(|| "could not bind the listen socket")?;

    let shared_data = Rc::new(SharedData {
                                  config,
                                  listen_connections: RefCell::new(HashMap::new()),
                                  control_password: RefCell::new(generate_password()),
                                  current_control_tx: RefCell::new(None),
                                  tls_acceptor,
                              });

    let control_server =
        control_tcp.incoming()
                   .map_err(|err| Error::with_chain(err, "I/O error handling the control socket"))
                   .for_each(|(tcp, addr)| {
                                 control::serve(&handle, &shared_data, tcp, addr);
                                 Ok(())
                             });

    let listen_server =
        listen_tcp.incoming()
                  .map_err(|err| Error::with_chain(err, "I/O error handling the listen socket"))
                  .for_each(|(tcp, addr)| {
                                listen::serve(&handle, &shared_data, tcp, addr);
                                Ok(())
                            });

    core.run(control_server.join(listen_server))
        .chain_err(|| "error running the event loop")?;

    Ok(())
}

quick_main!(run);
