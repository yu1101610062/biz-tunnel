use std::{
    fs,
    net::{SocketAddr, TcpListener as StdTcpListener},
    path::{Path, PathBuf},
    time::Duration,
};

use biz_tunnel::{config::Config, runtime::Runtime};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    time::{sleep, timeout},
};

const SERVER_FINGERPRINT: &str = "7d1b1548bb35bca623e35a75ecf7032280ac7d226ba9f94175f9e4e56211cec9";
const AGENT_FINGERPRINT: &str = "116222b9f9dd043d6943bdc3ab6527f660ab54269aa9f888f2041bf74db0c118";

#[tokio::test]
async fn quic_token_mode_proxies_both_directions_without_tcp_tunnel_listener() {
    let a_target = spawn_echo_server("a").await;
    let b_target = spawn_echo_server("b").await;
    let tunnel_port = free_port();
    let b_expose_port = free_port();
    let a_expose_port = free_port();
    let relay_admin_port = free_port();
    let dir = tempfile::tempdir().expect("tempdir");
    let certs = write_quic_fixtures(dir.path());

    let relay_config_path = dir.path().join("relay.toml");
    let relay_body = relay_config(
        tunnel_port,
        b_expose_port,
        a_expose_port,
        &a_target.to_string(),
        &b_target.to_string(),
        &certs,
        r#"
[security]
mode = "token"
cert = "__SERVER_CERT__"
key = "__SERVER_KEY__"
"#,
    )
    .replace(
        "listen = \"127.0.0.1:0\"",
        &format!("listen = \"127.0.0.1:{relay_admin_port}\""),
    )
    .replace(
        "[[b_to_a]]",
        "[defaults]\ndial_timeout_secs = 1\n\n[[b_to_a]]",
    );
    write_config(&relay_config_path, &relay_body);

    let agent_config_path = dir.path().join("agent.toml");
    let agent_body = agent_config(
        tunnel_port,
        b_expose_port,
        a_expose_port,
        &a_target.to_string(),
        &b_target.to_string(),
        &certs,
        r#"
[security]
mode = "token"
ca_cert = "__CA_CERT__"
server_name = "localhost"
"#,
    )
    .replace(
        "[[b_to_a]]",
        "[defaults]\ndial_timeout_secs = 1\n\n[[b_to_a]]",
    );
    write_config(&agent_config_path, &agent_body);

    let relay = Runtime::spawn(Config::load(&relay_config_path).expect("relay config"))
        .await
        .expect("relay starts");
    assert_tcp_connect_fails(tunnel_port).await;
    let agent = Runtime::spawn(Config::load(&agent_config_path).expect("agent config"))
        .await
        .expect("agent starts");

    wait_connected(&relay).await;
    wait_for_tcp(("127.0.0.1", b_expose_port)).await;
    wait_for_tcp(("127.0.0.1", a_expose_port)).await;
    wait_for_tcp(("127.0.0.1", relay_admin_port)).await;

    let remote_test = http_post(relay_admin_port, "/v1/services/test/b-echo").await;
    assert!(
        remote_test.contains(r#""status":"ok""#),
        "unexpected remote QUIC route test: {remote_test}"
    );

    let b_to_a_reply = round_trip(("127.0.0.1", b_expose_port), b"from-b").await;
    assert_eq!(b_to_a_reply, b"a:from-b");

    let a_to_b_reply = round_trip(("127.0.0.1", a_expose_port), b"from-a").await;
    assert_eq!(a_to_b_reply, b"b:from-a");

    agent.shutdown().await;
    relay.shutdown().await;
}

#[tokio::test]
async fn quic_rejects_wrong_server_name() {
    let a_target = spawn_echo_server("a").await;
    let tunnel_port = free_port();
    let b_expose_port = free_port();
    let dir = tempfile::tempdir().expect("tempdir");
    let certs = write_quic_fixtures(dir.path());

    let relay_config_path = dir.path().join("relay.toml");
    write_config(
        &relay_config_path,
        &relay_config(
            tunnel_port,
            b_expose_port,
            free_port(),
            &a_target.to_string(),
            "127.0.0.1:9",
            &certs,
            r#"
[security]
mode = "token"
cert = "__SERVER_CERT__"
key = "__SERVER_KEY__"
"#,
        ),
    );

    let agent_config_path = dir.path().join("agent.toml");
    write_config(
        &agent_config_path,
        &agent_config(
            tunnel_port,
            b_expose_port,
            free_port(),
            &a_target.to_string(),
            "127.0.0.1:9",
            &certs,
            r#"
[security]
mode = "token"
ca_cert = "__CA_CERT__"
server_name = "wrong.localhost"
"#,
        ),
    );

    let relay = Runtime::spawn(Config::load(&relay_config_path).expect("relay config"))
        .await
        .expect("relay starts");
    let agent = Runtime::spawn(Config::load(&agent_config_path).expect("agent config"))
        .await
        .expect("agent starts");

    sleep(Duration::from_millis(250)).await;
    assert!(
        !relay.status().await.agent_connected,
        "relay should not accept an agent with the wrong server_name"
    );

    agent.shutdown().await;
    relay.shutdown().await;
}

#[tokio::test]
async fn quic_mtls_mode_proxies_with_client_certificate() {
    let a_target = spawn_echo_server("a").await;
    let tunnel_port = free_port();
    let b_expose_port = free_port();
    let dir = tempfile::tempdir().expect("tempdir");
    let certs = write_quic_fixtures(dir.path());

    let relay_config_path = dir.path().join("relay.toml");
    write_config(
        &relay_config_path,
        &relay_config(
            tunnel_port,
            b_expose_port,
            free_port(),
            &a_target.to_string(),
            "127.0.0.1:9",
            &certs,
            &format!(
                r#"
[security]
mode = "mtls"
ca_cert = "__CA_CERT__"
cert = "__SERVER_CERT__"
key = "__SERVER_KEY__"
server_name = "localhost"
expected_peer_cert_sha256 = "{AGENT_FINGERPRINT}"
"#
            ),
        ),
    );

    let agent_config_path = dir.path().join("agent.toml");
    write_config(
        &agent_config_path,
        &agent_config(
            tunnel_port,
            b_expose_port,
            free_port(),
            &a_target.to_string(),
            "127.0.0.1:9",
            &certs,
            &format!(
                r#"
[security]
mode = "mtls"
ca_cert = "__CA_CERT__"
cert = "__AGENT_CERT__"
key = "__AGENT_KEY__"
server_name = "localhost"
expected_peer_cert_sha256 = "{SERVER_FINGERPRINT}"
"#
            ),
        ),
    );

    let relay = Runtime::spawn(Config::load(&relay_config_path).expect("relay config"))
        .await
        .expect("relay starts");
    assert_tcp_connect_fails(tunnel_port).await;
    let agent = Runtime::spawn(Config::load(&agent_config_path).expect("agent config"))
        .await
        .expect("agent starts");

    wait_connected(&relay).await;
    let b_to_a_reply = round_trip(("127.0.0.1", b_expose_port), b"from-b").await;
    assert_eq!(b_to_a_reply, b"a:from-b");

    agent.shutdown().await;
    relay.shutdown().await;
}

#[tokio::test]
async fn quic_rejects_peer_certificate_fingerprint_mismatch() {
    let a_target = spawn_echo_server("a").await;
    let tunnel_port = free_port();
    let b_expose_port = free_port();
    let dir = tempfile::tempdir().expect("tempdir");
    let certs = write_quic_fixtures(dir.path());

    let relay_config_path = dir.path().join("relay.toml");
    write_config(
        &relay_config_path,
        &relay_config(
            tunnel_port,
            b_expose_port,
            free_port(),
            &a_target.to_string(),
            "127.0.0.1:9",
            &certs,
            r#"
[security]
mode = "token"
cert = "__SERVER_CERT__"
key = "__SERVER_KEY__"
"#,
        ),
    );

    let agent_config_path = dir.path().join("agent.toml");
    write_config(
        &agent_config_path,
        &agent_config(
            tunnel_port,
            b_expose_port,
            free_port(),
            &a_target.to_string(),
            "127.0.0.1:9",
            &certs,
            r#"
[security]
mode = "token"
ca_cert = "__CA_CERT__"
server_name = "localhost"
expected_peer_cert_sha256 = "0000000000000000000000000000000000000000000000000000000000000000"
"#,
        ),
    );

    let relay = Runtime::spawn(Config::load(&relay_config_path).expect("relay config"))
        .await
        .expect("relay starts");
    let agent = Runtime::spawn(Config::load(&agent_config_path).expect("agent config"))
        .await
        .expect("agent starts");

    sleep(Duration::from_millis(250)).await;
    assert!(
        !relay.status().await.agent_connected,
        "relay should reject a peer certificate fingerprint mismatch"
    );

    agent.shutdown().await;
    relay.shutdown().await;
}

#[derive(Debug)]
struct QuicFixtures {
    ca_cert: PathBuf,
    server_cert: PathBuf,
    server_key: PathBuf,
    agent_cert: PathBuf,
    agent_key: PathBuf,
}

fn relay_config(
    tunnel_port: u16,
    b_expose_port: u16,
    a_expose_port: u16,
    a_target: &str,
    b_target: &str,
    certs: &QuicFixtures,
    security: &str,
) -> String {
    replace_cert_placeholders(
        &format!(
            r#"
role = "relay"

[tunnel]
id = "room-a-to-room-b"
listen = "127.0.0.1:{tunnel_port}"
token = "secret-token"

[transport]
mode = "quic"

{security}

[admin]
listen = "127.0.0.1:0"

[[b_to_a]]
name = "a-echo"
expose_on_relay = "127.0.0.1:{b_expose_port}"
target_from_agent = "{a_target}"

[[a_to_b]]
name = "b-echo"
expose_on_agent = "127.0.0.1:{a_expose_port}"
target_from_relay = "{b_target}"
"#
        ),
        certs,
    )
}

fn agent_config(
    tunnel_port: u16,
    b_expose_port: u16,
    a_expose_port: u16,
    a_target: &str,
    b_target: &str,
    certs: &QuicFixtures,
    security: &str,
) -> String {
    replace_cert_placeholders(
        &format!(
            r#"
role = "agent"

[tunnel]
id = "room-a-to-room-b"
relay_addr = "127.0.0.1:{tunnel_port}"
token = "secret-token"

[transport]
mode = "quic"

{security}

[admin]
listen = "127.0.0.1:0"

[[b_to_a]]
name = "a-echo"
expose_on_relay = "127.0.0.1:{b_expose_port}"
target_from_agent = "{a_target}"

[[a_to_b]]
name = "b-echo"
expose_on_agent = "127.0.0.1:{a_expose_port}"
target_from_relay = "{b_target}"
"#
        ),
        certs,
    )
}

fn replace_cert_placeholders(config: &str, certs: &QuicFixtures) -> String {
    config
        .replace("__CA_CERT__", &certs.ca_cert.display().to_string())
        .replace("__SERVER_CERT__", &certs.server_cert.display().to_string())
        .replace("__SERVER_KEY__", &certs.server_key.display().to_string())
        .replace("__AGENT_CERT__", &certs.agent_cert.display().to_string())
        .replace("__AGENT_KEY__", &certs.agent_key.display().to_string())
}

fn write_quic_fixtures(dir: &Path) -> QuicFixtures {
    let fixture_dir = dir.join("quic-fixtures");
    fs::create_dir_all(&fixture_dir).expect("create fixtures");
    write_fixture(&fixture_dir, "ca.pem", include_str!("fixtures/quic/ca.pem"));
    write_fixture(
        &fixture_dir,
        "server.pem",
        include_str!("fixtures/quic/server.pem"),
    );
    write_fixture(
        &fixture_dir,
        "server.key",
        include_str!("fixtures/quic/server.key"),
    );
    write_fixture(
        &fixture_dir,
        "agent.pem",
        include_str!("fixtures/quic/agent.pem"),
    );
    write_fixture(
        &fixture_dir,
        "agent.key",
        include_str!("fixtures/quic/agent.key"),
    );

    QuicFixtures {
        ca_cert: fixture_dir.join("ca.pem"),
        server_cert: fixture_dir.join("server.pem"),
        server_key: fixture_dir.join("server.key"),
        agent_cert: fixture_dir.join("agent.pem"),
        agent_key: fixture_dir.join("agent.key"),
    }
}

fn write_fixture(dir: &Path, name: &str, body: &str) {
    fs::write(dir.join(name), body).expect("write fixture");
}

async fn spawn_echo_server(prefix: &'static str) -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind echo server");
    let addr = listener.local_addr().expect("echo local addr");
    tokio::spawn(async move {
        loop {
            let Ok((mut socket, _)) = listener.accept().await else {
                break;
            };
            tokio::spawn(async move {
                let mut buf = vec![0_u8; 64 * 1024];
                let Ok(n) = socket.read(&mut buf).await else {
                    return;
                };
                if n == 0 {
                    return;
                }
                let mut reply = prefix.as_bytes().to_vec();
                reply.push(b':');
                reply.extend_from_slice(&buf[..n]);
                let _ = socket.write_all(&reply).await;
                let _ = socket.shutdown().await;
            });
        }
    });
    addr
}

async fn round_trip(addr: (&str, u16), payload: &[u8]) -> Vec<u8> {
    timeout(Duration::from_secs(3), async {
        let mut socket = TcpStream::connect(addr).await.expect("connect expose port");
        socket.write_all(payload).await.expect("write payload");
        socket.shutdown().await.expect("shutdown write side");
        let mut reply = Vec::new();
        socket.read_to_end(&mut reply).await.expect("read reply");
        reply
    })
    .await
    .expect("round trip should complete")
}

async fn http_post(port: u16, path: &str) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port))
        .await
        .expect("connect admin");
    let request = format!(
        "POST {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    );
    stream
        .write_all(request.as_bytes())
        .await
        .expect("write request");
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .await
        .expect("read response");
    response
}

async fn wait_connected(runtime: &Runtime) {
    timeout(Duration::from_secs(3), async {
        loop {
            if runtime.status().await.agent_connected {
                return;
            }
            sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("agent should connect");
}

async fn wait_for_tcp(addr: (&str, u16)) {
    timeout(Duration::from_secs(3), async {
        loop {
            match TcpStream::connect(addr).await {
                Ok(_) => return,
                Err(_) => sleep(Duration::from_millis(25)).await,
            }
        }
    })
    .await
    .expect("tcp listener should become ready");
}

async fn assert_tcp_connect_fails(port: u16) {
    let result = timeout(
        Duration::from_millis(250),
        TcpStream::connect(("127.0.0.1", port)),
    )
    .await;
    assert!(
        !matches!(result, Ok(Ok(_))),
        "QUIC tunnel port {port} must not accept TCP connections"
    );
}

fn write_config(path: &Path, body: &str) {
    fs::write(path, body).expect("write config");
}

fn free_port() -> u16 {
    let listener = StdTcpListener::bind("127.0.0.1:0").expect("bind free port probe");
    listener.local_addr().expect("local addr").port()
}
