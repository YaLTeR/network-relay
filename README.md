This is a network relay that runs on a server. It opens two ports, one for the "controller" (the person sending messages) and one for the "listeners". Every message from the controller is relayed to all connected listeners. The controller port accepts normal connections, while the listener port accepts secure websocket connections.

There's basic authorization. Listeners need to supply a pre-configured password as their first message. The first message a listener receives is the (randomly-generated) current controller password. The controller needs to send this password as their first message. As soon as the controller is authorized, the previously connected controller, if any, is disconnected, and a new controller password is generated and sent to listeners.

The controller password is sent in a `control_password <password>` format. The password consists of randomly generated 0-9a-zA-Z characters. If the controller tries sending a message starting with `control_password ` it will be ignored.

Listeners can send messages as well, and they are relayed to other listeners. To distinguish listener messages from controller messages, listeners can start their messages with `host `. If the controller tries sending a message starting with `host ` it will be ignored.

The server requires simple configuration, in the `config.yaml` file. See the example `config.yaml`, it's self-explanatory.

Made with Rust and Tokio.

### Running
1. Get nightly Rust. The easiest way is to use [rustup](https://rustup.rs/).
2. Clone the repository.
3. If you are using rustup, `rustup override set nightly`.
4. `cargo run --release`

## License

Licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
