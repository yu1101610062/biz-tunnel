use std::fs;

use biz_tunnel::config::{Config, Direction, Role, SecurityMode, TransportMode};

#[test]
fn parses_bidirectional_service_config() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("biz-tunnel.toml");
    fs::write(
        &path,
        r#"
role = "relay"

[tunnel]
id = "room-a-to-room-b"
listen = "127.0.0.1:9443"
token = "secret-token"

[admin]
listen = "127.0.0.1:18080"

[[b_to_a]]
name = "a-order-grpc"
expose_on_relay = "127.0.0.1:15001"
target_from_agent = "10.10.1.20:50051"

[[a_to_b]]
name = "b-inventory-http"
expose_on_agent = "127.0.0.1:16001"
target_from_relay = "10.20.1.30:8080"
"#,
    )
    .expect("write config");

    let config = Config::load(&path).expect("config should parse");

    assert_eq!(config.role, Role::Relay);
    assert_eq!(config.tunnel.id, "room-a-to-room-b");
    assert_eq!(config.tunnel.listen.as_deref(), Some("127.0.0.1:9443"));

    let b_to_a = config
        .service_by_name("a-order-grpc")
        .expect("b_to_a service should be indexed");
    assert_eq!(b_to_a.direction, Direction::BToA);
    assert_eq!(b_to_a.expose_addr(), "127.0.0.1:15001");
    assert_eq!(b_to_a.target_addr(), "10.10.1.20:50051");

    let a_to_b = config
        .service_by_name("b-inventory-http")
        .expect("a_to_b service should be indexed");
    assert_eq!(a_to_b.direction, Direction::AToB);
    assert_eq!(a_to_b.expose_addr(), "127.0.0.1:16001");
    assert_eq!(a_to_b.target_addr(), "10.20.1.30:8080");
}

#[test]
fn rejects_duplicate_service_names_across_directions() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("biz-tunnel.toml");
    fs::write(
        &path,
        r#"
role = "agent"

[tunnel]
id = "room-a-to-room-b"
relay_addr = "127.0.0.1:9443"
token = "secret-token"

[[b_to_a]]
name = "duplicate"
expose_on_relay = "127.0.0.1:15001"
target_from_agent = "10.10.1.20:50051"

[[a_to_b]]
name = "duplicate"
expose_on_agent = "127.0.0.1:16001"
target_from_relay = "10.20.1.30:8080"
"#,
    )
    .expect("write config");

    let err = Config::load(&path).expect_err("duplicate service names should fail");

    assert!(
        err.to_string().contains("duplicate service name"),
        "unexpected error: {err}"
    );
}

#[test]
fn rejects_role_specific_missing_tunnel_address() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("biz-tunnel.toml");
    fs::write(
        &path,
        r#"
role = "agent"

[tunnel]
id = "room-a-to-room-b"
token = "secret-token"
"#,
    )
    .expect("write config");

    let err = Config::load(&path).expect_err("agent without relay_addr should fail");

    assert!(
        err.to_string().contains("tunnel.relay_addr is required"),
        "unexpected error: {err}"
    );
}

#[test]
fn parses_production_transport_security_and_service_options() {
    let dir = tempfile::tempdir().expect("tempdir");
    let token_path = dir.path().join("token");
    let admin_token_path = dir.path().join("admin-token");
    let ca_path = dir.path().join("ca.pem");
    let cert_path = dir.path().join("node.pem");
    let key_path = dir.path().join("node-key.pem");
    fs::write(&token_path, "secret-token\n").expect("token");
    fs::write(&admin_token_path, "admin-token\n").expect("admin token");
    fs::write(&ca_path, "ca").expect("ca");
    fs::write(&cert_path, "cert").expect("cert");
    fs::write(&key_path, "key").expect("key");

    let path = dir.path().join("biz-tunnel.toml");
    fs::write(
        &path,
        format!(
            r#"
role = "relay"

[tunnel]
id = "room-a-to-room-b"
node_id = "b-relay-1"
listen = "127.0.0.1:9443"

[transport]
mode = "quic"
fallback = ["tls_tcp"]
connect_timeout_secs = 7
idle_timeout_secs = 301
max_frame_bytes = 2048

[security]
mode = "mtls"
token_file = "{}"
ca_cert = "{}"
cert = "{}"
key = "{}"
server_name = "biz-relay.local"
expected_peer_cert_sha256 = "7d1b1548bb35bca623e35a75ecf7032280ac7d226ba9f94175f9e4e56211cec9"

[admin]
listen = "127.0.0.1:18080"
token_file = "{}"

[defaults]
drain_timeout_secs = 9
dial_timeout_secs = 4

[[b_to_a]]
name = "a-order-grpc"
expose_on_relay = "127.0.0.1:15001"
target_from_agent = "10.10.1.20:50051"
allowed_sources = ["127.0.0.1/32"]
idle_timeout_secs = 600
dial_timeout_secs = 3
"#,
            token_path.display(),
            ca_path.display(),
            cert_path.display(),
            key_path.display(),
            admin_token_path.display()
        ),
    )
    .expect("write config");

    let config = Config::load(&path).expect("config should parse");

    assert_eq!(config.transport.mode, TransportMode::Quic);
    assert_eq!(config.transport.fallback, vec![TransportMode::TlsTcp]);
    assert_eq!(config.transport.connect_timeout_secs, 7);
    assert_eq!(config.transport.idle_timeout_secs, 301);
    assert_eq!(config.transport.max_frame_bytes, 2048);
    assert_eq!(config.security.mode, SecurityMode::Mtls);
    assert_eq!(
        config.security.expected_peer_cert_sha256.as_deref(),
        Some("7d1b1548bb35bca623e35a75ecf7032280ac7d226ba9f94175f9e4e56211cec9")
    );
    assert_eq!(config.auth_token(), Some("secret-token"));
    assert_eq!(config.admin_token(), Some("admin-token"));
    assert_eq!(config.defaults.drain_timeout_secs, 9);
    assert_eq!(config.defaults.dial_timeout_secs, 4);

    let service = config
        .service_by_name("a-order-grpc")
        .expect("service should be indexed");
    assert_eq!(service.allowed_sources(), &["127.0.0.1/32"]);
    assert_eq!(service.idle_timeout_secs(), Some(600));
    assert_eq!(service.dial_timeout_secs(), Some(3));
}

#[test]
fn rejects_quic_relay_without_certificate() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("biz-tunnel.toml");
    fs::write(
        &path,
        r#"
role = "relay"

[tunnel]
id = "room-a-to-room-b"
listen = "127.0.0.1:9443"
token = "secret-token"

[transport]
mode = "quic"

[security]
mode = "token"
"#,
    )
    .expect("write config");

    let err = Config::load(&path).expect_err("quic relay without cert/key should fail");

    assert!(
        err.to_string().contains("security.cert"),
        "unexpected error: {err}"
    );
}

#[test]
fn rejects_invalid_service_source_cidr() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("biz-tunnel.toml");
    fs::write(
        &path,
        r#"
role = "relay"

[tunnel]
id = "room-a-to-room-b"
listen = "127.0.0.1:9443"
token = "secret-token"

[[b_to_a]]
name = "bad-source"
expose_on_relay = "127.0.0.1:15001"
target_from_agent = "10.10.1.20:50051"
allowed_sources = ["not-a-cidr"]
"#,
    )
    .expect("write config");

    let err = Config::load(&path).expect_err("invalid CIDR should fail");

    assert!(
        err.to_string().contains("allowed_sources"),
        "unexpected error: {err}"
    );
}
