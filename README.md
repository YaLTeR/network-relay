This is a network relay that runs on a server. It opens two ports, one for the "controller" (the person sending messages) and one for the "listeners". Every message from the controller is relayed to all connected listeners.

There's basic authorization. Listeners need to supply a pre-configured password as their first (and only) message. The first message a listener receives is the (randomly-generated) current controller password. The controller needs to send this password as their first message. As soon as the controller is authorized, the previously connected controller, if any, is disconnected, and a new controller password is generated and sent to listeners.

The server requires simple configuration, in the `config.yaml` file. See the example `config.yaml`, it's self-explanatory.

Made with Rust and Tokio.

### Running
1. Get nightly Rust.
2. Clone the repository.
3. `cargo run --release`

## License

Licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
