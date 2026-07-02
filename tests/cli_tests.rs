use biz_tunnel::cli::{
    CtlCommand, certificate_fingerprint_from_path, config_path_from_args, ctl_command_from_args,
    generate_token,
};

#[test]
fn parses_config_path_flag() {
    let args = vec![
        "biz-tunnel".to_string(),
        "--config".to_string(),
        "/etc/biz-tunnel/relay.toml".to_string(),
    ];

    let path = config_path_from_args(args).expect("config path should parse");

    assert_eq!(path.to_string_lossy(), "/etc/biz-tunnel/relay.toml");
}

#[test]
fn rejects_missing_config_path() {
    let args = vec!["biz-tunnel".to_string()];

    let err = config_path_from_args(args).expect_err("missing config should fail");

    assert!(
        err.to_string().contains("--config <path>"),
        "unexpected error: {err}"
    );
}

#[test]
fn parses_ctl_check_config_command() {
    let args = vec![
        "biz-tunnel".to_string(),
        "check-config".to_string(),
        "--config".to_string(),
        "/etc/biz-tunnel/relay.toml".to_string(),
    ];

    let command = ctl_command_from_args(args).expect("ctl command should parse");

    assert_eq!(
        command,
        CtlCommand::CheckConfig {
            config: "/etc/biz-tunnel/relay.toml".into()
        }
    );
}

#[test]
fn generates_url_safe_token() {
    let token = generate_token().expect("token should generate");

    assert_eq!(token.len(), 64);
    assert!(token.chars().all(|ch| ch.is_ascii_hexdigit()));
}

#[test]
fn parses_ctl_cert_fingerprint_command() {
    let args = vec![
        "biz-tunnel".to_string(),
        "cert-fingerprint".to_string(),
        "--cert".to_string(),
        "tests/fixtures/quic/server.pem".to_string(),
    ];

    let command = ctl_command_from_args(args).expect("ctl command should parse");

    assert_eq!(
        command,
        CtlCommand::CertFingerprint {
            cert: "tests/fixtures/quic/server.pem".into()
        }
    );
}

#[test]
fn computes_certificate_fingerprint_from_pem() {
    let fingerprint = certificate_fingerprint_from_path("tests/fixtures/quic/server.pem")
        .expect("fingerprint should compute");

    assert_eq!(
        fingerprint,
        "7d1b1548bb35bca623e35a75ecf7032280ac7d226ba9f94175f9e4e56211cec9"
    );
}
