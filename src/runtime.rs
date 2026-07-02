use std::{
    collections::HashMap,
    fmt,
    net::{IpAddr, SocketAddr},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use quinn::{Connection, Endpoint, RecvStream, SendStream};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::{Mutex, Notify, RwLock, mpsc},
    task::JoinHandle,
    time::{sleep, timeout},
};

use crate::{
    admin_ui,
    config::{Config, Direction, Role, Service, TransportMode},
    quic as quic_transport,
};

const FRAME_HELLO: u8 = 1;
const FRAME_HELLO_OK: u8 = 2;
const FRAME_OPEN: u8 = 3;
const FRAME_DATA: u8 = 4;
const FRAME_CLOSE: u8 = 5;
const FRAME_OPEN_ERROR: u8 = 6;
const FRAME_FIN: u8 = 7;
const MAX_FRAME_LEN: usize = 1024 * 1024;
const RETRY_DELAY: Duration = Duration::from_millis(100);
const TUNNEL_WAIT: Duration = Duration::from_secs(3);

#[derive(Clone)]
pub struct Runtime {
    state: Arc<SharedState>,
    handles: Arc<Mutex<Vec<JoinHandle<()>>>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeStatus {
    pub agent_connected: bool,
    pub active_streams: usize,
}

impl Runtime {
    pub async fn spawn(config: Config) -> Result<Self, RuntimeError> {
        let state = Arc::new(SharedState::new(config));
        let handles = Arc::new(Mutex::new(Vec::new()));
        let runtime = Self {
            state: state.clone(),
            handles: handles.clone(),
        };

        runtime.spawn_admin().await?;
        let role = state.config.read().await.role;
        match role {
            Role::Relay => runtime.spawn_relay().await?,
            Role::Agent => runtime.spawn_agent().await?,
        }

        Ok(runtime)
    }

    pub async fn status(&self) -> RuntimeStatus {
        RuntimeStatus {
            agent_connected: self.state.connected.load(Ordering::SeqCst),
            active_streams: self.state.streams.lock().await.len(),
        }
    }

    pub async fn shutdown(&self) {
        self.state.shutting_down.store(true, Ordering::SeqCst);
        self.state.abort_service_listeners().await;
        let mut handles = self.handles.lock().await;
        for handle in handles.drain(..) {
            handle.abort();
        }
    }

    async fn spawn_relay(&self) -> Result<(), RuntimeError> {
        let config = self.state.config.read().await.clone();
        match config.transport.mode {
            TransportMode::Quic => {
                let endpoint = quic_transport::server_endpoint(&config)
                    .map_err(|error| RuntimeError::Config(error.to_string()))?;
                self.push_task(spawn_quic_accept_loop(self.state.clone(), endpoint))
                    .await;
            }
            TransportMode::Tcp | TransportMode::TlsTcp => {
                let listen =
                    config.tunnel.listen.as_deref().ok_or_else(|| {
                        RuntimeError::Config("relay missing tunnel.listen".into())
                    })?;
                let tunnel_listener = TcpListener::bind(listen).await?;
                self.push_task(spawn_tunnel_accept_loop(
                    self.state.clone(),
                    tunnel_listener,
                ))
                .await;
            }
        }

        for service in listen_services(config.role, config.services()) {
            self.state.start_service_listener(service).await?;
        }

        Ok(())
    }

    async fn spawn_agent(&self) -> Result<(), RuntimeError> {
        let config = self.state.config.read().await.clone();
        for service in listen_services(config.role, config.services()) {
            self.state.start_service_listener(service).await?;
        }

        self.push_task(spawn_agent_connect_loop(self.state.clone()))
            .await;
        Ok(())
    }

    async fn push_task(&self, handle: JoinHandle<()>) {
        self.handles.lock().await.push(handle);
    }

    async fn spawn_admin(&self) -> Result<(), RuntimeError> {
        let listen = self.state.config.read().await.admin.listen.clone();
        let listener = TcpListener::bind(&listen).await?;
        self.push_task(spawn_admin_loop(self.state.clone(), listener))
            .await;
        Ok(())
    }
}

struct SharedState {
    config: RwLock<Config>,
    writer: RwLock<Option<mpsc::Sender<Frame>>>,
    quic_connection: RwLock<Option<Connection>>,
    streams: Mutex<HashMap<u64, mpsc::Sender<StreamEvent>>>,
    connections: Mutex<HashMap<u64, ConnectionInfo>>,
    service_handles: Mutex<HashMap<String, JoinHandle<()>>>,
    generation: AtomicU64,
    next_stream_id: AtomicU64,
    connected: AtomicBool,
    shutting_down: AtomicBool,
    connected_notify: Notify,
    counters: Counters,
}

impl SharedState {
    fn new(config: Config) -> Self {
        let first_stream = match config.role {
            Role::Relay => 1,
            Role::Agent => 2,
        };
        Self {
            config: RwLock::new(config),
            writer: RwLock::new(None),
            quic_connection: RwLock::new(None),
            streams: Mutex::new(HashMap::new()),
            connections: Mutex::new(HashMap::new()),
            service_handles: Mutex::new(HashMap::new()),
            generation: AtomicU64::new(1),
            next_stream_id: AtomicU64::new(first_stream),
            connected: AtomicBool::new(false),
            shutting_down: AtomicBool::new(false),
            connected_notify: Notify::new(),
            counters: Counters::default(),
        }
    }

    fn allocate_stream_id(&self) -> u64 {
        self.next_stream_id.fetch_add(2, Ordering::SeqCst)
    }

    async fn send_frame(&self, frame: Frame) -> Result<(), RuntimeError> {
        let writer = self.writer.read().await.clone();
        let Some(writer) = writer else {
            return Err(RuntimeError::TunnelUnavailable);
        };
        writer
            .send(frame)
            .await
            .map_err(|_| RuntimeError::TunnelUnavailable)
    }

    async fn wait_for_tunnel(&self) -> bool {
        if self.connected.load(Ordering::SeqCst) {
            return true;
        }

        tokio::select! {
            _ = self.connected_notify.notified() => self.connected.load(Ordering::SeqCst),
            _ = sleep(TUNNEL_WAIT) => false,
        }
    }

    async fn mark_tcp_connected(&self, sender: mpsc::Sender<Frame>) {
        *self.writer.write().await = Some(sender);
        self.connected.store(true, Ordering::SeqCst);
        self.connected_notify.notify_waiters();
        eprintln!("event=tunnel_connected");
    }

    async fn mark_quic_connected(&self, connection: Connection) {
        *self.quic_connection.write().await = Some(connection);
        self.connected.store(true, Ordering::SeqCst);
        self.connected_notify.notify_waiters();
        eprintln!("event=tunnel_connected transport=quic");
    }

    async fn mark_disconnected(&self) {
        *self.writer.write().await = None;
        *self.quic_connection.write().await = None;
        self.connected.store(false, Ordering::SeqCst);
        let mut streams = self.streams.lock().await;
        for (_, sender) in streams.drain() {
            let _ = sender.send(StreamEvent::Close).await;
        }
        self.connections.lock().await.clear();
        eprintln!("event=tunnel_disconnected");
    }

    async fn start_service_listener(
        self: &Arc<Self>,
        service: Service,
    ) -> Result<(), RuntimeError> {
        let listener = TcpListener::bind(service.expose_addr()).await?;
        let name = service.name.clone();
        let handle = spawn_expose_loop(self.clone(), service, listener);
        self.service_handles.lock().await.insert(name, handle);
        Ok(())
    }

    async fn stop_service_listener(&self, name: &str) {
        if let Some(handle) = self.service_handles.lock().await.remove(name) {
            handle.abort();
        }
    }

    async fn abort_service_listeners(&self) {
        let mut handles = self.service_handles.lock().await;
        for (_, handle) in handles.drain() {
            handle.abort();
        }
    }

    async fn reload_services(self: &Arc<Self>) -> Result<ReloadReport, RuntimeError> {
        let current = self.config.read().await.clone();
        let path = current
            .source_path()
            .ok_or_else(|| RuntimeError::Config("config source path is unavailable".into()))?
            .to_path_buf();
        let next = Config::load(&path).map_err(|error| RuntimeError::Config(error.to_string()))?;
        ensure_reload_compatible(&current, &next)?;

        let role = current.role;
        let current_services = service_map_for_role(role, current.services());
        let next_services = service_map_for_role(role, next.services());
        let mut report = ReloadReport {
            generation: self.generation.load(Ordering::SeqCst) + 1,
            added: Vec::new(),
            removed: Vec::new(),
            changed: Vec::new(),
            unchanged: Vec::new(),
        };

        for name in current_services.keys() {
            if !next_services.contains_key(name) {
                self.stop_service_listener(name).await;
                report.removed.push(name.clone());
            }
        }

        for (name, next_service) in &next_services {
            match current_services.get(name) {
                None => {
                    self.start_service_listener(next_service.clone()).await?;
                    report.added.push(name.clone());
                }
                Some(current_service)
                    if current_service.expose_addr() != next_service.expose_addr() =>
                {
                    self.stop_service_listener(name).await;
                    self.start_service_listener(next_service.clone()).await?;
                    report.changed.push(name.clone());
                }
                Some(current_service) if current_service != next_service => {
                    report.changed.push(name.clone());
                }
                Some(_) => report.unchanged.push(name.clone()),
            }
        }

        *self.config.write().await = next;
        self.generation.store(report.generation, Ordering::SeqCst);
        self.counters.reload_success.fetch_add(1, Ordering::SeqCst);
        eprintln!("event=reload_applied generation={}", report.generation);
        Ok(report)
    }

    async fn register_connection(&self, info: ConnectionInfo) {
        self.counters.accepted.fetch_add(1, Ordering::SeqCst);
        eprintln!(
            "event=connection_opened connection_id={} service={} direction={}",
            info.id,
            info.service_name,
            direction_label(info.direction)
        );
        self.connections.lock().await.insert(info.id, info);
    }

    async fn add_bytes(&self, stream_id: u64, from_local: u64, to_local: u64) {
        if let Some(info) = self.connections.lock().await.get_mut(&stream_id) {
            info.bytes_from_local += from_local;
            info.bytes_to_local += to_local;
        }
    }

    async fn close_connection(&self, stream_id: u64, reason: &str) {
        if let Some(info) = self.connections.lock().await.remove(&stream_id) {
            eprintln!(
                "event=connection_closed connection_id={} service={} reason={} bytes_from_local={} bytes_to_local={}",
                info.id, info.service_name, reason, info.bytes_from_local, info.bytes_to_local
            );
        }
    }
}

#[derive(Default)]
struct Counters {
    accepted: AtomicU64,
    dial_failed: AtomicU64,
    auth_failed: AtomicU64,
    reload_success: AtomicU64,
    reload_failure: AtomicU64,
}

#[derive(Debug, Clone)]
struct ConnectionInfo {
    id: u64,
    service_name: String,
    direction: Direction,
    source_addr: String,
    target_addr: String,
    opened_at_unix_ms: u64,
    bytes_from_local: u64,
    bytes_to_local: u64,
}

#[derive(Debug)]
struct ReloadReport {
    generation: u64,
    added: Vec<String>,
    removed: Vec<String>,
    changed: Vec<String>,
    unchanged: Vec<String>,
}

fn spawn_tunnel_accept_loop(state: Arc<SharedState>, listener: TcpListener) -> JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            let Ok((socket, _)) = listener.accept().await else {
                break;
            };
            let state = state.clone();
            tokio::spawn(async move {
                if let Err(error) = handle_relay_tunnel_socket(state.clone(), socket).await {
                    eprintln!("event=tunnel_error side=relay error={error}");
                    state.mark_disconnected().await;
                }
            });
        }
    })
}

fn spawn_quic_accept_loop(state: Arc<SharedState>, endpoint: Endpoint) -> JoinHandle<()> {
    tokio::spawn(async move {
        while let Some(incoming) = endpoint.accept().await {
            let state = state.clone();
            tokio::spawn(async move {
                match incoming.await {
                    Ok(connection) => {
                        if let Err(error) =
                            handle_quic_relay_connection(state.clone(), connection).await
                        {
                            eprintln!("event=quic_tunnel_error side=relay error={error}");
                            state.mark_disconnected().await;
                        }
                    }
                    Err(error) => eprintln!("event=quic_accept_failed error={error}"),
                }
            });
        }
    })
}

async fn connect_quic_agent(state: Arc<SharedState>, config: Config) -> Result<(), RuntimeError> {
    let relay_addr = config
        .tunnel
        .relay_addr
        .as_deref()
        .ok_or_else(|| RuntimeError::Config("agent missing tunnel.relay_addr".into()))?
        .parse::<SocketAddr>()
        .map_err(|error| RuntimeError::Config(format!("invalid tunnel.relay_addr: {error}")))?;
    let endpoint = quic_transport::client_endpoint(&config)
        .map_err(|error| RuntimeError::Config(error.to_string()))?;
    let server_name = quic_transport::server_name(&config)
        .map_err(|error| RuntimeError::Config(error.to_string()))?;
    let connection = endpoint
        .connect(relay_addr, server_name)
        .map_err(quic_error)?
        .await
        .map_err(quic_error)?;
    if let Err(error) = quic_transport::verify_expected_peer_fingerprint(&connection, &config) {
        connection.close(quic_transport::CLOSE_CODE, b"peer fingerprint mismatch");
        return Err(RuntimeError::AuthenticationFailedWithReason(
            error.to_string(),
        ));
    }

    let (mut send, mut recv) = connection.open_bi().await.map_err(quic_error)?;
    quic_transport::write_stream_message(
        &mut send,
        quic_transport::STREAM_CONTROL,
        hello_payload(&config).as_bytes(),
    )
    .await
    .map_err(quic_error)?;
    let _ = send.finish();
    let (kind, _) = quic_transport::read_stream_message(&mut recv)
        .await
        .map_err(quic_error)?;
    if kind != FRAME_HELLO_OK {
        connection.close(quic_transport::CLOSE_CODE, b"authentication failed");
        return Err(RuntimeError::AuthenticationFailed);
    }

    state.mark_quic_connected(connection.clone()).await;
    run_quic_accept_loop(state, connection).await
}

async fn handle_quic_relay_connection(
    state: Arc<SharedState>,
    connection: Connection,
) -> Result<(), RuntimeError> {
    let config = state.config.read().await.clone();
    if let Err(error) = quic_transport::verify_expected_peer_fingerprint(&connection, &config) {
        connection.close(quic_transport::CLOSE_CODE, b"peer fingerprint mismatch");
        return Err(RuntimeError::AuthenticationFailedWithReason(
            error.to_string(),
        ));
    }

    let (mut send, mut recv) = connection.accept_bi().await.map_err(quic_error)?;
    let (kind, payload) = quic_transport::read_stream_message(&mut recv)
        .await
        .map_err(quic_error)?;
    if kind != quic_transport::STREAM_CONTROL {
        connection.close(quic_transport::CLOSE_CODE, b"expected control stream");
        return Err(RuntimeError::Protocol(
            "expected QUIC control stream".into(),
        ));
    }
    let hello = String::from_utf8(payload)
        .map_err(|_| RuntimeError::Protocol("QUIC HELLO must be UTF-8".into()))?;
    if !hello_matches_config(&hello, &config) {
        state.counters.auth_failed.fetch_add(1, Ordering::SeqCst);
        connection.close(quic_transport::CLOSE_CODE, b"authentication failed");
        return Err(RuntimeError::AuthenticationFailed);
    }
    quic_transport::write_stream_message(&mut send, FRAME_HELLO_OK, &[])
        .await
        .map_err(quic_error)?;
    let _ = send.finish();

    state.mark_quic_connected(connection.clone()).await;
    run_quic_accept_loop(state, connection).await
}

async fn run_quic_accept_loop(
    state: Arc<SharedState>,
    connection: Connection,
) -> Result<(), RuntimeError> {
    loop {
        match connection.accept_bi().await {
            Ok((send, recv)) => {
                let state = state.clone();
                tokio::spawn(async move {
                    if let Err(error) = handle_quic_bi_stream(state, send, recv).await {
                        eprintln!("event=quic_stream_error error={error}");
                    }
                });
            }
            Err(error) => return Err(quic_error(error)),
        }
    }
}

async fn handle_quic_bi_stream(
    state: Arc<SharedState>,
    send: SendStream,
    mut recv: RecvStream,
) -> Result<(), RuntimeError> {
    let (kind, payload) = quic_transport::read_stream_message(&mut recv)
        .await
        .map_err(quic_error)?;
    match kind {
        quic_transport::STREAM_OPEN => handle_quic_open_stream(state, send, recv, payload).await,
        other => Err(RuntimeError::Protocol(format!(
            "unexpected QUIC stream type {other}"
        ))),
    }
}

fn spawn_agent_connect_loop(state: Arc<SharedState>) -> JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            if state.shutting_down.load(Ordering::SeqCst) {
                return;
            }

            let config = state.config.read().await.clone();
            if config.transport.mode == TransportMode::Quic {
                match connect_quic_agent(state.clone(), config).await {
                    Ok(()) => {}
                    Err(error) => eprintln!("event=quic_tunnel_error side=agent error={error}"),
                }
                state.mark_disconnected().await;
                sleep(RETRY_DELAY).await;
                continue;
            }

            let relay_addr = config.tunnel.relay_addr.clone();
            let Some(relay_addr) = relay_addr else {
                return;
            };
            match TcpStream::connect(relay_addr).await {
                Ok(socket) => {
                    if let Err(error) = handle_agent_tunnel_socket(state.clone(), socket).await {
                        eprintln!("event=tunnel_error side=agent error={error}");
                    }
                    state.mark_disconnected().await;
                }
                Err(error) => {
                    eprintln!("event=tunnel_connect_failed error={error}");
                }
            }

            sleep(RETRY_DELAY).await;
        }
    })
}

fn spawn_expose_loop(
    state: Arc<SharedState>,
    service: Service,
    listener: TcpListener,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            let Ok((socket, peer)) = listener.accept().await else {
                break;
            };
            let state = state.clone();
            let service = service.clone();
            tokio::spawn(async move {
                if let Err(error) = handle_local_connection(state, service, socket, peer).await {
                    eprintln!("event=local_connection_closed error={error}");
                }
            });
        }
    })
}

fn spawn_admin_loop(state: Arc<SharedState>, listener: TcpListener) -> JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            let Ok((socket, peer)) = listener.accept().await else {
                break;
            };
            let state = state.clone();
            tokio::spawn(async move {
                if let Err(error) = handle_admin_connection(state, socket, peer).await {
                    eprintln!("event=admin_connection_closed error={error}");
                }
            });
        }
    })
}

async fn handle_admin_connection(
    state: Arc<SharedState>,
    mut socket: TcpStream,
    peer: SocketAddr,
) -> Result<(), RuntimeError> {
    let mut buf = vec![0_u8; 8192];
    let n = socket.read(&mut buf).await?;
    if n == 0 {
        return Ok(());
    }
    let request = String::from_utf8_lossy(&buf[..n]);
    let first_line = request.lines().next().unwrap_or_default();
    let mut parts = first_line.split_whitespace();
    let method = parts.next().unwrap_or_default();
    let path = parts.next().unwrap_or_default();
    let path = path.split('?').next().unwrap_or(path);
    let public_path = matches!(path, "/healthz" | "/readyz" | "/" | "/ui");
    let admin_token = state
        .config
        .read()
        .await
        .admin_token()
        .map(ToString::to_string);

    if !public_path && !admin_authorized(&request, admin_token.as_deref(), peer) {
        write_http(
            &mut socket,
            "401 Unauthorized",
            "application/json",
            "{\"error\":\"unauthorized\"}\n",
        )
        .await?;
        return Ok(());
    }

    let response = match (method, path) {
        ("GET", "/") | ("GET", "/ui") => HttpResponse {
            status: "200 OK",
            content_type: "text/html; charset=utf-8",
            body: admin_ui::html().to_string(),
        },
        ("GET", "/healthz") | ("GET", "/readyz") => {
            let role = state.config.read().await.role;
            HttpResponse::json(
                "200 OK",
                format!("{{\"status\":\"ok\",\"role\":\"{}\"}}\n", role_label(role)),
            )
        }
        ("GET", "/v1/services") => {
            let config = state.config.read().await.clone();
            HttpResponse::json("200 OK", services_json(&config))
        }
        ("GET", "/v1/topology") => {
            let config = state.config.read().await.clone();
            HttpResponse::json("200 OK", topology_json(&config))
        }
        ("GET", "/v1/tunnel") => {
            let config = state.config.read().await.clone();
            let connected = state.connected.load(Ordering::SeqCst);
            HttpResponse::json(
                "200 OK",
                format!(
                    "{{\"id\":\"{}\",\"role\":\"{}\",\"agent_connected\":{},\"generation\":{}}}\n",
                    json_escape(&config.tunnel.id),
                    role_label(config.role),
                    connected,
                    state.generation.load(Ordering::SeqCst)
                ),
            )
        }
        ("GET", "/v1/connections") => {
            let connections = state.connections.lock().await;
            let body = connections_json(&connections);
            HttpResponse::json("200 OK", body)
        }
        ("GET", path) if path.starts_with("/v1/connections/") => {
            let id = path.trim_start_matches("/v1/connections/");
            let connections = state.connections.lock().await;
            let body = connection_json(&connections, id);
            match body {
                Some(body) => HttpResponse::json("200 OK", body),
                None => HttpResponse::json("404 Not Found", "{\"error\":\"not found\"}\n".into()),
            }
        }
        ("POST", path) if path.starts_with("/v1/services/test/") => {
            let name = path.trim_start_matches("/v1/services/test/");
            HttpResponse::json("200 OK", service_test_json(&state, name).await)
        }
        ("GET", "/metrics") => {
            let config = state.config.read().await.clone();
            let connections = state.connections.lock().await;
            HttpResponse {
                status: "200 OK",
                content_type: "text/plain; version=0.0.4",
                body: metrics_text(&state, &config, &connections),
            }
        }
        ("POST", "/v1/services/reload") => match state.reload_services().await {
            Ok(report) => HttpResponse::json("200 OK", reload_report_json("applied", &report)),
            Err(error) => {
                state.counters.reload_failure.fetch_add(1, Ordering::SeqCst);
                eprintln!("event=reload_rejected error={error}");
                HttpResponse::json(
                    "422 Unprocessable Entity",
                    format!(
                        "{{\"status\":\"rejected\",\"generation\":{},\"errors\":[\"{}\"]}}\n",
                        state.generation.load(Ordering::SeqCst),
                        json_escape(&error.to_string())
                    ),
                )
            }
        },
        _ => HttpResponse::json("404 Not Found", "{\"error\":\"not found\"}\n".into()),
    };

    write_http(
        &mut socket,
        response.status,
        response.content_type,
        &response.body,
    )
    .await?;
    Ok(())
}

async fn service_test_json(state: &Arc<SharedState>, name: &str) -> String {
    let config = state.config.read().await.clone();
    let Some(service) = config.service_by_name(name).cloned() else {
        if let Some(service) = topology_configs(&config)
            .iter()
            .find_map(|config| config.service_by_name(name).cloned())
        {
            return service_test_result_json(
                &service,
                "skipped",
                &format!(
                    "该路径监听在 {} 侧，请到对应页面测试",
                    route_owner_label(service.direction)
                ),
            );
        }
        return format!(
            "{{\"status\":\"not_found\",\"service\":\"{}\",\"message\":\"未找到路径\"}}\n",
            json_escape(name)
        );
    };

    if !should_listen(config.role, service.direction) {
        return format!(
            "{{\"status\":\"skipped\",\"service\":\"{}\",\"direction\":\"{}\",\"message\":\"该路径监听在 {} 侧，请到对应页面测试\"}}\n",
            json_escape(&service.name),
            direction_label(service.direction),
            route_owner_label(service.direction)
        );
    }
    if !state.wait_for_tunnel().await {
        return service_test_result_json(&service, "failed", "隧道未连接");
    }

    let result = match config.transport.mode {
        TransportMode::Quic => test_quic_service(state, &config, &service).await,
        TransportMode::Tcp | TransportMode::TlsTcp => {
            test_tcp_service(state, &config, &service).await
        }
    };
    match result {
        Ok(()) => service_test_result_json(&service, "ok", "已通过隧道拨通对端目标 TCP 端口"),
        Err(error) => service_test_result_json(&service, "failed", &error.to_string()),
    }
}

async fn test_tcp_service(
    state: &Arc<SharedState>,
    config: &Config,
    service: &Service,
) -> Result<(), RuntimeError> {
    let stream_id = state.allocate_stream_id();
    let (events_tx, mut events_rx) = mpsc::channel(1);
    state.streams.lock().await.insert(stream_id, events_tx);
    state
        .send_frame(Frame {
            kind: FRAME_OPEN,
            stream_id,
            payload: encode_open_payload(service, test_source_addr(), unix_ms()),
        })
        .await?;

    let wait = service_test_wait(config, service);
    let result = match timeout(wait, events_rx.recv()).await {
        Ok(Some(StreamEvent::Close)) | Ok(None) => {
            Err(RuntimeError::Config("对端目标 TCP 端口不可达".into()))
        }
        Ok(Some(StreamEvent::Data(_))) | Ok(Some(StreamEvent::FinishWrite)) | Err(_) => Ok(()),
    };
    let _ = state
        .send_frame(Frame {
            kind: FRAME_CLOSE,
            stream_id,
            payload: Vec::new(),
        })
        .await;
    state.streams.lock().await.remove(&stream_id);
    result
}

async fn test_quic_service(
    state: &Arc<SharedState>,
    config: &Config,
    service: &Service,
) -> Result<(), RuntimeError> {
    let connection = state.quic_connection.read().await.clone();
    let Some(connection) = connection else {
        return Err(RuntimeError::TunnelUnavailable);
    };
    let (mut send, mut recv) = connection.open_bi().await.map_err(quic_error)?;
    quic_transport::write_stream_message(
        &mut send,
        quic_transport::STREAM_OPEN,
        &encode_open_payload(service, test_source_addr(), unix_ms()),
    )
    .await
    .map_err(quic_error)?;
    let _ = send.finish();

    let mut buf = [0_u8; 1];
    match timeout(service_test_wait(config, service), recv.read(&mut buf)).await {
        Ok(Err(error)) => Err(quic_error(error)),
        Ok(Ok(_)) | Err(_) => Ok(()),
    }
}

fn service_test_wait(config: &Config, service: &Service) -> Duration {
    Duration::from_secs(
        service
            .dial_timeout_secs()
            .unwrap_or(config.defaults.dial_timeout_secs),
    ) + Duration::from_millis(250)
}

fn test_source_addr() -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], 0))
}

fn service_test_result_json(service: &Service, status: &str, message: &str) -> String {
    format!(
        "{{\"status\":\"{}\",\"service\":\"{}\",\"direction\":\"{}\",\"message\":\"{}\"}}\n",
        status,
        json_escape(&service.name),
        direction_label(service.direction),
        json_escape(message)
    )
}

async fn handle_relay_tunnel_socket(
    state: Arc<SharedState>,
    mut socket: TcpStream,
) -> Result<(), RuntimeError> {
    let hello = read_frame(&mut socket).await?;
    if hello.kind != FRAME_HELLO {
        return Err(RuntimeError::Protocol("expected HELLO".into()));
    }
    let payload = String::from_utf8(hello.payload)
        .map_err(|_| RuntimeError::Protocol("HELLO must be UTF-8".into()))?;
    let config = state.config.read().await.clone();
    if !hello_matches_config(&payload, &config) {
        state.counters.auth_failed.fetch_add(1, Ordering::SeqCst);
        return Err(RuntimeError::AuthenticationFailed);
    }
    write_frame(
        &mut socket,
        &Frame {
            kind: FRAME_HELLO_OK,
            stream_id: 0,
            payload: Vec::new(),
        },
    )
    .await?;
    run_tunnel(state, socket).await
}

async fn handle_agent_tunnel_socket(
    state: Arc<SharedState>,
    mut socket: TcpStream,
) -> Result<(), RuntimeError> {
    let config = state.config.read().await.clone();
    let payload = hello_payload(&config);
    write_frame(
        &mut socket,
        &Frame {
            kind: FRAME_HELLO,
            stream_id: 0,
            payload: payload.into_bytes(),
        },
    )
    .await?;
    let response = read_frame(&mut socket).await?;
    if response.kind != FRAME_HELLO_OK {
        return Err(RuntimeError::AuthenticationFailed);
    }
    run_tunnel(state, socket).await
}

async fn run_tunnel(state: Arc<SharedState>, socket: TcpStream) -> Result<(), RuntimeError> {
    let (mut reader, mut writer) = socket.into_split();
    let (frame_tx, mut frame_rx) = mpsc::channel::<Frame>(1024);
    state.mark_tcp_connected(frame_tx).await;

    let write_task = tokio::spawn(async move {
        while let Some(frame) = frame_rx.recv().await {
            if write_frame(&mut writer, &frame).await.is_err() {
                break;
            }
        }
    });

    let read_result = loop {
        match read_frame(&mut reader).await {
            Ok(frame) => handle_incoming_frame(state.clone(), frame).await?,
            Err(error) => break Err(error),
        }
    };

    write_task.abort();
    state.mark_disconnected().await;
    read_result
}

async fn handle_local_connection(
    state: Arc<SharedState>,
    service: Service,
    socket: TcpStream,
    peer: SocketAddr,
) -> Result<(), RuntimeError> {
    if !source_allowed(service.allowed_sources(), peer.ip()) {
        return Err(RuntimeError::SourceNotAllowed(peer));
    }
    if !state.wait_for_tunnel().await {
        return Err(RuntimeError::TunnelUnavailable);
    }

    let transport = state.config.read().await.transport.mode;
    if transport == TransportMode::Quic {
        return handle_quic_local_connection(state, service, socket, peer).await;
    }

    let stream_id = state.allocate_stream_id();
    let opened_at = unix_ms();
    let (events_tx, events_rx) = mpsc::channel(128);
    state.streams.lock().await.insert(stream_id, events_tx);
    state
        .register_connection(ConnectionInfo {
            id: stream_id,
            service_name: service.name.clone(),
            direction: service.direction,
            source_addr: peer.to_string(),
            target_addr: service.target_addr().to_string(),
            opened_at_unix_ms: opened_at,
            bytes_from_local: 0,
            bytes_to_local: 0,
        })
        .await;
    state
        .send_frame(Frame {
            kind: FRAME_OPEN,
            stream_id,
            payload: encode_open_payload(&service, peer, opened_at),
        })
        .await?;
    bridge_socket(state, stream_id, socket, events_rx).await;
    Ok(())
}

async fn handle_quic_local_connection(
    state: Arc<SharedState>,
    service: Service,
    socket: TcpStream,
    peer: SocketAddr,
) -> Result<(), RuntimeError> {
    let connection = state.quic_connection.read().await.clone();
    let Some(connection) = connection else {
        return Err(RuntimeError::TunnelUnavailable);
    };

    let stream_id = state.allocate_stream_id();
    let opened_at = unix_ms();
    let (mut send, recv) = connection.open_bi().await.map_err(quic_error)?;
    quic_transport::write_stream_message(
        &mut send,
        quic_transport::STREAM_OPEN,
        &encode_open_payload(&service, peer, opened_at),
    )
    .await
    .map_err(quic_error)?;

    state
        .register_connection(ConnectionInfo {
            id: stream_id,
            service_name: service.name.clone(),
            direction: service.direction,
            source_addr: peer.to_string(),
            target_addr: service.target_addr().to_string(),
            opened_at_unix_ms: opened_at,
            bytes_from_local: 0,
            bytes_to_local: 0,
        })
        .await;

    bridge_quic_stream(state, stream_id, socket, send, recv).await;
    Ok(())
}

async fn handle_incoming_frame(state: Arc<SharedState>, frame: Frame) -> Result<(), RuntimeError> {
    match frame.kind {
        FRAME_OPEN => handle_open_frame(state, frame).await,
        FRAME_DATA => {
            if let Some(sender) = state.streams.lock().await.get(&frame.stream_id).cloned() {
                let _ = sender.send(StreamEvent::Data(frame.payload)).await;
            }
            Ok(())
        }
        FRAME_CLOSE | FRAME_OPEN_ERROR => {
            if let Some(sender) = state.streams.lock().await.remove(&frame.stream_id) {
                let _ = sender.send(StreamEvent::Close).await;
            }
            state
                .close_connection(frame.stream_id, "remote_close")
                .await;
            Ok(())
        }
        FRAME_FIN => {
            if let Some(sender) = state.streams.lock().await.get(&frame.stream_id).cloned() {
                let _ = sender.send(StreamEvent::FinishWrite).await;
            }
            Ok(())
        }
        other => Err(RuntimeError::Protocol(format!(
            "unknown frame type {other}"
        ))),
    }
}

async fn handle_open_frame(state: Arc<SharedState>, frame: Frame) -> Result<(), RuntimeError> {
    let open = decode_open_payload(&frame.payload)?;
    let config = state.config.read().await.clone();
    let Some(service) = config.service_by_name(&open.service_name).cloned() else {
        state
            .send_frame(Frame {
                kind: FRAME_OPEN_ERROR,
                stream_id: frame.stream_id,
                payload: b"unknown service".to_vec(),
            })
            .await?;
        return Ok(());
    };

    let expected_direction = match config.role {
        Role::Relay => Direction::AToB,
        Role::Agent => Direction::BToA,
    };
    if service.direction != expected_direction {
        state
            .send_frame(Frame {
                kind: FRAME_OPEN_ERROR,
                stream_id: frame.stream_id,
                payload: b"wrong service direction".to_vec(),
            })
            .await?;
        return Ok(());
    }

    let dial_timeout = Duration::from_secs(
        service
            .dial_timeout_secs()
            .unwrap_or(config.defaults.dial_timeout_secs),
    );
    match timeout(dial_timeout, TcpStream::connect(service.target_addr())).await {
        Ok(Ok(socket)) => {
            let (events_tx, events_rx) = mpsc::channel(128);
            state
                .streams
                .lock()
                .await
                .insert(frame.stream_id, events_tx);
            state
                .register_connection(ConnectionInfo {
                    id: frame.stream_id,
                    service_name: service.name.clone(),
                    direction: service.direction,
                    source_addr: open.source_addr,
                    target_addr: service.target_addr().to_string(),
                    opened_at_unix_ms: open.opened_at_unix_ms,
                    bytes_from_local: 0,
                    bytes_to_local: 0,
                })
                .await;
            tokio::spawn(bridge_socket(state, frame.stream_id, socket, events_rx));
        }
        Ok(Err(error)) => {
            state.counters.dial_failed.fetch_add(1, Ordering::SeqCst);
            state
                .send_frame(Frame {
                    kind: FRAME_OPEN_ERROR,
                    stream_id: frame.stream_id,
                    payload: error.to_string().into_bytes(),
                })
                .await?;
        }
        Err(_) => {
            state.counters.dial_failed.fetch_add(1, Ordering::SeqCst);
            state
                .send_frame(Frame {
                    kind: FRAME_OPEN_ERROR,
                    stream_id: frame.stream_id,
                    payload: b"dial timeout".to_vec(),
                })
                .await?;
        }
    }
    Ok(())
}

async fn handle_quic_open_stream(
    state: Arc<SharedState>,
    mut send: SendStream,
    recv: RecvStream,
    payload: Vec<u8>,
) -> Result<(), RuntimeError> {
    let open = decode_open_payload(&payload)?;
    let config = state.config.read().await.clone();
    let Some(service) = config.service_by_name(&open.service_name).cloned() else {
        let _ = send.reset(quic_transport::CLOSE_CODE);
        return Ok(());
    };

    let expected_direction = match config.role {
        Role::Relay => Direction::AToB,
        Role::Agent => Direction::BToA,
    };
    if service.direction != expected_direction {
        let _ = send.reset(quic_transport::CLOSE_CODE);
        return Ok(());
    }

    let dial_timeout = Duration::from_secs(
        service
            .dial_timeout_secs()
            .unwrap_or(config.defaults.dial_timeout_secs),
    );
    match timeout(dial_timeout, TcpStream::connect(service.target_addr())).await {
        Ok(Ok(socket)) => {
            let stream_id = state.allocate_stream_id();
            state
                .register_connection(ConnectionInfo {
                    id: stream_id,
                    service_name: service.name.clone(),
                    direction: service.direction,
                    source_addr: open.source_addr,
                    target_addr: service.target_addr().to_string(),
                    opened_at_unix_ms: open.opened_at_unix_ms,
                    bytes_from_local: 0,
                    bytes_to_local: 0,
                })
                .await;
            tokio::spawn(bridge_quic_stream(state, stream_id, socket, send, recv));
        }
        Ok(Err(error)) => {
            state.counters.dial_failed.fetch_add(1, Ordering::SeqCst);
            eprintln!(
                "event=quic_dial_failed service={} error={error}",
                service.name
            );
            let _ = send.reset(quic_transport::CLOSE_CODE);
        }
        Err(_) => {
            state.counters.dial_failed.fetch_add(1, Ordering::SeqCst);
            eprintln!(
                "event=quic_dial_failed service={} error=timeout",
                service.name
            );
            let _ = send.reset(quic_transport::CLOSE_CODE);
        }
    }
    Ok(())
}

async fn bridge_socket(
    state: Arc<SharedState>,
    stream_id: u64,
    socket: TcpStream,
    mut events_rx: mpsc::Receiver<StreamEvent>,
) {
    let (mut reader, mut writer) = socket.into_split();
    let mut buf = vec![0_u8; 16 * 1024];
    let mut local_read_done = false;
    let mut remote_write_done = false;

    loop {
        if local_read_done && remote_write_done {
            break;
        }

        tokio::select! {
            read = reader.read(&mut buf), if !local_read_done => {
                match read {
                    Ok(0) => {
                        local_read_done = true;
                        let _ = state
                            .send_frame(Frame {
                                kind: FRAME_FIN,
                                stream_id,
                                payload: Vec::new(),
                            })
                            .await;
                    }
                    Ok(n) => {
                        state.add_bytes(stream_id, n as u64, 0).await;
                        if state
                            .send_frame(Frame {
                                kind: FRAME_DATA,
                                stream_id,
                                payload: buf[..n].to_vec(),
                            })
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                    Err(_) => {
                        let _ = state
                            .send_frame(Frame {
                                kind: FRAME_CLOSE,
                                stream_id,
                                payload: Vec::new(),
                            })
                            .await;
                        break;
                    }
                }
            }
            event = events_rx.recv() => {
                match event {
                    Some(StreamEvent::Data(data)) => {
                        state.add_bytes(stream_id, 0, data.len() as u64).await;
                        if writer.write_all(&data).await.is_err() {
                            break;
                        }
                    }
                    Some(StreamEvent::FinishWrite) => {
                        remote_write_done = true;
                        let _ = writer.shutdown().await;
                    }
                    Some(StreamEvent::Close) | None => break,
                }
            }
        }
    }

    let _ = writer.shutdown().await;
    state.streams.lock().await.remove(&stream_id);
    state.close_connection(stream_id, "local_closed").await;
}

async fn bridge_quic_stream(
    state: Arc<SharedState>,
    stream_id: u64,
    socket: TcpStream,
    mut send: SendStream,
    mut recv: RecvStream,
) {
    let (mut tcp_reader, mut tcp_writer) = socket.into_split();
    let mut tcp_buf = vec![0_u8; 16 * 1024];
    let mut quic_buf = vec![0_u8; 16 * 1024];
    let mut tcp_read_done = false;
    let mut quic_read_done = false;

    loop {
        if tcp_read_done && quic_read_done {
            break;
        }

        tokio::select! {
            read = tcp_reader.read(&mut tcp_buf), if !tcp_read_done => {
                match read {
                    Ok(0) => {
                        tcp_read_done = true;
                        let _ = send.finish();
                    }
                    Ok(n) => {
                        state.add_bytes(stream_id, n as u64, 0).await;
                        if send.write_all(&tcp_buf[..n]).await.is_err() {
                            break;
                        }
                    }
                    Err(_) => {
                        let _ = send.reset(quic_transport::CLOSE_CODE);
                        break;
                    }
                }
            }
            read = recv.read(&mut quic_buf), if !quic_read_done => {
                match read {
                    Ok(Some(n)) => {
                        state.add_bytes(stream_id, 0, n as u64).await;
                        if tcp_writer.write_all(&quic_buf[..n]).await.is_err() {
                            break;
                        }
                    }
                    Ok(None) => {
                        quic_read_done = true;
                        let _ = tcp_writer.shutdown().await;
                    }
                    Err(_) => break,
                }
            }
        }
    }

    let _ = send.finish();
    let _ = tcp_writer.shutdown().await;
    state.close_connection(stream_id, "local_closed").await;
}

#[derive(Debug)]
struct Frame {
    kind: u8,
    stream_id: u64,
    payload: Vec<u8>,
}

enum StreamEvent {
    Data(Vec<u8>),
    FinishWrite,
    Close,
}

async fn read_frame<R>(reader: &mut R) -> Result<Frame, RuntimeError>
where
    R: AsyncRead + Unpin,
{
    let kind = reader.read_u8().await?;
    let stream_id = reader.read_u64().await?;
    let len = reader.read_u32().await? as usize;
    if len > MAX_FRAME_LEN {
        return Err(RuntimeError::Protocol(format!(
            "frame too large: {len} bytes"
        )));
    }
    let mut payload = vec![0_u8; len];
    reader.read_exact(&mut payload).await?;
    Ok(Frame {
        kind,
        stream_id,
        payload,
    })
}

async fn write_frame<W>(writer: &mut W, frame: &Frame) -> Result<(), RuntimeError>
where
    W: AsyncWrite + Unpin,
{
    if frame.payload.len() > MAX_FRAME_LEN {
        return Err(RuntimeError::Protocol(format!(
            "frame too large: {} bytes",
            frame.payload.len()
        )));
    }
    writer.write_u8(frame.kind).await?;
    writer.write_u64(frame.stream_id).await?;
    writer.write_u32(frame.payload.len() as u32).await?;
    writer.write_all(&frame.payload).await?;
    writer.flush().await?;
    Ok(())
}

fn hello_payload(config: &Config) -> String {
    format!(
        "v2\n{}\n{}\n{}\n{}\n{}\n{}",
        config.tunnel.id,
        config.auth_token().unwrap_or_default(),
        role_label(config.role),
        config.tunnel.node_id.as_deref().unwrap_or_default(),
        env!("CARGO_PKG_VERSION"),
        service_digest(config)
    )
}

fn hello_matches_config(payload: &str, config: &Config) -> bool {
    let token = config.auth_token().unwrap_or_default();
    if let Some(rest) = payload.strip_prefix("v2\n") {
        let mut lines = rest.lines();
        return lines.next() == Some(config.tunnel.id.as_str()) && lines.next() == Some(token);
    }
    payload == format!("{}\n{}", config.tunnel.id, token)
}

fn service_digest(config: &Config) -> String {
    let mut services: Vec<_> = config
        .services()
        .iter()
        .map(|service| {
            format!(
                "{}:{}:{}",
                service.name,
                direction_label(service.direction),
                service.target_addr()
            )
        })
        .collect();
    services.sort();
    services.join("|")
}

struct OpenRequest {
    service_name: String,
    source_addr: String,
    opened_at_unix_ms: u64,
}

fn encode_open_payload(service: &Service, peer: SocketAddr, opened_at_unix_ms: u64) -> Vec<u8> {
    format!(
        "v2\n{}\n{}\n{}\n{}",
        service.name,
        direction_label(service.direction),
        peer,
        opened_at_unix_ms
    )
    .into_bytes()
}

fn decode_open_payload(payload: &[u8]) -> Result<OpenRequest, RuntimeError> {
    let payload = String::from_utf8(payload.to_vec())
        .map_err(|_| RuntimeError::Protocol("OPEN payload must be UTF-8".into()))?;
    if let Some(rest) = payload.strip_prefix("v2\n") {
        let mut lines = rest.lines();
        let Some(service_name) = lines.next() else {
            return Err(RuntimeError::Protocol(
                "OPEN v2 missing service name".into(),
            ));
        };
        let _direction = lines.next();
        let source_addr = lines.next().unwrap_or_default().to_string();
        let opened_at_unix_ms = lines
            .next()
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or_else(unix_ms);
        return Ok(OpenRequest {
            service_name: service_name.to_string(),
            source_addr,
            opened_at_unix_ms,
        });
    }
    Ok(OpenRequest {
        service_name: payload,
        source_addr: String::new(),
        opened_at_unix_ms: unix_ms(),
    })
}

fn services_json(config: &Config) -> String {
    let mut body = String::from("{\"services\":[");
    for (index, service) in config.services().iter().enumerate() {
        if index > 0 {
            body.push(',');
        }
        body.push_str(&service_json(service));
    }
    body.push_str("]}\n");
    body
}

fn topology_json(config: &Config) -> String {
    let configs = topology_configs(config);
    let mut body = String::from("{\"nodes\":[");
    for (index, config) in configs.iter().enumerate() {
        if index > 0 {
            body.push(',');
        }
        let address = match config.role {
            Role::Relay => config.tunnel.listen.as_deref().unwrap_or_default(),
            Role::Agent => config.tunnel.relay_addr.as_deref().unwrap_or_default(),
        };
        body.push_str(&format!(
            "{{\"role\":\"{}\",\"node_id\":\"{}\",\"address\":\"{}\",\"admin_listen\":\"{}\",\"transport\":\"{}\"}}",
            role_label(config.role),
            json_escape(config.tunnel.node_id.as_deref().unwrap_or_default()),
            json_escape(address),
            json_escape(&config.admin.listen),
            transport_label(config.transport.mode)
        ));
    }
    body.push_str("],\"services\":[");
    let mut seen = Vec::new();
    let mut first = true;
    for config in &configs {
        for service in config.services() {
            let key = format!("{}:{}", service.name, direction_label(service.direction));
            if seen.contains(&key) {
                continue;
            }
            seen.push(key);
            if !first {
                body.push(',');
            }
            first = false;
            body.push_str(&service_json(service));
        }
    }
    body.push_str("]}\n");
    body
}

fn topology_configs(config: &Config) -> Vec<Config> {
    let mut configs = vec![config.clone()];
    let Some(source_path) = config.source_path() else {
        return configs;
    };
    let Some(config_dir) = source_path.parent() else {
        return configs;
    };
    let peer_file = match config.role {
        Role::Relay => "agent.toml",
        Role::Agent => "relay.toml",
    };
    let peer_path = config_dir.join(peer_file);
    if peer_path != source_path && peer_path.exists() {
        if let Ok(peer) = Config::load(&peer_path) {
            configs.push(peer);
        }
    }
    configs.sort_by_key(|config| match config.role {
        Role::Agent => 0,
        Role::Relay => 1,
    });
    configs
}

fn service_json(service: &Service) -> String {
    format!(
        "{{\"name\":\"{}\",\"direction\":\"{}\",\"expose\":\"{}\",\"target\":\"{}\",\"allowed_sources\":{}}}",
        json_escape(&service.name),
        direction_label(service.direction),
        json_escape(service.expose_addr()),
        json_escape(service.target_addr()),
        json_string_array(service.allowed_sources())
    )
}

fn connections_json(connections: &HashMap<u64, ConnectionInfo>) -> String {
    let mut items: Vec<_> = connections.values().collect();
    items.sort_by_key(|info| info.id);
    let mut body = String::from("{\"connections\":[");
    for (index, info) in items.iter().enumerate() {
        if index > 0 {
            body.push(',');
        }
        body.push_str(&connection_info_json(info));
    }
    body.push_str(&format!("],\"active_streams\":{}}}\n", items.len()));
    body
}

fn connection_json(connections: &HashMap<u64, ConnectionInfo>, id: &str) -> Option<String> {
    let id = id.parse::<u64>().ok()?;
    connections
        .get(&id)
        .map(|info| format!("{}\n", connection_info_json(info)))
}

fn connection_info_json(info: &ConnectionInfo) -> String {
    format!(
        "{{\"connection_id\":{},\"service_name\":\"{}\",\"direction\":\"{}\",\"source_addr\":\"{}\",\"target_addr\":\"{}\",\"opened_at_unix_ms\":{},\"bytes_from_local\":{},\"bytes_to_local\":{}}}",
        info.id,
        json_escape(&info.service_name),
        direction_label(info.direction),
        json_escape(&info.source_addr),
        json_escape(&info.target_addr),
        info.opened_at_unix_ms,
        info.bytes_from_local,
        info.bytes_to_local
    )
}

fn metrics_text(
    state: &SharedState,
    config: &Config,
    connections: &HashMap<u64, ConnectionInfo>,
) -> String {
    let connected = u8::from(state.connected.load(Ordering::SeqCst));
    let streams = connections.len();
    let mut body = format!(
        "biz_tunnel_agent_connected {connected}\nbiz_tunnel_active_streams {streams}\nbiz_tunnel_connections_total {}\nbiz_tunnel_dial_failures_total {}\nbiz_tunnel_auth_failures_total {}\nbiz_tunnel_reload_success_total {}\nbiz_tunnel_reload_failure_total {}\n",
        state.counters.accepted.load(Ordering::SeqCst),
        state.counters.dial_failed.load(Ordering::SeqCst),
        state.counters.auth_failed.load(Ordering::SeqCst),
        state.counters.reload_success.load(Ordering::SeqCst),
        state.counters.reload_failure.load(Ordering::SeqCst)
    );
    for service in config.services() {
        let count = connections
            .values()
            .filter(|info| info.service_name == service.name && info.direction == service.direction)
            .count();
        body.push_str(&format!(
            "biz_tunnel_service_active_streams{{service=\"{}\",direction=\"{}\"}} {count}\n",
            metric_escape(&service.name),
            direction_label(service.direction)
        ));
    }
    body
}

fn reload_report_json(status: &str, report: &ReloadReport) -> String {
    format!(
        "{{\"status\":\"{}\",\"generation\":{},\"added\":{},\"removed\":{},\"changed\":{},\"unchanged\":{}}}\n",
        json_escape(status),
        report.generation,
        json_string_array(&report.added),
        json_string_array(&report.removed),
        json_string_array(&report.changed),
        json_string_array(&report.unchanged)
    )
}

fn role_label(role: Role) -> &'static str {
    match role {
        Role::Relay => "relay",
        Role::Agent => "agent",
    }
}

fn direction_label(direction: Direction) -> &'static str {
    match direction {
        Direction::BToA => "b_to_a",
        Direction::AToB => "a_to_b",
    }
}

fn route_owner_label(direction: Direction) -> &'static str {
    match direction {
        Direction::BToA => "relay",
        Direction::AToB => "agent",
    }
}

fn transport_label(mode: TransportMode) -> &'static str {
    match mode {
        TransportMode::Tcp => "tcp",
        TransportMode::TlsTcp => "tls_tcp",
        TransportMode::Quic => "quic",
    }
}

fn json_escape(value: &str) -> String {
    let mut escaped = String::new();
    for ch in value.chars() {
        match ch {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            ch if ch.is_control() => escaped.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => escaped.push(ch),
        }
    }
    escaped
}

fn metric_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn json_string_array(values: &[String]) -> String {
    let mut body = String::from("[");
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            body.push(',');
        }
        body.push('"');
        body.push_str(&json_escape(value));
        body.push('"');
    }
    body.push(']');
    body
}

struct HttpResponse {
    status: &'static str,
    content_type: &'static str,
    body: String,
}

impl HttpResponse {
    fn json(status: &'static str, body: String) -> Self {
        Self {
            status,
            content_type: "application/json",
            body,
        }
    }
}

async fn write_http(
    socket: &mut TcpStream,
    status: &str,
    content_type: &str,
    body: &str,
) -> Result<(), RuntimeError> {
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    socket.write_all(response.as_bytes()).await?;
    socket.shutdown().await?;
    Ok(())
}

fn admin_authorized(request: &str, token: Option<&str>, peer: SocketAddr) -> bool {
    if peer.ip().is_loopback() {
        return true;
    }
    let Some(token) = token else {
        return true;
    };
    let expected = format!("Bearer {token}");
    request.lines().skip(1).any(|line| {
        let Some((name, value)) = line.split_once(':') else {
            return false;
        };
        name.eq_ignore_ascii_case("authorization") && value.trim() == expected
    })
}

#[cfg(test)]
mod tests {
    use std::net::{Ipv4Addr, SocketAddr};

    use super::admin_authorized;

    #[test]
    fn admin_auth_allows_loopback_without_token() {
        let peer = SocketAddr::from((Ipv4Addr::LOCALHOST, 12345));
        assert!(admin_authorized(
            "GET /v1/services HTTP/1.1\r\n\r\n",
            Some("secret"),
            peer
        ));
    }

    #[test]
    fn admin_auth_requires_token_for_non_loopback() {
        let peer = SocketAddr::from((Ipv4Addr::new(192, 0, 2, 10), 12345));
        assert!(!admin_authorized(
            "GET /v1/services HTTP/1.1\r\n\r\n",
            Some("secret"),
            peer
        ));
        assert!(admin_authorized(
            "GET /v1/services HTTP/1.1\r\nAuthorization: Bearer secret\r\n\r\n",
            Some("secret"),
            peer
        ));
    }
}

fn listen_services(role: Role, services: &[Service]) -> Vec<Service> {
    services
        .iter()
        .filter(|service| should_listen(role, service.direction))
        .cloned()
        .collect()
}

fn service_map_for_role(role: Role, services: &[Service]) -> HashMap<String, Service> {
    listen_services(role, services)
        .into_iter()
        .map(|service| (service.name.clone(), service))
        .collect()
}

fn should_listen(role: Role, direction: Direction) -> bool {
    matches!(
        (role, direction),
        (Role::Relay, Direction::BToA) | (Role::Agent, Direction::AToB)
    )
}

fn ensure_reload_compatible(current: &Config, next: &Config) -> Result<(), RuntimeError> {
    if current.role != next.role {
        return Err(RuntimeError::Config("role changes require restart".into()));
    }
    if current.tunnel.id != next.tunnel.id {
        return Err(RuntimeError::Config(
            "tunnel.id changes require restart".into(),
        ));
    }
    if current.tunnel.listen != next.tunnel.listen {
        return Err(RuntimeError::Config(
            "tunnel.listen changes require restart".into(),
        ));
    }
    if current.tunnel.relay_addr != next.tunnel.relay_addr {
        return Err(RuntimeError::Config(
            "tunnel.relay_addr changes require restart".into(),
        ));
    }
    if current.admin.listen != next.admin.listen {
        return Err(RuntimeError::Config(
            "admin.listen changes require restart".into(),
        ));
    }
    if current.transport != next.transport {
        return Err(RuntimeError::Config(
            "transport changes require restart".into(),
        ));
    }
    if current.security != next.security {
        return Err(RuntimeError::Config(
            "security changes require restart".into(),
        ));
    }
    Ok(())
}

fn source_allowed(allowed_sources: &[String], ip: IpAddr) -> bool {
    allowed_sources.is_empty()
        || allowed_sources
            .iter()
            .any(|cidr| source_cidr_matches(cidr, ip).unwrap_or(false))
}

fn source_cidr_matches(cidr: &str, ip: IpAddr) -> Option<bool> {
    let (network, prefix) = cidr.split_once('/')?;
    let network: IpAddr = network.parse().ok()?;
    let prefix: u8 = prefix.parse().ok()?;
    match (network, ip) {
        (IpAddr::V4(network), IpAddr::V4(ip)) => {
            let prefix = prefix.min(32);
            let mask = if prefix == 0 {
                0
            } else {
                u32::MAX << (32 - prefix)
            };
            Some((u32::from(network) & mask) == (u32::from(ip) & mask))
        }
        (IpAddr::V6(network), IpAddr::V6(ip)) => {
            let prefix = prefix.min(128);
            let mask = if prefix == 0 {
                0
            } else {
                u128::MAX << (128 - prefix)
            };
            Some((u128::from(network) & mask) == (u128::from(ip) & mask))
        }
        _ => Some(false),
    }
}

fn unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn quic_error(error: impl fmt::Display) -> RuntimeError {
    RuntimeError::Protocol(format!("quic error: {error}"))
}

#[derive(Debug)]
pub enum RuntimeError {
    Io(std::io::Error),
    Config(String),
    AuthenticationFailed,
    AuthenticationFailedWithReason(String),
    Protocol(String),
    TunnelUnavailable,
    SourceNotAllowed(SocketAddr),
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuntimeError::Io(error) => write!(f, "{error}"),
            RuntimeError::Config(message) => write!(f, "{message}"),
            RuntimeError::AuthenticationFailed => write!(f, "authentication failed"),
            RuntimeError::AuthenticationFailedWithReason(message) => {
                write!(f, "authentication failed: {message}")
            }
            RuntimeError::Protocol(message) => write!(f, "protocol error: {message}"),
            RuntimeError::TunnelUnavailable => write!(f, "tunnel unavailable"),
            RuntimeError::SourceNotAllowed(addr) => write!(f, "source is not allowed: {addr}"),
        }
    }
}

impl std::error::Error for RuntimeError {}

impl From<std::io::Error> for RuntimeError {
    fn from(value: std::io::Error) -> Self {
        RuntimeError::Io(value)
    }
}
