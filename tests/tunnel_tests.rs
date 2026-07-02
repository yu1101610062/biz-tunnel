use std::{
    fs,
    net::{SocketAddr, TcpListener as StdTcpListener},
    path::Path,
    time::Duration,
};

use biz_tunnel::{config::Config, runtime::Runtime};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    time::{sleep, timeout},
};

#[tokio::test]
async fn proxies_b_to_a_and_a_to_b_over_agent_initiated_tunnel() {
    let a_target = spawn_echo_server("a").await;
    let b_target = spawn_echo_server("b").await;
    let tunnel_port = free_port();
    let b_expose_port = free_port();
    let a_expose_port = free_port();
    let relay_admin_port = free_port();
    let agent_admin_port = free_port();
    let dir = tempfile::tempdir().expect("tempdir");

    let relay_config_path = dir.path().join("relay.toml");
    write_config(
        &relay_config_path,
        &format!(
            r#"
role = "relay"

[tunnel]
id = "room-a-to-room-b"
listen = "127.0.0.1:{tunnel_port}"
token = "secret-token"

[admin]
listen = "127.0.0.1:{relay_admin_port}"

[defaults]
dial_timeout_secs = 1

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
    );

    let agent_config_path = dir.path().join("agent.toml");
    write_config(
        &agent_config_path,
        &format!(
            r#"
role = "agent"

[tunnel]
id = "room-a-to-room-b"
relay_addr = "127.0.0.1:{tunnel_port}"
token = "secret-token"

[admin]
listen = "127.0.0.1:{agent_admin_port}"

[defaults]
dial_timeout_secs = 1

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
    );

    let relay = Runtime::spawn(Config::load(&relay_config_path).expect("relay config"))
        .await
        .expect("relay starts");
    let agent = Runtime::spawn(Config::load(&agent_config_path).expect("agent config"))
        .await
        .expect("agent starts");

    wait_for_tcp(("127.0.0.1", b_expose_port)).await;
    wait_for_tcp(("127.0.0.1", a_expose_port)).await;
    wait_for_tcp(("127.0.0.1", relay_admin_port)).await;
    wait_for_tcp(("127.0.0.1", agent_admin_port)).await;

    timeout(Duration::from_secs(3), async {
        loop {
            if relay.status().await.agent_connected {
                return;
            }
            sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("agent should connect");

    let relay_test = http_post(relay_admin_port, "/v1/services/test/a-echo").await;
    assert!(
        relay_test.contains(r#""status":"ok""#),
        "unexpected relay route test: {relay_test}"
    );
    let agent_test = http_post(agent_admin_port, "/v1/services/test/b-echo").await;
    assert!(
        agent_test.contains(r#""status":"ok""#),
        "unexpected agent route test: {agent_test}"
    );
    let remote_test = http_post(relay_admin_port, "/v1/services/test/b-echo").await;
    assert!(
        remote_test.contains(r#""status":"ok""#),
        "unexpected remote route test: {remote_test}"
    );

    let b_to_a_reply = round_trip(("127.0.0.1", b_expose_port), b"from-b").await;
    assert_eq!(b_to_a_reply, b"a:from-b");

    let a_to_b_reply = round_trip(("127.0.0.1", a_expose_port), b"from-a").await;
    assert_eq!(a_to_b_reply, b"b:from-a");

    let changed_target = "127.0.0.1:9";
    let routes = format!(
        r#"
[[b_to_a]]
name = "a-echo"
expose_on_relay = "127.0.0.1:{b_expose_port}"
target_from_agent = "{changed_target}"

[[a_to_b]]
name = "b-echo"
expose_on_agent = "127.0.0.1:{a_expose_port}"
target_from_relay = "{b_target}"
"#
    );
    let save = http_post_body(relay_admin_port, "/v1/configs/save", &routes).await;
    assert!(
        save.contains(r#""status":"saved""#),
        "unexpected save response: {save}"
    );
    assert!(
        fs::read_to_string(&relay_config_path)
            .expect("read relay config")
            .contains(changed_target),
        "relay config should be updated"
    );
    assert!(
        fs::read_to_string(&agent_config_path)
            .expect("read agent config")
            .contains(changed_target),
        "agent config should be updated"
    );

    agent.shutdown().await;
    relay.shutdown().await;
}

#[tokio::test]
async fn rejects_agent_with_wrong_token() {
    let a_target = spawn_echo_server("a").await;
    let tunnel_port = free_port();
    let b_expose_port = free_port();
    let relay_admin_port = free_port();
    let agent_admin_port = free_port();
    let dir = tempfile::tempdir().expect("tempdir");

    let relay_config_path = dir.path().join("relay.toml");
    write_config(
        &relay_config_path,
        &format!(
            r#"
role = "relay"

[tunnel]
id = "room-a-to-room-b"
listen = "127.0.0.1:{tunnel_port}"
token = "relay-token"

[admin]
listen = "127.0.0.1:{relay_admin_port}"

[[b_to_a]]
name = "a-echo"
expose_on_relay = "127.0.0.1:{b_expose_port}"
target_from_agent = "{a_target}"
"#
        ),
    );

    let agent_config_path = dir.path().join("agent.toml");
    write_config(
        &agent_config_path,
        &format!(
            r#"
role = "agent"

[tunnel]
id = "room-a-to-room-b"
relay_addr = "127.0.0.1:{tunnel_port}"
token = "wrong-token"

[admin]
listen = "127.0.0.1:{agent_admin_port}"

[[b_to_a]]
name = "a-echo"
expose_on_relay = "127.0.0.1:{b_expose_port}"
target_from_agent = "{a_target}"
"#
        ),
    );

    let relay = Runtime::spawn(Config::load(&relay_config_path).expect("relay config"))
        .await
        .expect("relay starts");
    let agent = Runtime::spawn(Config::load(&agent_config_path).expect("agent config"))
        .await
        .expect("agent starts");

    sleep(Duration::from_millis(150)).await;
    assert!(
        !relay.status().await.agent_connected,
        "relay should not accept an agent with the wrong token"
    );

    agent.shutdown().await;
    relay.shutdown().await;
}

#[tokio::test]
async fn reports_active_connection_details_and_service_metrics() {
    let a_target = spawn_holding_server().await;
    let tunnel_port = free_port();
    let b_expose_port = free_port();
    let relay_admin_port = free_port();
    let agent_admin_port = free_port();
    let dir = tempfile::tempdir().expect("tempdir");

    let relay_config_path = dir.path().join("relay.toml");
    write_config(
        &relay_config_path,
        &format!(
            r#"
role = "relay"

[tunnel]
id = "room-a-to-room-b"
listen = "127.0.0.1:{tunnel_port}"
token = "secret-token"

[admin]
listen = "127.0.0.1:{relay_admin_port}"

[[b_to_a]]
name = "a-hold"
expose_on_relay = "127.0.0.1:{b_expose_port}"
target_from_agent = "{a_target}"
"#
        ),
    );

    let agent_config_path = dir.path().join("agent.toml");
    write_config(
        &agent_config_path,
        &format!(
            r#"
role = "agent"

[tunnel]
id = "room-a-to-room-b"
relay_addr = "127.0.0.1:{tunnel_port}"
token = "secret-token"

[admin]
listen = "127.0.0.1:{agent_admin_port}"

[[b_to_a]]
name = "a-hold"
expose_on_relay = "127.0.0.1:{b_expose_port}"
target_from_agent = "{a_target}"
"#
        ),
    );

    let relay = Runtime::spawn(Config::load(&relay_config_path).expect("relay config"))
        .await
        .expect("relay starts");
    let agent = Runtime::spawn(Config::load(&agent_config_path).expect("agent config"))
        .await
        .expect("agent starts");

    timeout(Duration::from_secs(3), async {
        loop {
            if relay.status().await.agent_connected {
                return;
            }
            sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("agent should connect");
    let mut client = TcpStream::connect(("127.0.0.1", b_expose_port))
        .await
        .expect("connect exposed service");
    client.write_all(b"hold").await.expect("write payload");
    sleep(Duration::from_millis(100)).await;

    let connections = http_get(relay_admin_port, "/v1/connections").await;
    assert!(
        connections.contains(r#""service_name":"a-hold""#)
            && connections.contains(r#""direction":"b_to_a""#)
            && connections.contains(r#""source_addr":"127.0.0.1"#),
        "unexpected connection list: {connections}"
    );

    let metrics = http_get(relay_admin_port, "/metrics").await;
    assert!(
        metrics.contains(
            "biz_tunnel_service_active_streams{service=\"a-hold\",direction=\"b_to_a\"} 1"
        ),
        "unexpected metrics: {metrics}"
    );

    drop(client);
    agent.shutdown().await;
    relay.shutdown().await;
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
                let mut buf = vec![0_u8; 1024];
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

async fn spawn_holding_server() -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind holding server");
    let addr = listener.local_addr().expect("holding local addr");
    tokio::spawn(async move {
        loop {
            let Ok((mut socket, _)) = listener.accept().await else {
                break;
            };
            tokio::spawn(async move {
                let mut buf = vec![0_u8; 1024];
                let _ = socket.read(&mut buf).await;
                sleep(Duration::from_secs(2)).await;
            });
        }
    });
    addr
}

async fn http_get(port: u16, path: &str) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port))
        .await
        .expect("connect admin");
    let request = format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n");
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

async fn http_post(port: u16, path: &str) -> String {
    http_post_body(port, path, "").await
}

async fn http_post_body(port: u16, path: &str, body: &str) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port))
        .await
        .expect("connect admin");
    let request = format!(
        "POST {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
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
