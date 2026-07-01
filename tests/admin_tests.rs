use std::{fs, net::TcpListener as StdTcpListener, path::Path, time::Duration};

use biz_tunnel::{config::Config, runtime::Runtime};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    time::{sleep, timeout},
};

#[tokio::test]
async fn admin_api_reports_health_services_tunnel_and_metrics() {
    let tunnel_port = free_port();
    let expose_port = free_port();
    let admin_port = free_port();
    let dir = tempfile::tempdir().expect("tempdir");
    let config_path = dir.path().join("relay.toml");
    write_config(
        &config_path,
        &format!(
            r#"
role = "relay"

[tunnel]
id = "room-a-to-room-b"
listen = "127.0.0.1:{tunnel_port}"
token = "secret-token"

[admin]
listen = "127.0.0.1:{admin_port}"

[[b_to_a]]
name = "a-order-grpc"
expose_on_relay = "127.0.0.1:{expose_port}"
target_from_agent = "10.10.1.20:50051"
"#
        ),
    );

    let runtime = Runtime::spawn(Config::load(&config_path).expect("config"))
        .await
        .expect("runtime starts");
    wait_for_tcp(("127.0.0.1", admin_port)).await;

    let health = http_get(admin_port, "/healthz").await;
    assert!(
        health.contains("200 OK"),
        "unexpected health response: {health}"
    );
    assert!(
        health.contains(r#""status":"ok""#),
        "unexpected health body: {health}"
    );

    let services = http_get(admin_port, "/v1/services").await;
    assert!(
        services.contains("a-order-grpc"),
        "unexpected services body: {services}"
    );
    assert!(
        services.contains("b_to_a"),
        "unexpected services body: {services}"
    );

    let tunnel = http_get(admin_port, "/v1/tunnel").await;
    assert!(
        tunnel.contains(r#""agent_connected":false"#),
        "unexpected tunnel body: {tunnel}"
    );

    let metrics = http_get(admin_port, "/metrics").await;
    assert!(
        metrics.contains("biz_tunnel_agent_connected 0"),
        "unexpected metrics body: {metrics}"
    );

    runtime.shutdown().await;
}

#[tokio::test]
async fn admin_api_requires_token_for_management_and_reloads_services() {
    let tunnel_port = free_port();
    let expose_port = free_port();
    let admin_port = free_port();
    let dir = tempfile::tempdir().expect("tempdir");
    let token_path = dir.path().join("admin-token");
    fs::write(&token_path, "admin-secret\n").expect("write admin token");
    let config_path = dir.path().join("relay.toml");
    write_config(
        &config_path,
        &format!(
            r#"
role = "relay"

[tunnel]
id = "room-a-to-room-b"
listen = "127.0.0.1:{tunnel_port}"
token = "secret-token"

[admin]
listen = "127.0.0.1:{admin_port}"
token_file = "{}"
"#,
            token_path.display()
        ),
    );

    let runtime = Runtime::spawn(Config::load(&config_path).expect("config"))
        .await
        .expect("runtime starts");
    wait_for_tcp(("127.0.0.1", admin_port)).await;

    let unauthorized = http_get(admin_port, "/v1/services").await;
    assert!(
        unauthorized.contains("401 Unauthorized"),
        "unexpected unauthorized response: {unauthorized}"
    );

    let authorized = http_get_with_token(admin_port, "/v1/services", Some("admin-secret")).await;
    assert!(
        authorized.contains("200 OK"),
        "unexpected authorized response: {authorized}"
    );

    write_config(
        &config_path,
        &format!(
            r#"
role = "relay"

[tunnel]
id = "room-a-to-room-b"
listen = "127.0.0.1:{tunnel_port}"
token = "secret-token"

[admin]
listen = "127.0.0.1:{admin_port}"
token_file = "{}"

[[b_to_a]]
name = "a-new"
expose_on_relay = "127.0.0.1:{expose_port}"
target_from_agent = "127.0.0.1:9"
"#,
            token_path.display()
        ),
    );

    let reload =
        http_post_with_token(admin_port, "/v1/services/reload", Some("admin-secret"), "").await;
    assert!(
        reload.contains("200 OK") && reload.contains(r#""status":"applied""#),
        "unexpected reload response: {reload}"
    );
    assert!(
        reload.contains(r#""added":["a-new"]"#),
        "unexpected reload diff: {reload}"
    );
    wait_for_tcp(("127.0.0.1", expose_port)).await;

    runtime.shutdown().await;
}

async fn http_get(port: u16, path: &str) -> String {
    http_get_with_token(port, path, None).await
}

async fn http_get_with_token(port: u16, path: &str, token: Option<&str>) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port))
        .await
        .expect("connect admin");
    let auth = token
        .map(|token| format!("Authorization: Bearer {token}\r\n"))
        .unwrap_or_default();
    let request =
        format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\n{auth}Connection: close\r\n\r\n");
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

async fn http_post_with_token(port: u16, path: &str, token: Option<&str>, body: &str) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port))
        .await
        .expect("connect admin");
    let auth = token
        .map(|token| format!("Authorization: Bearer {token}\r\n"))
        .unwrap_or_default();
    let request = format!(
        "POST {path} HTTP/1.1\r\nHost: 127.0.0.1\r\n{auth}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
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

fn write_config(path: &Path, body: &str) {
    fs::write(path, body).expect("write config");
}

fn free_port() -> u16 {
    let listener = StdTcpListener::bind("127.0.0.1:0").expect("bind free port probe");
    listener.local_addr().expect("local addr").port()
}
