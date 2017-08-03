#[derive(Debug, Deserialize)]
pub struct Config {
    pub listen_password: String,
    pub listen_port: u16,
    pub control_port: u16,
}
