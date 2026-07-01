use std::{
    env,
    error::Error,
    fmt, fs,
    io::{Read, Write},
    net::TcpStream as StdTcpStream,
    path::PathBuf,
};

use crate::{
    certs,
    config::{Config, Role},
    runtime::Runtime,
};

pub fn config_path_from_args<I>(args: I) -> Result<PathBuf, CliError>
where
    I: IntoIterator<Item = String>,
{
    let mut args = args.into_iter();
    let _program = args.next();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--config" | "-c" => {
                let value = args.next().ok_or(CliError::MissingConfigPath)?;
                return Ok(PathBuf::from(value));
            }
            "--help" | "-h" => return Err(CliError::Help),
            other => return Err(CliError::UnknownArgument(other.to_string())),
        }
    }
    Err(CliError::MissingConfigPath)
}

pub async fn run(expected_role: Role) -> Result<(), Box<dyn Error>> {
    let path = config_path_from_args(env::args())?;
    let config = Config::load(&path)?;
    if config.role != expected_role {
        return Err(format!(
            "config role is {:?}, but this binary expects {:?}",
            config.role, expected_role
        )
        .into());
    }

    let runtime = Runtime::spawn(config).await?;
    tokio::signal::ctrl_c().await?;
    runtime.shutdown().await;
    Ok(())
}

pub async fn run_ctl() -> Result<(), Box<dyn Error>> {
    let command = ctl_command_from_args(env::args())?;
    execute_ctl_command(command)?;
    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
pub enum CtlCommand {
    CheckConfig {
        config: PathBuf,
    },
    GenToken {
        out: PathBuf,
    },
    Reload {
        admin: String,
        token_file: Option<PathBuf>,
    },
    CertFingerprint {
        cert: PathBuf,
    },
}

pub fn ctl_command_from_args<I>(args: I) -> Result<CtlCommand, CliError>
where
    I: IntoIterator<Item = String>,
{
    let mut args = args.into_iter();
    let _program = args.next();
    let Some(command) = args.next() else {
        return Err(CliError::MissingCtlCommand);
    };

    match command.as_str() {
        "check-config" => {
            let config = parse_required_path_arg(&mut args, "--config")?;
            reject_trailing_arg(&mut args)?;
            Ok(CtlCommand::CheckConfig { config })
        }
        "gen-token" => {
            let out = parse_required_path_arg(&mut args, "--out")?;
            reject_trailing_arg(&mut args)?;
            Ok(CtlCommand::GenToken { out })
        }
        "reload" => {
            let mut admin = None;
            let mut token_file = None;
            while let Some(arg) = args.next() {
                match arg.as_str() {
                    "--admin" => {
                        admin = Some(args.next().ok_or(CliError::MissingValue("--admin"))?)
                    }
                    "--token-file" => {
                        token_file = Some(PathBuf::from(
                            args.next().ok_or(CliError::MissingValue("--token-file"))?,
                        ));
                    }
                    other => return Err(CliError::UnknownArgument(other.to_string())),
                }
            }
            Ok(CtlCommand::Reload {
                admin: admin.ok_or(CliError::MissingValue("--admin"))?,
                token_file,
            })
        }
        "cert-fingerprint" => {
            let cert = parse_required_path_arg(&mut args, "--cert")?;
            reject_trailing_arg(&mut args)?;
            Ok(CtlCommand::CertFingerprint { cert })
        }
        "--help" | "-h" => Err(CliError::Help),
        other => Err(CliError::UnknownArgument(other.to_string())),
    }
}

pub fn generate_token() -> std::io::Result<String> {
    let mut random = [0_u8; 32];
    fs::File::open("/dev/urandom")?.read_exact(&mut random)?;
    let mut token = String::with_capacity(64);
    for byte in random {
        token.push_str(&format!("{byte:02x}"));
    }
    Ok(token)
}

pub fn certificate_fingerprint_from_path(
    path: impl AsRef<std::path::Path>,
) -> certs::CertResult<String> {
    certs::certificate_fingerprint_from_path(path)
}

fn execute_ctl_command(command: CtlCommand) -> Result<(), Box<dyn Error>> {
    match command {
        CtlCommand::CheckConfig { config } => {
            let config = Config::load(config)?;
            println!(
                "ok role={:?} tunnel_id={} services={}",
                config.role,
                config.tunnel.id,
                config.services().len()
            );
        }
        CtlCommand::GenToken { out } => {
            let token = generate_token()?;
            fs::write(out, format!("{token}\n"))?;
            println!("{token}");
        }
        CtlCommand::Reload { admin, token_file } => {
            let token = match token_file {
                Some(path) => Some(fs::read_to_string(path)?.trim().to_string()),
                None => None,
            };
            let response = post_reload(&admin, token.as_deref())?;
            print!("{response}");
            if !response.starts_with("HTTP/1.1 2") {
                return Err("reload request failed".into());
            }
        }
        CtlCommand::CertFingerprint { cert } => {
            let fingerprint = certs::certificate_fingerprint_from_path(cert)
                .map_err(|error| format!("failed to compute certificate fingerprint: {error}"))?;
            println!("{fingerprint}");
        }
    }
    Ok(())
}

fn parse_required_path_arg<I>(args: &mut I, name: &'static str) -> Result<PathBuf, CliError>
where
    I: Iterator<Item = String>,
{
    match args.next().as_deref() {
        Some(value) if value == name => {
            let value = args.next().ok_or(CliError::MissingValue(name))?;
            Ok(PathBuf::from(value))
        }
        Some(other) => Err(CliError::UnknownArgument(other.to_string())),
        None => Err(CliError::MissingValue(name)),
    }
}

fn reject_trailing_arg<I>(args: &mut I) -> Result<(), CliError>
where
    I: Iterator<Item = String>,
{
    if let Some(arg) = args.next() {
        return Err(CliError::UnknownArgument(arg));
    }
    Ok(())
}

fn post_reload(admin: &str, token: Option<&str>) -> Result<String, Box<dyn Error>> {
    let address = admin.strip_prefix("http://").unwrap_or(admin);
    let address = address.trim_end_matches('/');
    let mut stream = StdTcpStream::connect(address)?;
    let auth = token
        .map(|token| format!("Authorization: Bearer {token}\r\n"))
        .unwrap_or_default();
    let request = format!(
        "POST /v1/services/reload HTTP/1.1\r\nHost: {address}\r\n{auth}Content-Length: 0\r\nConnection: close\r\n\r\n"
    );
    stream.write_all(request.as_bytes())?;
    let mut response = String::new();
    stream.read_to_string(&mut response)?;
    Ok(response)
}

#[derive(Debug, PartialEq, Eq)]
pub enum CliError {
    MissingConfigPath,
    MissingCtlCommand,
    MissingValue(&'static str),
    UnknownArgument(String),
    Help,
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliError::MissingConfigPath | CliError::Help => {
                write!(f, "usage: biz-relay|biz-agent --config <path>")
            }
            CliError::MissingCtlCommand => {
                write!(
                    f,
                    "usage: biz-tunnelctl <check-config|gen-token|reload|cert-fingerprint>"
                )
            }
            CliError::MissingValue(name) => write!(f, "missing value for {name}"),
            CliError::UnknownArgument(arg) => write!(f, "unknown argument: {arg}"),
        }
    }
}

impl Error for CliError {}
