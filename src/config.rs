#[derive(Debug, Deserialize)]
pub struct Config {
    pub listen_password: String,
    pub listen_port: u16,
    pub control_port: u16,

    pub listen_tls: bool,
    pub identity_file: Option<String>,
    pub identity_password: Option<String>,
}
