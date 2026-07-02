pub(crate) fn html() -> &'static str {
    ADMIN_UI
}

const ADMIN_UI: &str = r##"<!doctype html>
<html lang="zh-CN">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <link rel="icon" href="data:,">
  <title>中继拓扑配置</title>
  <style>
    :root {
      color-scheme: dark;
      --bg: #070b14;
      --panel: #101827;
      --canvas: #0b1220;
      --card: #141f32;
      --line: #263449;
      --text: #e7eef8;
      --muted: #91a0b7;
      --blue: #5aa7ff;
      --green: #39d98a;
      --amber: #f2b84b;
      --red: #ff7b72;
      --shadow: 0 18px 42px rgba(0, 0, 0, .35);
    }
    * { box-sizing: border-box; }
    [hidden] { display: none !important; }
    body {
      margin: 0;
      background: var(--bg);
      color: var(--text);
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
      letter-spacing: 0;
    }
    button, input, textarea, select {
      font: inherit;
      color: var(--text);
    }
    button {
      height: 36px;
      border: 1px solid var(--line);
      border-radius: 8px;
      background: var(--card);
      padding: 0 12px;
      cursor: pointer;
      font-weight: 650;
      white-space: nowrap;
    }
    button.primary {
      border-color: var(--blue);
      background: var(--blue);
      color: #fff;
    }
    button.subtle { background: #121c2d; }
    button.danger { color: var(--red); }
    input, textarea, select {
      width: 100%;
      border: 1px solid var(--line);
      border-radius: 8px;
      background: #0d1626;
      padding: 9px 10px;
      outline: none;
    }
    textarea { min-height: 72px; resize: vertical; }
    header {
      display: grid;
      grid-template-columns: minmax(230px, 1fr) auto auto;
      align-items: center;
      gap: 16px;
      padding: 18px 22px;
      border-bottom: 1px solid var(--line);
      background: rgba(9, 14, 24, .94);
      position: sticky;
      top: 0;
      z-index: 5;
      backdrop-filter: blur(10px);
    }
    h1 { margin: 0; font-size: 22px; line-height: 1.2; }
    h2 { margin: 0; font-size: 15px; }
    .muted { color: var(--muted); font-size: 12px; }
    .toolbar, .actions, .row { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
    .status-chip {
      display: inline-flex;
      align-items: center;
      gap: 7px;
      height: 28px;
      border: 1px solid rgba(57, 217, 138, .35);
      border-radius: 999px;
      background: rgba(57, 217, 138, .12);
      color: var(--green);
      padding: 0 10px;
      font-size: 12px;
      font-weight: 700;
    }
    .status-chip::before {
      content: "";
      width: 8px;
      height: 8px;
      border-radius: 999px;
      background: currentColor;
    }
    .status-chip.offline {
      border-color: rgba(242, 184, 75, .35);
      background: rgba(242, 184, 75, .12);
      color: var(--amber);
    }
    .shell {
      display: grid;
      grid-template-columns: 260px minmax(520px, 1fr) 330px;
      gap: 16px;
      padding: 16px;
      min-height: calc(100vh - 74px);
    }
    .panel {
      min-width: 0;
      border: 1px solid var(--line);
      border-radius: 8px;
      background: var(--panel);
      box-shadow: var(--shadow);
    }
    .panel-head {
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 10px;
      padding: 14px 16px;
      border-bottom: 1px solid var(--line);
    }
    .panel-body { padding: 14px; }
    .add-card {
      width: 100%;
      min-height: 84px;
      border: 1.5px dashed #355071;
      border-radius: 8px;
      background: #0d1626;
      display: grid;
      place-items: center;
      color: var(--blue);
      font-weight: 750;
      cursor: pointer;
      margin-bottom: 12px;
    }
    .node-list { display: grid; gap: 8px; }
    .node-card {
      border: 1px solid var(--line);
      border-radius: 8px;
      background: var(--card);
      padding: 10px;
      cursor: pointer;
    }
    .node-card.active { border-color: var(--blue); box-shadow: 0 0 0 2px rgba(90, 167, 255, .18); }
    .node-title {
      display: flex;
      align-items: flex-start;
      justify-content: space-between;
      gap: 10px;
      font-weight: 750;
      font-size: 13px;
    }
    .node-note { margin-top: 4px; color: var(--muted); font-size: 12px; line-height: 1.45; }
    .icon-btn {
      width: 28px;
      height: 28px;
      padding: 0;
      display: inline-grid;
      place-items: center;
    }
    .canvas-panel {
      display: grid;
      grid-template-rows: auto auto minmax(520px, 1fr) auto;
      overflow: hidden;
    }
    .main-tunnel {
      display: grid;
      grid-template-columns: minmax(180px, 1fr) auto minmax(180px, 1fr);
      gap: 14px;
      align-items: center;
      padding: 14px 16px;
      border-bottom: 1px solid var(--line);
      background: #0d1626;
    }
    .main-tunnel-card {
      border: 1px solid var(--line);
      border-radius: 8px;
      background: var(--card);
      padding: 10px;
      min-width: 0;
    }
    .main-tunnel-title {
      color: var(--green);
      font-size: 12px;
      font-weight: 800;
      margin-bottom: 5px;
    }
    .main-tunnel-name {
      font-size: 13px;
      font-weight: 800;
      overflow-wrap: anywhere;
    }
    .main-tunnel-note {
      color: var(--muted);
      font-size: 12px;
      line-height: 1.45;
      margin-top: 4px;
      overflow-wrap: anywhere;
    }
    .main-tunnel-link {
      display: grid;
      gap: 4px;
      justify-items: center;
      min-width: 170px;
      color: var(--green);
      font-size: 12px;
      font-weight: 800;
    }
    .main-tunnel-line {
      width: 170px;
      height: 2px;
      background: linear-gradient(90deg, transparent, var(--green), transparent);
      position: relative;
    }
    .main-tunnel-line::before,
    .main-tunnel-line::after {
      content: "";
      position: absolute;
      top: 50%;
      width: 9px;
      height: 9px;
      border-radius: 999px;
      background: var(--green);
      transform: translateY(-50%);
      box-shadow: 0 0 16px rgba(57, 217, 138, .45);
    }
    .main-tunnel-line::before { left: 0; }
    .main-tunnel-line::after { right: 0; }
    .canvas {
      position: relative;
      min-height: 560px;
      background:
        linear-gradient(var(--canvas), var(--canvas)) padding-box,
        repeating-linear-gradient(0deg, transparent 0 35px, rgba(148, 163, 184, .06) 36px),
        repeating-linear-gradient(90deg, transparent 0 35px, rgba(148, 163, 184, .06) 36px);
      overflow: hidden;
    }
    .lane {
      position: absolute;
      top: 18px;
      bottom: 18px;
      width: calc(50% - 25px);
      border: 1px solid var(--line);
      border-radius: 8px;
      background: rgba(12, 20, 34, .72);
      padding: 14px;
    }
    .lane.a { left: 18px; }
    .lane.b { right: 18px; }
    .lane-title {
      color: #cbd5e1;
      font-size: 13px;
      font-weight: 800;
      margin-bottom: 12px;
    }
    .canvas-node {
      position: relative;
      z-index: 2;
      width: min(245px, 88%);
      border: 1px solid var(--line);
      border-radius: 8px;
      background: var(--card);
      padding: 10px;
      margin-bottom: 14px;
      box-shadow: 0 10px 24px rgba(0, 0, 0, .22);
      cursor: pointer;
    }
    .canvas-node.selected { border-color: var(--blue); box-shadow: 0 0 0 2px rgba(90, 167, 255, .18); }
    .canvas-node .path-handle {
      position: absolute;
      right: -13px;
      top: 50%;
      width: 26px;
      height: 26px;
      transform: translateY(-50%);
      border-radius: 999px;
      border: 1px solid #456991;
      background: #0d1626;
      color: var(--blue);
      display: flex;
      align-items: center;
      justify-content: center;
      padding: 0;
      line-height: 1;
      font-size: 18px;
      font-weight: 800;
    }
    .lane.b .canvas-node .path-handle { left: -13px; right: auto; }
    .hint {
      position: absolute;
      left: 50%;
      bottom: 22px;
      transform: translateX(-50%);
      color: var(--muted);
      background: rgba(13, 22, 38, .92);
      border: 1px dashed #3b5575;
      border-radius: 8px;
      padding: 8px 12px;
      font-size: 12px;
      z-index: 3;
    }
    svg.links {
      position: absolute;
      inset: 0;
      pointer-events: auto;
      z-index: 1;
    }
    svg.links path:not(.path-hit) { pointer-events: none; }
    .path-hit {
      pointer-events: stroke;
      cursor: pointer;
    }
    .path-label {
      position: absolute;
      z-index: 3;
      max-width: 260px;
      border: 1px solid var(--line);
      border-radius: 8px;
      background: rgba(17, 28, 45, .96);
      padding: 6px 8px;
      box-shadow: 0 8px 20px rgba(0, 0, 0, .3);
      font-size: 12px;
      opacity: 0;
      transform: translateY(4px);
      pointer-events: none;
      transition: opacity .12s ease, transform .12s ease;
    }
    .path-label.visible {
      opacity: 1;
      transform: translateY(0);
    }
    .canvas:has(.path-hit:hover) .path-label {
      opacity: 1;
      transform: translateY(0);
    }
    .path-label.active { border-color: var(--blue); }
    .path-name { font-weight: 800; }
    .path-endpoints { color: var(--muted); margin-top: 3px; overflow-wrap: anywhere; }
    .test-result { color: var(--muted); font-size: 12px; line-height: 1.4; }
    .test-result.ok { color: var(--green); }
    .test-result.failed { color: var(--red); }
    .test-result.skipped, .test-result.running { color: var(--amber); }
    .inspector { display: grid; grid-template-rows: auto 1fr; }
    .form { display: grid; gap: 10px; }
    .field label {
      display: block;
      margin-bottom: 5px;
      color: #cbd5e1;
      font-size: 12px;
      font-weight: 750;
    }
    .seg {
      display: grid;
      grid-template-columns: 1fr 1fr;
      border: 1px solid var(--line);
      border-radius: 8px;
      overflow: hidden;
    }
    .seg button {
      border: 0;
      border-radius: 0;
      background: #0d1626;
    }
    .seg button.active { background: rgba(90, 167, 255, .16); color: var(--blue); }
    .output {
      border-top: 1px solid var(--line);
      background: #0d1626;
      padding: 12px 14px;
      display: grid;
      gap: 10px;
    }
    .tabs { display: flex; gap: 8px; align-items: center; flex-wrap: wrap; }
    .tabs button.active { border-color: var(--blue); color: var(--blue); background: rgba(90, 167, 255, .13); }
    pre {
      max-height: 220px;
      margin: 0;
      overflow: auto;
      border-radius: 8px;
      background: #050a13;
      color: #e6edf6;
      padding: 12px;
      font: 12px/1.45 ui-monospace, SFMono-Regular, Menlo, monospace;
    }
    .chips { display: flex; gap: 8px; flex-wrap: wrap; }
    .chip {
      display: inline-flex;
      align-items: center;
      height: 26px;
      border-radius: 999px;
      background: rgba(90, 167, 255, .13);
      color: var(--blue);
      padding: 0 10px;
      font-size: 12px;
      font-weight: 750;
    }
    .token {
      max-width: 170px;
      height: 36px;
      padding: 0 10px;
    }
    @media (max-width: 1180px) {
      header { grid-template-columns: 1fr; }
      .shell { grid-template-columns: 1fr; }
      .canvas { min-height: 720px; overflow: auto; }
      .lane { position: relative; inset: auto; width: auto; margin: 14px; min-height: 280px; }
      .hint { position: static; transform: none; margin: 0 14px 14px; }
      .path-label { position: relative; left: auto !important; top: auto !important; margin: 8px 14px; max-width: none; opacity: 1; transform: none; }
      svg.links { display: none; }
      .main-tunnel { grid-template-columns: 1fr; }
      .main-tunnel-link { justify-items: start; }
      .canvas-panel { grid-template-rows: auto auto auto auto; }
    }
  </style>
</head>
<body>
  <header>
    <div>
      <h1>中继拓扑配置</h1>
      <div class="muted">在流程图中添加节点、连接通讯路径并生成两端配置 · ui-route-nodes-20260702</div>
    </div>
    <div class="toolbar">
      <span id="statusChip" class="status-chip offline">agent 未连接 relay</span>
      <button id="addNodeTop">＋ 添加节点</button>
      <button id="addPathTop">＋ 添加通讯路径</button>
      <button id="generateTop" class="primary">自动生成配置</button>
      <button id="validateTop">校验</button>
    </div>
    <div class="actions">
      <input id="token" class="token" type="password" autocomplete="current-password" placeholder="管理令牌">
      <button id="saveToken" class="subtle">保存</button>
      <button id="importJson" class="subtle">导入</button>
      <button id="exportJson" class="subtle">导出</button>
      <button id="reload" class="subtle">热重载</button>
    </div>
  </header>

  <main class="shell">
    <aside class="panel">
      <div class="panel-head">
        <h2>节点库</h2>
        <button id="addNodeSmall" class="icon-btn" title="添加节点">＋</button>
      </div>
      <div class="panel-body">
        <div id="addNodeCard" class="add-card">＋ 添加节点</div>
        <div id="nodeList" class="node-list"></div>
      </div>
    </aside>

    <section class="panel canvas-panel">
      <div class="panel-head">
        <div>
          <h2>通讯流程画布</h2>
          <div id="tunnelText" class="muted">agent role -> relay role</div>
        </div>
        <div class="row">
          <span id="serviceCount" class="muted">0 条路径</span>
          <span id="lastRefresh" class="muted">未刷新</span>
        </div>
      </div>
      <div class="main-tunnel">
        <div class="main-tunnel-card">
          <div class="main-tunnel-title">主隧道主动连接侧</div>
          <div id="mainTunnelAgent" class="main-tunnel-name">agent role</div>
          <div id="mainTunnelAgentNote" class="main-tunnel-note">等待运行态</div>
        </div>
        <div class="main-tunnel-link">
          <div>主隧道 QUIC</div>
          <div class="main-tunnel-line"></div>
          <div id="mainTunnelState">未连接</div>
        </div>
        <div class="main-tunnel-card">
          <div class="main-tunnel-title">主隧道被连接侧</div>
          <div id="mainTunnelRelay" class="main-tunnel-name">relay role</div>
          <div id="mainTunnelRelayNote" class="main-tunnel-note">等待运行态</div>
        </div>
      </div>
      <div id="canvas" class="canvas">
        <svg id="links" class="links"></svg>
        <div class="lane a">
          <div class="lane-title">romm-zg（主动连接侧）</div>
          <div id="laneA"></div>
        </div>
        <div class="lane b">
          <div class="lane-title">room-kg（被连接侧）</div>
          <div id="laneB"></div>
        </div>
        <div id="pathLabels"></div>
        <div class="hint">点击节点右侧 ＋ 连接两个节点，或使用顶部“添加通讯路径”</div>
      </div>
      <div class="output">
        <div class="tabs">
          <button data-tab="relay" class="active">relay.toml <span id="relayCount">0</span> 条路径</button>
          <button data-tab="agent">agent.toml <span id="agentCount">0</span> 条路径</button>
          <button id="copyConfig" class="subtle">复制当前配置</button>
          <button id="saveConfig" class="primary">保存并同步配置</button>
        </div>
        <pre id="configPreview"></pre>
        <div class="chips">
          <span class="chip" id="checkConfig">配置待校验</span>
          <span class="chip" id="saveConfigResult">尚未保存</span>
          <span class="chip">端口 80 可监听</span>
          <span class="chip">本机管理免授权</span>
        </div>
      </div>
    </section>

    <aside class="panel inspector">
      <div class="panel-head">
        <h2 id="inspectorTitle">路径配置</h2>
        <button id="deleteSelected" class="icon-btn danger" title="删除">×</button>
      </div>
      <div class="panel-body">
        <div id="pathForm" class="form">
          <div class="field">
            <label>通讯名称</label>
            <input id="pathName">
          </div>
          <div class="field">
            <label>方向</label>
            <div class="seg">
              <button id="dirAToB" type="button">a_to_b</button>
              <button id="dirBToA" type="button">b_to_a</button>
            </div>
          </div>
          <div class="field">
            <label>起点节点</label>
            <select id="pathFrom"></select>
          </div>
          <div class="field">
            <label>目标节点</label>
            <select id="pathTo"></select>
          </div>
          <div class="field">
            <label>起点入口</label>
            <input id="pathExpose">
          </div>
          <div class="field">
            <label>目标地址</label>
            <input id="pathTarget">
          </div>
          <div class="field">
            <label>来源限制</label>
            <input id="pathAllowed" placeholder="127.0.0.1/32, 192.0.2.0/24">
          </div>
          <div class="field">
            <label>备注</label>
            <textarea id="pathNote" placeholder="例如：Agent 上报 Core"></textarea>
          </div>
          <button id="savePath" class="primary">保存路径</button>
          <button id="testPath" type="button">测试当前路径</button>
          <div id="pathTestResult" class="test-result"></div>
        </div>

        <div id="nodeForm" class="form" hidden>
          <div class="field">
            <label>节点名称</label>
            <input id="nodeName">
          </div>
          <div class="field">
            <label>地址</label>
            <input id="nodeAddress">
          </div>
          <div class="field">
            <label>所属机房</label>
            <select id="nodeLane">
              <option value="a">romm-zg（主动连接侧）</option>
              <option value="b">room-kg（被连接侧）</option>
            </select>
          </div>
          <div class="field">
            <label>节点备注</label>
            <textarea id="nodeNote" placeholder="例如：流媒体 core 节点"></textarea>
          </div>
          <button id="saveNode" class="primary">保存节点</button>
        </div>
      </div>
    </aside>
  </main>

  <script>
    const $ = (id) => document.getElementById(id);
    const tokenInput = $("token");
    tokenInput.value = sessionStorage.getItem("bizTunnelAdminToken") || "";

    const state = {
      tunnelId: "change-me-tunnel-id",
      relayAddr: "relay.example.local:9443",
      role: "",
      activeTab: "relay",
      selection: { type: "path", id: "example-forward" },
      editing: false,
      runtimeReady: false,
      mainTunnel: {
        connected: false,
        agent: { name: "agent role", note: "等待运行态" },
        relay: { name: "relay role", note: "等待运行态" },
      },
      tests: {},
      nodes: [
        { id: "agent-proxy", lane: "a", name: "agent role", address: "agent.example.local", note: "主动连接侧代理节点" },
        { id: "local-client", lane: "a", name: "local client", address: "client.example.local", note: "本地业务客户端" },
        { id: "local-service", lane: "a", name: "local service", address: "service-a.example.local", note: "主动侧目标服务" },
        { id: "relay-proxy", lane: "b", name: "relay role", address: "relay.example.local", note: "被连接侧中继节点" },
        { id: "remote-client", lane: "b", name: "remote client", address: "operator.example.local", note: "远端业务客户端" },
        { id: "remote-service", lane: "b", name: "remote service", address: "service-b.example.local", note: "被连接侧目标服务" },
      ],
      paths: [
        { id: "example-forward", name: "replace-with-forward-route", direction: "a_to_b", from: "local-client", to: "remote-service", expose: "0.0.0.0:0", target: "service-b.example.local:0", allowed: "", note: "主动侧访问被连接侧", color: "blue" },
        { id: "example-reverse", name: "replace-with-reverse-route", direction: "b_to_a", from: "remote-client", to: "local-service", expose: "0.0.0.0:0", target: "service-a.example.local:0", allowed: "", note: "被连接侧访问主动侧", color: "green" },
      ],
    };

    function authHeaders() {
      const token = tokenInput.value.trim();
      return token ? { Authorization: `Bearer ${token}` } : {};
    }

    async function api(path, options = {}) {
      const response = await fetch(path, {
        ...options,
        headers: { ...authHeaders(), ...(options.headers || {}) },
      });
      if (!response.ok) throw new Error(`${path} ${response.status}`);
      const type = response.headers.get("content-type") || "";
      return type.includes("json") ? response.json() : response.text();
    }

    const esc = (value) => String(value ?? "").replace(/[&<>"']/g, (c) => ({
      "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;"
    }[c]));

    function uid(prefix) {
      return `${prefix}-${Math.random().toString(16).slice(2, 8)}`;
    }

    function routeId(name) {
      return `route-${String(name).replace(/[^a-z0-9_-]/gi, "-")}`;
    }

    function selectedNode() {
      return state.nodes.find((node) => node.id === state.selection.id);
    }

    function selectedPath() {
      return state.paths.find((path) => path.id === state.selection.id);
    }

    function select(type, id) {
      state.editing = false;
      state.selection = { type, id };
      render();
    }

    function addNode() {
      const id = uid("node");
      state.nodes.push({ id, lane: "a", name: "新节点", address: "", note: "备注：" });
      select("node", id);
    }

    function addPath(fromId) {
      const from = state.nodes.find((node) => node.id === fromId) || state.nodes[0];
      if (!from) {
        addNode();
        return;
      }
      const to = state.nodes.find((node) => node.lane !== from.lane) || state.nodes[1] || from;
      const id = uid("route");
      const direction = from.lane === "a" ? "a_to_b" : "b_to_a";
      state.paths.push({
        id,
        name: "new-route",
        direction,
        from: from.id,
        to: to.id,
        expose: direction === "a_to_b" ? "127.0.0.1:0" : "0.0.0.0:0",
        target: "目标IP:端口",
        allowed: "",
        note: "新通讯路径",
        color: direction === "a_to_b" ? "blue" : "green",
      });
      select("path", id);
    }

    function deleteSelected() {
      if (state.selection.type === "node") {
        state.paths = state.paths.filter((path) => path.from !== state.selection.id && path.to !== state.selection.id);
        state.nodes = state.nodes.filter((node) => node.id !== state.selection.id);
        state.selection = { type: "path", id: state.paths[0]?.id };
      } else {
        state.paths = state.paths.filter((path) => path.id !== state.selection.id);
        state.selection = { type: "path", id: state.paths[0]?.id };
      }
      render();
    }

    function nodeCard(node, inCanvas) {
      const active = state.selection.type === "node" && state.selection.id === node.id ? "active selected" : "";
      const cls = inCanvas ? `canvas-node ${active}` : `node-card ${active}`;
      const pathButton = inCanvas ? `<button class="path-handle" data-path-from="${node.id}" title="添加路径">＋</button>` : "";
      return `<div class="${cls}" data-node="${node.id}">
        <div class="node-title">
          <span>${esc(node.name)} ${esc(node.address)}</span>
          ${inCanvas ? "" : `<button class="icon-btn" data-node-edit="${node.id}" title="编辑">✎</button>`}
        </div>
        <div class="node-note">${esc(node.note || "备注：")}</div>
        ${pathButton}
      </div>`;
    }

    function renderNodes() {
      $("nodeList").innerHTML = state.nodes.map((node) => nodeCard(node, false)).join("");
      $("laneA").innerHTML = state.nodes.filter((node) => node.lane === "a").map((node) => nodeCard(node, true)).join("");
      $("laneB").innerHTML = state.nodes.filter((node) => node.lane === "b").map((node) => nodeCard(node, true)).join("");
      bindNodeClicks();
    }

    function bindNodeClicks() {
      document.querySelectorAll("[data-node]").forEach((el) => {
        el.addEventListener("click", (event) => {
          if (event.target.closest("[data-path-from]")) return;
          select("node", el.dataset.node);
        });
      });
      document.querySelectorAll("[data-node-edit]").forEach((el) => {
        el.addEventListener("click", (event) => {
          event.stopPropagation();
          select("node", el.dataset.nodeEdit);
        });
      });
      document.querySelectorAll("[data-path-from]").forEach((el) => {
        el.addEventListener("click", (event) => {
          event.stopPropagation();
          addPath(el.dataset.pathFrom);
        });
      });
    }

    function colorOf(path) {
      if (path.color === "green" || path.direction === "b_to_a") return "#15945b";
      if (path.color === "amber") return "#b7791f";
      return "#2563eb";
    }

    function renderLinks() {
      const canvas = $("canvas");
      const box = canvas.getBoundingClientRect();
      const svg = $("links");
      svg.setAttribute("viewBox", `0 0 ${box.width} ${box.height}`);
      svg.innerHTML = "";
      $("pathLabels").innerHTML = "";

      const linkGroups = new Map();
      for (const path of state.paths) {
        const key = [path.from, path.to].sort().join("|");
        if (!linkGroups.has(key)) linkGroups.set(key, []);
        linkGroups.get(key).push(path.id);
      }

      for (const path of state.paths) {
        const fromEl = canvas.querySelector(`[data-node="${path.from}"]`);
        const toEl = canvas.querySelector(`[data-node="${path.to}"]`);
        if (!fromEl || !toEl) continue;
        const a = fromEl.getBoundingClientRect();
        const b = toEl.getBoundingClientRect();
        const group = linkGroups.get([path.from, path.to].sort().join("|")) || [path.id];
        const offset = (group.indexOf(path.id) - (group.length - 1) / 2) * 28;
        const startX = a.left < b.left ? a.right - box.left : a.left - box.left;
        const startY = a.top + a.height / 2 - box.top + offset;
        const endX = a.left < b.left ? b.left - box.left : b.right - box.left;
        const endY = b.top + b.height / 2 - box.top + offset;
        const midX = (startX + endX) / 2;
        const d = `M ${startX} ${startY} C ${midX} ${startY}, ${midX} ${endY}, ${endX} ${endY}`;
        const line = document.createElementNS("http://www.w3.org/2000/svg", "path");
        line.setAttribute("d", d);
        line.setAttribute("fill", "none");
        line.setAttribute("stroke", colorOf(path));
        line.setAttribute("stroke-width", state.selection.id === path.id ? "3" : "2");
        line.setAttribute("stroke-linecap", "round");
        svg.appendChild(line);

        const label = document.createElement("div");
        label.className = `path-label ${state.selection.type === "path" && state.selection.id === path.id ? "active" : ""}`;
        label.style.left = `${Math.max(16, Math.min(box.width - 280, midX - 125))}px`;
        label.style.top = `${Math.max(58, Math.min(box.height - 80, (startY + endY) / 2 - 22))}px`;
        const test = state.tests[path.id];
        const testHtml = test ? `<div class="test-result ${esc(test.status)}">${esc(testResultText(test))}</div>` : "";
        label.innerHTML = `<div class="path-name">${esc(path.name)} / ${esc(path.direction)}</div>
          <div class="path-endpoints">${esc(path.expose)} -> ${esc(path.target)}</div>
          ${testHtml}`;
        $("pathLabels").appendChild(label);

        const hit = document.createElementNS("http://www.w3.org/2000/svg", "path");
        hit.setAttribute("class", "path-hit");
        hit.setAttribute("d", d);
        hit.setAttribute("fill", "none");
        hit.setAttribute("stroke", "rgba(37, 99, 235, 0.01)");
        hit.setAttribute("stroke-width", "18");
        hit.setAttribute("stroke-linecap", "round");
        hit.addEventListener("pointerenter", () => label.classList.add("visible"));
        hit.addEventListener("pointerleave", () => label.classList.remove("visible"));
        hit.addEventListener("pointerover", () => label.classList.add("visible"));
        hit.addEventListener("pointerout", () => label.classList.remove("visible"));
        hit.addEventListener("mouseenter", () => label.classList.add("visible"));
        hit.addEventListener("mouseleave", () => label.classList.remove("visible"));
        hit.addEventListener("mouseover", () => label.classList.add("visible"));
        hit.addEventListener("mouseout", () => label.classList.remove("visible"));
        hit.addEventListener("click", () => select("path", path.id));
        svg.appendChild(hit);
      }
    }

    function fillNodeOptions(selectEl, current) {
      selectEl.innerHTML = state.nodes.map((node) =>
        `<option value="${esc(node.id)}" ${node.id === current ? "selected" : ""}>${esc(node.name)} ${esc(node.address)}</option>`
      ).join("");
    }

    function renderInspector() {
      const pathMode = state.selection.type === "path" && selectedPath();
      $("pathForm").hidden = !pathMode;
      $("nodeForm").hidden = !!pathMode;
      $("inspectorTitle").textContent = pathMode ? "路径配置" : "节点备注";

      if (pathMode) {
        const path = selectedPath();
        $("pathName").value = path.name;
        $("pathExpose").value = path.expose;
        $("pathTarget").value = path.target;
        $("pathAllowed").value = path.allowed;
        $("pathNote").value = path.note;
        fillNodeOptions($("pathFrom"), path.from);
        fillNodeOptions($("pathTo"), path.to);
        $("dirAToB").classList.toggle("active", path.direction === "a_to_b");
        $("dirBToA").classList.toggle("active", path.direction === "b_to_a");
        const test = state.tests[path.id];
        $("pathTestResult").textContent = test ? testResultText(test) : localTestHint(path);
        $("pathTestResult").className = `test-result ${test?.status || ""}`;
      } else {
        const node = selectedNode() || state.nodes[0];
        if (!node) return;
        state.selection = { type: "node", id: node.id };
        $("nodeName").value = node.name;
        $("nodeAddress").value = node.address;
        $("nodeLane").value = node.lane;
        $("nodeNote").value = node.note;
      }
    }

    function updatePathFromForm() {
      const path = selectedPath();
      if (!path) return;
      path.name = $("pathName").value.trim() || "new-route";
      path.from = $("pathFrom").value;
      path.to = $("pathTo").value;
      path.expose = $("pathExpose").value.trim();
      path.target = $("pathTarget").value.trim();
      path.allowed = $("pathAllowed").value.trim();
      path.note = $("pathNote").value.trim();
      path.color = path.direction === "b_to_a" ? "green" : "blue";
      render();
    }

    function updateNodeFromForm() {
      const node = selectedNode();
      if (!node) return;
      node.name = $("nodeName").value.trim() || "新节点";
      node.address = $("nodeAddress").value.trim();
      node.lane = $("nodeLane").value;
      node.note = $("nodeNote").value.trim();
      render();
    }

    function setDirection(direction) {
      const path = selectedPath();
      if (!path) return;
      path.direction = direction;
      path.color = direction === "b_to_a" ? "green" : "blue";
      render();
    }

    function routeOwner(path) {
      return path.direction === "b_to_a" ? "relay" : "agent";
    }

    function localTestHint(path) {
      if (!state.role) return "测试会从当前节点发起";
      if (state.role === routeOwner(path)) return "测试当前节点入口到对端目标 TCP 端口";
      return `该路径监听在 ${routeOwner(path)} 侧，请到对应页面测试`;
    }

    function testResultText(result) {
      const labels = { ok: "通过", failed: "失败", skipped: "跳过", not_found: "未找到", running: "测试中" };
      return `${labels[result.status] || result.status}：${result.message || ""}`;
    }

    async function testSelectedPath() {
      const path = selectedPath();
      if (!path) return;
      const id = path.id;
      state.tests[id] = { status: "running", message: "正在测试通道联通性" };
      render();
      try {
        state.tests[id] = await api(`/v1/services/test/${encodeURIComponent(path.name)}`, { method: "POST" });
      } catch (error) {
        state.tests[id] = { status: "failed", message: error.message };
      }
      render();
    }

    function listValues(value) {
      return value.split(",").map((item) => item.trim()).filter(Boolean);
    }

    function tomlArray(values) {
      return `[${values.map((value) => `"${value.replace(/"/g, '\\"')}"`).join(", ")}]`;
    }

    function routeToml(path) {
      const lines = [];
      lines.push(path.direction === "b_to_a" ? "[[b_to_a]]" : "[[a_to_b]]");
      lines.push(`name = "${path.name}"`);
      if (path.direction === "b_to_a") {
        lines.push(`expose_on_relay = "${path.expose}"`);
        lines.push(`target_from_agent = "${path.target}"`);
      } else {
        lines.push(`expose_on_agent = "${path.expose}"`);
        lines.push(`target_from_relay = "${path.target}"`);
      }
      const allowed = listValues(path.allowed);
      if (allowed.length) lines.push(`allowed_sources = ${tomlArray(allowed)}`);
      return lines.join("\n");
    }

    function routesToml() {
      return `${state.paths.map(routeToml).join("\n\n")}\n`;
    }

    function configFor(role) {
      const relay = role === "relay";
      const header = relay
        ? `role = "relay"\n\n[tunnel]\nid = "${state.tunnelId}"\nnode_id = "change-me-relay-node"\nlisten = "0.0.0.0:9443"\ntoken = "change-me-long-random-token"`
        : `role = "agent"\n\n[tunnel]\nid = "${state.tunnelId}"\nnode_id = "change-me-agent-node"\nrelay_addr = "${state.relayAddr}"\ntoken = "change-me-long-random-token"`;
      const security = relay
        ? `[security]\nmode = "token"\ncert = "/etc/biz-tunnel/certs/server.pem"\nkey = "/etc/biz-tunnel/certs/server.key"`
        : `[security]\nmode = "token"\nca_cert = "/etc/biz-tunnel/certs/ca.pem"\nserver_name = "relay.example.local"`;
      const common = `\n\n[transport]\nmode = "quic"\nfallback = []\nconnect_timeout_secs = 10\nidle_timeout_secs = 300\nmax_frame_bytes = 1048576\n\n${security}\n\n[admin]\nlisten = "${relay ? "0.0.0.0:18080" : "0.0.0.0:18081"}"\n\n[defaults]\ndrain_timeout_secs = 30\ndial_timeout_secs = 5`;
      return `${header}${common}\n\n${routesToml()}`;
    }

    function renderOutput() {
      $("relayCount").textContent = state.paths.length;
      $("agentCount").textContent = state.paths.length;
      $("configPreview").textContent = configFor(state.activeTab);
      $("serviceCount").textContent = `${state.paths.length} 条路径`;
    }

    function renderMainTunnel() {
      $("mainTunnelAgent").textContent = state.mainTunnel.agent.name;
      $("mainTunnelAgentNote").textContent = state.mainTunnel.agent.note;
      $("mainTunnelRelay").textContent = state.mainTunnel.relay.name;
      $("mainTunnelRelayNote").textContent = state.mainTunnel.relay.note;
      $("mainTunnelState").textContent = state.mainTunnel.connected ? "已连接" : "未连接";
      $("mainTunnelState").style.color = state.mainTunnel.connected ? "var(--green)" : "var(--amber)";
    }

    function validateTopology() {
      const errors = [];
      const names = new Set();
      for (const path of state.paths) {
        if (!path.name || names.has(path.name)) errors.push(`路径名称重复或为空：${path.name || "(空)"}`);
        names.add(path.name);
        if (!path.expose) errors.push(`${path.name} 缺少本地入口`);
        if (!path.target) errors.push(`${path.name} 缺少目标地址`);
      }
      $("checkConfig").textContent = errors.length ? `校验失败 ${errors.length} 项` : "配置校验通过";
      $("checkConfig").style.background = errors.length ? "rgba(255, 123, 114, .14)" : "rgba(57, 217, 138, .14)";
      $("checkConfig").style.color = errors.length ? "var(--red)" : "var(--green)";
      if (errors.length) alert(errors.join("\n"));
    }

    async function saveConfig() {
      if (state.selection.type === "path") updatePathFromForm();
      $("saveConfigResult").textContent = "正在保存";
      $("saveConfigResult").style.background = "rgba(242, 184, 75, .14)";
      $("saveConfigResult").style.color = "var(--amber)";
      try {
        const result = await api("/v1/configs/save", {
          method: "POST",
          headers: { "Content-Type": "text/plain; charset=utf-8" },
          body: routesToml(),
        });
        $("saveConfigResult").textContent = result.message || result.status;
        $("saveConfigResult").style.background = result.status === "saved" ? "rgba(57, 217, 138, .14)" : "rgba(255, 123, 114, .14)";
        $("saveConfigResult").style.color = result.status === "saved" ? "var(--green)" : "var(--red)";
        refreshRuntime();
      } catch (error) {
        $("saveConfigResult").textContent = error.message;
        $("saveConfigResult").style.background = "rgba(255, 123, 114, .14)";
        $("saveConfigResult").style.color = "var(--red)";
      }
    }

    async function refreshRuntime() {
      const firstLoad = !state.runtimeReady;
      try {
        const [health, tunnel, topology, connections] = await Promise.all([
          api("/healthz"),
          api("/v1/tunnel"),
          api("/v1/topology"),
          api("/v1/connections"),
        ]);
        state.role = health.role || state.role;
        state.tunnelId = tunnel.id || state.tunnelId;
        state.mainTunnel.connected = !!tunnel.agent_connected;
        $("statusChip").textContent = tunnel.agent_connected ? "agent 已连接 relay" : "agent 未连接 relay";
        $("statusChip").classList.toggle("offline", !tunnel.agent_connected);
        $("tunnelText").textContent = `${health.role || "node"} · ${state.tunnelId} · ${connections.active_streams || 0} 条活动连接`;
        $("lastRefresh").textContent = new Date().toLocaleTimeString();
        syncTopology(topology);
      } catch {
        $("statusChip").textContent = "管理接口未连接";
        $("statusChip").classList.add("offline");
      }
      state.runtimeReady = true;
      render(!firstLoad);
    }

    function syncTopology(topology) {
      const runtimeNodes = (topology.nodes || []).map((node) => {
        const agent = node.role === "agent";
        const address = agent
          ? (node.node_id || node.admin_listen || node.address || "")
          : (node.address || node.node_id || node.admin_listen || "");
        const details = [
          `${agent ? "主动连接侧" : "被连接侧"} · ${node.transport || "tcp"}`,
          node.node_id ? `node_id: ${node.node_id}` : "",
          node.address ? `${agent ? "relay_addr" : "listen"}: ${node.address}` : "",
          node.admin_listen ? `admin: ${node.admin_listen}` : "",
        ].filter(Boolean);
        return {
          role: node.role,
          name: agent ? "agent role" : "relay role",
          address,
          note: details.join(" / "),
        };
      });
      const agentNode = runtimeNodes.find((node) => node.role === "agent");
      const relayNode = runtimeNodes.find((node) => node.role === "relay");
      state.mainTunnel.agent = agentNode
        ? { name: `${agentNode.name} ${agentNode.address}`, note: agentNode.note }
        : { name: "agent role", note: "主动连接侧代理节点" };
      state.mainTunnel.relay = relayNode
        ? { name: `${relayNode.name} ${relayNode.address}`, note: relayNode.note }
        : { name: "relay role", note: "被连接侧中继节点" };
      state.nodes = [];
      if (topology.services) syncRuntimeServices(topology.services);
    }

    function syncRuntimeServices(services) {
      const nodes = [];
      state.paths = services.map((service) => {
        const old = state.paths.find((item) => item.name === service.name);
        const direction = service.direction;
        const id = old?.id || routeId(service.name);
        const from = `${id}-from`;
        const to = `${id}-to`;
        nodes.push({
          id: from,
          lane: direction === "a_to_b" ? "a" : "b",
          name: `${service.name} 入口`,
          address: service.expose,
          note: direction === "a_to_b" ? "主动侧本地监听" : "被连接侧本地监听",
        });
        nodes.push({
          id: to,
          lane: direction === "a_to_b" ? "b" : "a",
          name: `${service.name} 目标`,
          address: service.target,
          note: direction === "a_to_b" ? "被连接侧目标服务" : "主动侧目标服务",
        });
        return {
          id,
          name: service.name,
          direction,
          from,
          to,
          expose: service.expose,
          target: service.target,
          allowed: (service.allowed_sources || []).join(", "),
          note: old?.note || "来自合并配置",
          color: direction === "b_to_a" ? "green" : "blue",
        };
      });
      state.nodes = nodes;
      if (state.selection.type === "path" && !state.paths.some((path) => path.id === state.selection.id)) {
        state.selection = { type: "path", id: state.paths[0]?.id };
      } else if (state.selection.type === "node" && !state.nodes.some((node) => node.id === state.selection.id)) {
        state.selection = { type: "path", id: state.paths[0]?.id };
      }
    }

    function render(skipInspector = false) {
      renderNodes();
      if (!skipInspector && !state.editing) renderInspector();
      renderMainTunnel();
      renderOutput();
      requestAnimationFrame(renderLinks);
    }

    function exportJson() {
      const blob = new Blob([JSON.stringify({ nodes: state.nodes, paths: state.paths }, null, 2)], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = "biz-tunnel-topology.json";
      a.click();
      URL.revokeObjectURL(url);
    }

    function importJson() {
      const raw = prompt("粘贴拓扑 JSON");
      if (!raw) return;
      const data = JSON.parse(raw);
      if (Array.isArray(data.nodes)) state.nodes = data.nodes;
      if (Array.isArray(data.paths)) state.paths = data.paths;
      state.selection = { type: "path", id: state.paths[0]?.id };
      render();
    }

    $("addNodeTop").addEventListener("click", addNode);
    $("addNodeSmall").addEventListener("click", addNode);
    $("addNodeCard").addEventListener("click", addNode);
    $("addPathTop").addEventListener("click", () => addPath(state.selection.type === "node" ? state.selection.id : undefined));
    $("generateTop").addEventListener("click", renderOutput);
    $("validateTop").addEventListener("click", validateTopology);
    $("deleteSelected").addEventListener("click", deleteSelected);
    $("savePath").addEventListener("click", updatePathFromForm);
    $("testPath").addEventListener("click", testSelectedPath);
    $("saveNode").addEventListener("click", updateNodeFromForm);
    $("dirAToB").addEventListener("click", () => setDirection("a_to_b"));
    $("dirBToA").addEventListener("click", () => setDirection("b_to_a"));
    $("pathName").addEventListener("change", updatePathFromForm);
    $("pathExpose").addEventListener("change", updatePathFromForm);
    $("pathTarget").addEventListener("change", updatePathFromForm);
    $("pathAllowed").addEventListener("change", updatePathFromForm);
    $("pathNote").addEventListener("change", updatePathFromForm);
    $("pathFrom").addEventListener("change", updatePathFromForm);
    $("pathTo").addEventListener("change", updatePathFromForm);
    $("nodeName").addEventListener("change", updateNodeFromForm);
    $("nodeAddress").addEventListener("change", updateNodeFromForm);
    $("nodeLane").addEventListener("change", updateNodeFromForm);
    $("nodeNote").addEventListener("change", updateNodeFromForm);
    document.querySelectorAll(".inspector input, .inspector textarea, .inspector select").forEach((el) => {
      el.addEventListener("focusin", () => { state.editing = true; });
      el.addEventListener("focusout", () => { setTimeout(() => { state.editing = false; }, 0); });
    });
    $("copyConfig").addEventListener("click", () => navigator.clipboard?.writeText(configFor(state.activeTab)));
    $("saveConfig").addEventListener("click", saveConfig);
    $("saveToken").addEventListener("click", () => sessionStorage.setItem("bizTunnelAdminToken", tokenInput.value.trim()));
    $("importJson").addEventListener("click", importJson);
    $("exportJson").addEventListener("click", exportJson);
    $("reload").addEventListener("click", async () => {
      await api("/v1/services/reload", { method: "POST" });
      refreshRuntime();
    });
    document.querySelectorAll(".tabs [data-tab]").forEach((button) => {
      button.addEventListener("click", () => {
        state.activeTab = button.dataset.tab;
        document.querySelectorAll(".tabs [data-tab]").forEach((item) => item.classList.toggle("active", item === button));
        renderOutput();
      });
    });
    window.addEventListener("resize", () => requestAnimationFrame(renderLinks));

    render();
    refreshRuntime();
    setInterval(refreshRuntime, 5000);
  </script>
</body>
</html>
"##;
