use std::{
    collections::HashSet,
    fmt, fs,
    net::IpAddr,
    path::{Path, PathBuf},
};

use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Relay,
    Agent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    BToA,
    AToB,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransportMode {
    Tcp,
    TlsTcp,
    Quic,
}

impl Default for TransportMode {
    fn default() -> Self {
        Self::Tcp
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct TransportConfig {
    #[serde(default)]
    pub mode: TransportMode,
    #[serde(default)]
    pub fallback: Vec<TransportMode>,
    #[serde(default = "default_connect_timeout_secs")]
    pub connect_timeout_secs: u64,
    #[serde(default = "default_idle_timeout_secs")]
    pub idle_timeout_secs: u64,
    #[serde(default = "default_max_frame_bytes")]
    pub max_frame_bytes: usize,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            mode: TransportMode::Tcp,
            fallback: Vec::new(),
            connect_timeout_secs: default_connect_timeout_secs(),
            idle_timeout_secs: default_idle_timeout_secs(),
            max_frame_bytes: default_max_frame_bytes(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecurityMode {
    Token,
    Mtls,
}

impl Default for SecurityMode {
    fn default() -> Self {
        Self::Token
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct SecurityConfig {
    #[serde(default)]
    pub mode: SecurityMode,
    #[serde(default)]
    pub token_file: Option<String>,
    #[serde(default)]
    pub ca_cert: Option<String>,
    #[serde(default)]
    pub cert: Option<String>,
    #[serde(default)]
    pub key: Option<String>,
    #[serde(default)]
    pub server_name: Option<String>,
    #[serde(default)]
    pub expected_peer_cert_sha256: Option<String>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            mode: SecurityMode::Token,
            token_file: None,
            ca_cert: None,
            cert: None,
            key: None,
            server_name: None,
            expected_peer_cert_sha256: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct DefaultsConfig {
    #[serde(default = "default_drain_timeout_secs")]
    pub drain_timeout_secs: u64,
    #[serde(default = "default_dial_timeout_secs")]
    pub dial_timeout_secs: u64,
}

impl Default for DefaultsConfig {
    fn default() -> Self {
        Self {
            drain_timeout_secs: default_drain_timeout_secs(),
            dial_timeout_secs: default_dial_timeout_secs(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct TunnelConfig {
    pub id: String,
    #[serde(default)]
    pub node_id: Option<String>,
    #[serde(default)]
    pub listen: Option<String>,
    #[serde(default)]
    pub relay_addr: Option<String>,
    #[serde(default)]
    pub token: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AdminConfig {
    #[serde(default = "default_admin_listen")]
    pub listen: String,
    #[serde(default)]
    pub token_file: Option<String>,
    #[serde(skip)]
    token: Option<String>,
}

impl Default for AdminConfig {
    fn default() -> Self {
        Self {
            listen: default_admin_listen(),
            token_file: None,
            token: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct BToAService {
    pub name: String,
    pub expose_on_relay: String,
    pub target_from_agent: String,
    #[serde(default)]
    pub allowed_sources: Vec<String>,
    #[serde(default)]
    pub idle_timeout_secs: Option<u64>,
    #[serde(default)]
    pub dial_timeout_secs: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AToBService {
    pub name: String,
    pub expose_on_agent: String,
    pub target_from_relay: String,
    #[serde(default)]
    pub allowed_sources: Vec<String>,
    #[serde(default)]
    pub idle_timeout_secs: Option<u64>,
    #[serde(default)]
    pub dial_timeout_secs: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Service {
    pub name: String,
    pub direction: Direction,
    expose: String,
    target: String,
    allowed_sources: Vec<String>,
    idle_timeout_secs: Option<u64>,
    dial_timeout_secs: Option<u64>,
}

impl Service {
    pub fn expose_addr(&self) -> &str {
        &self.expose
    }

    pub fn target_addr(&self) -> &str {
        &self.target
    }

    pub fn allowed_sources(&self) -> &[String] {
        &self.allowed_sources
    }

    pub fn idle_timeout_secs(&self) -> Option<u64> {
        self.idle_timeout_secs
    }

    pub fn dial_timeout_secs(&self) -> Option<u64> {
        self.dial_timeout_secs
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub role: Role,
    pub tunnel: TunnelConfig,
    #[serde(default)]
    pub transport: TransportConfig,
    #[serde(default)]
    pub security: SecurityConfig,
    #[serde(default)]
    pub admin: AdminConfig,
    #[serde(default)]
    pub defaults: DefaultsConfig,
    #[serde(default)]
    pub b_to_a: Vec<BToAService>,
    #[serde(default)]
    pub a_to_b: Vec<AToBService>,
    #[serde(skip)]
    services: Vec<Service>,
    #[serde(skip)]
    source_path: Option<PathBuf>,
    #[serde(skip)]
    auth_token: Option<String>,
}

impl Config {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        let body = fs::read_to_string(path)?;
        Self::parse(&body, path)
    }

    pub fn parse(body: &str, path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        let mut config: Config = toml::from_str(&body)?;
        config.source_path = Some(path.to_path_buf());
        config.validate()?;
        Ok(config)
    }

    pub fn service_by_name(&self, name: &str) -> Option<&Service> {
        self.services.iter().find(|service| service.name == name)
    }

    pub fn services(&self) -> &[Service] {
        &self.services
    }

    pub fn source_path(&self) -> Option<&Path> {
        self.source_path.as_deref()
    }

    pub fn auth_token(&self) -> Option<&str> {
        self.auth_token.as_deref()
    }

    pub fn admin_token(&self) -> Option<&str> {
        self.admin.token.as_deref()
    }

    fn validate(&mut self) -> Result<(), ConfigError> {
        require_non_empty("tunnel.id", &self.tunnel.id)?;
        if let Some(node_id) = &self.tunnel.node_id {
            require_non_empty("tunnel.node_id", node_id)?;
        }

        if self.transport.connect_timeout_secs == 0 {
            return Err(ConfigError::Validation(
                "transport.connect_timeout_secs must be greater than 0".into(),
            ));
        }
        if self.transport.idle_timeout_secs == 0 {
            return Err(ConfigError::Validation(
                "transport.idle_timeout_secs must be greater than 0".into(),
            ));
        }
        if self.transport.max_frame_bytes == 0 {
            return Err(ConfigError::Validation(
                "transport.max_frame_bytes must be greater than 0".into(),
            ));
        }

        self.auth_token =
            resolve_auth_token(&self.tunnel.token, self.security.token_file.as_deref())?;
        if self.auth_token.is_none() && self.security.mode == SecurityMode::Token {
            return Err(ConfigError::Validation(
                "tunnel.token or security.token_file is required for token security".into(),
            ));
        }
        match self.role {
            Role::Relay => {
                let listen = self.tunnel.listen.as_deref().ok_or_else(|| {
                    ConfigError::Validation("tunnel.listen is required for relay".into())
                })?;
                validate_addr("tunnel.listen", listen)?;
            }
            Role::Agent => {
                let relay_addr = self.tunnel.relay_addr.as_deref().ok_or_else(|| {
                    ConfigError::Validation("tunnel.relay_addr is required for agent".into())
                })?;
                validate_addr("tunnel.relay_addr", relay_addr)?;
            }
        }
        validate_security_for_transport(self.role, &self.transport, &self.security)?;

        validate_addr_allow_zero("admin.listen", &self.admin.listen)?;
        self.admin.token = match self.admin.token_file.as_deref() {
            Some(path) => Some(read_secret_file("admin.token_file", path)?),
            None => None,
        };

        let mut names = HashSet::new();
        let mut exposes = HashSet::new();
        let mut services = Vec::new();
        for service in &self.b_to_a {
            validate_service_name(&mut names, &service.name)?;
            validate_unique_expose(&mut exposes, &service.expose_on_relay)?;
            validate_addr("b_to_a.expose_on_relay", &service.expose_on_relay)?;
            validate_addr("b_to_a.target_from_agent", &service.target_from_agent)?;
            validate_allowed_sources("b_to_a.allowed_sources", &service.allowed_sources)?;
            services.push(Service {
                name: service.name.trim().to_string(),
                direction: Direction::BToA,
                expose: service.expose_on_relay.trim().to_string(),
                target: service.target_from_agent.trim().to_string(),
                allowed_sources: trim_strings(&service.allowed_sources),
                idle_timeout_secs: service.idle_timeout_secs,
                dial_timeout_secs: service.dial_timeout_secs,
            });
        }
        for service in &self.a_to_b {
            validate_service_name(&mut names, &service.name)?;
            validate_unique_expose(&mut exposes, &service.expose_on_agent)?;
            validate_addr("a_to_b.expose_on_agent", &service.expose_on_agent)?;
            validate_addr("a_to_b.target_from_relay", &service.target_from_relay)?;
            validate_allowed_sources("a_to_b.allowed_sources", &service.allowed_sources)?;
            services.push(Service {
                name: service.name.trim().to_string(),
                direction: Direction::AToB,
                expose: service.expose_on_agent.trim().to_string(),
                target: service.target_from_relay.trim().to_string(),
                allowed_sources: trim_strings(&service.allowed_sources),
                idle_timeout_secs: service.idle_timeout_secs,
                dial_timeout_secs: service.dial_timeout_secs,
            });
        }

        self.services = services;
        Ok(())
    }
}

#[derive(Debug)]
pub enum ConfigError {
    Io(std::io::Error),
    Toml(toml::de::Error),
    Validation(String),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::Io(error) => write!(f, "{error}"),
            ConfigError::Toml(error) => write!(f, "{error}"),
            ConfigError::Validation(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<std::io::Error> for ConfigError {
    fn from(value: std::io::Error) -> Self {
        ConfigError::Io(value)
    }
}

impl From<toml::de::Error> for ConfigError {
    fn from(value: toml::de::Error) -> Self {
        ConfigError::Toml(value)
    }
}

fn default_admin_listen() -> String {
    "127.0.0.1:18080".to_string()
}

fn default_connect_timeout_secs() -> u64 {
    10
}

fn default_idle_timeout_secs() -> u64 {
    300
}

fn default_max_frame_bytes() -> usize {
    1024 * 1024
}

fn default_drain_timeout_secs() -> u64 {
    30
}

fn default_dial_timeout_secs() -> u64 {
    5
}

fn validate_service_name(names: &mut HashSet<String>, value: &str) -> Result<(), ConfigError> {
    let name = value.trim();
    require_non_empty("service.name", name)?;
    if !names.insert(name.to_string()) {
        return Err(ConfigError::Validation(format!(
            "duplicate service name: {name}"
        )));
    }
    Ok(())
}

fn validate_unique_expose(names: &mut HashSet<String>, value: &str) -> Result<(), ConfigError> {
    let expose = value.trim();
    if !names.insert(expose.to_string()) {
        return Err(ConfigError::Validation(format!(
            "duplicate service expose address: {expose}"
        )));
    }
    Ok(())
}

fn require_non_empty(field: &str, value: &str) -> Result<(), ConfigError> {
    if value.trim().is_empty() {
        return Err(ConfigError::Validation(format!(
            "{field} must not be empty"
        )));
    }
    Ok(())
}

fn require_non_empty_option(field: &str, value: Option<&str>) -> Result<(), ConfigError> {
    let Some(value) = value else {
        return Err(ConfigError::Validation(format!("{field} is required")));
    };
    require_non_empty(field, value)
}

fn require_existing_file(field: &str, value: Option<&str>) -> Result<(), ConfigError> {
    let Some(path) = value else {
        return Err(ConfigError::Validation(format!("{field} is required")));
    };
    require_non_empty(field, path)?;
    if !Path::new(path).is_file() {
        return Err(ConfigError::Validation(format!(
            "{field} file does not exist: {path}"
        )));
    }
    Ok(())
}

fn validate_security_for_transport(
    role: Role,
    transport: &TransportConfig,
    security: &SecurityConfig,
) -> Result<(), ConfigError> {
    if let Some(fingerprint) = security.expected_peer_cert_sha256.as_deref() {
        validate_sha256_hex("security.expected_peer_cert_sha256", fingerprint)?;
    }

    if security.mode == SecurityMode::Mtls {
        require_existing_file("security.ca_cert", security.ca_cert.as_deref())?;
        require_existing_file("security.cert", security.cert.as_deref())?;
        require_existing_file("security.key", security.key.as_deref())?;
        require_non_empty_option("security.server_name", security.server_name.as_deref())?;
        return Ok(());
    }

    if transport.mode != TransportMode::Quic {
        return Ok(());
    }

    match role {
        Role::Relay => {
            require_existing_file("security.cert", security.cert.as_deref())?;
            require_existing_file("security.key", security.key.as_deref())?;
        }
        Role::Agent => {
            require_existing_file("security.ca_cert", security.ca_cert.as_deref())?;
            require_non_empty_option("security.server_name", security.server_name.as_deref())?;
        }
    }
    Ok(())
}

fn validate_sha256_hex(field: &str, value: &str) -> Result<(), ConfigError> {
    let value = value.trim();
    if value.len() != 64 || !value.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Err(ConfigError::Validation(format!(
            "{field} must be a 64-character hex SHA-256 fingerprint"
        )));
    }
    Ok(())
}

fn resolve_auth_token(
    inline: &str,
    token_file: Option<&str>,
) -> Result<Option<String>, ConfigError> {
    if let Some(path) = token_file {
        return Ok(Some(read_secret_file("security.token_file", path)?));
    }
    let token = inline.trim();
    if token.is_empty() {
        Ok(None)
    } else {
        Ok(Some(token.to_string()))
    }
}

fn read_secret_file(field: &str, path: &str) -> Result<String, ConfigError> {
    require_non_empty(field, path)?;
    let value = fs::read_to_string(path)?;
    let value = value.trim();
    if value.is_empty() {
        return Err(ConfigError::Validation(format!(
            "{field} must not be empty"
        )));
    }
    Ok(value.to_string())
}

fn trim_strings(values: &[String]) -> Vec<String> {
    values
        .iter()
        .map(|value| value.trim().to_string())
        .collect()
}

fn validate_allowed_sources(field: &str, values: &[String]) -> Result<(), ConfigError> {
    for value in values {
        validate_source_cidr(field, value)?;
    }
    Ok(())
}

fn validate_source_cidr(field: &str, value: &str) -> Result<(), ConfigError> {
    let value = value.trim();
    let Some((ip, prefix)) = value.split_once('/') else {
        return Err(ConfigError::Validation(format!(
            "{field} entry must be CIDR: {value}"
        )));
    };
    let ip: IpAddr = ip.parse().map_err(|_| {
        ConfigError::Validation(format!("{field} entry has invalid IP address: {value}"))
    })?;
    let prefix: u8 = prefix.parse().map_err(|_| {
        ConfigError::Validation(format!("{field} entry has invalid prefix: {value}"))
    })?;
    let max_prefix = if ip.is_ipv4() { 32 } else { 128 };
    if prefix > max_prefix {
        return Err(ConfigError::Validation(format!(
            "{field} entry prefix is too large: {value}"
        )));
    }
    Ok(())
}

fn validate_addr(field: &str, value: &str) -> Result<(), ConfigError> {
    validate_addr_inner(field, value, false)
}

fn validate_addr_allow_zero(field: &str, value: &str) -> Result<(), ConfigError> {
    validate_addr_inner(field, value, true)
}

fn validate_addr_inner(field: &str, value: &str, allow_zero_port: bool) -> Result<(), ConfigError> {
    let trimmed = value.trim();
    let Some((host, port)) = trimmed.rsplit_once(':') else {
        return Err(ConfigError::Validation(format!(
            "{field} must be in host:port form"
        )));
    };
    if host.trim().is_empty() {
        return Err(ConfigError::Validation(format!(
            "{field} host must not be empty"
        )));
    }
    let port = port.parse::<u16>().map_err(|_| {
        ConfigError::Validation(format!("{field} port must be a number between 1 and 65535"))
    })?;
    if port == 0 && !allow_zero_port {
        return Err(ConfigError::Validation(format!(
            "{field} port must be greater than 0"
        )));
    }
    Ok(())
}
