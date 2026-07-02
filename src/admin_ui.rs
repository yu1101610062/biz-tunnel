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
      color-scheme: light;
      --bg: #f4f7fb;
      --panel: #ffffff;
      --canvas: #f8fafd;
      --line: #d9e2ee;
      --text: #182230;
      --muted: #667085;
      --blue: #2563eb;
      --green: #15945b;
      --amber: #b7791f;
      --red: #b42318;
      --shadow: 0 14px 36px rgba(16, 24, 40, .08);
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
      background: #fff;
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
    button.subtle { background: #f8fafc; }
    button.danger { color: var(--red); }
    input, textarea, select {
      width: 100%;
      border: 1px solid var(--line);
      border-radius: 8px;
      background: #fff;
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
      background: rgba(255, 255, 255, .94);
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
      border: 1px solid #b7dfca;
      border-radius: 999px;
      background: #f0fbf5;
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
      border-color: #efd0a3;
      background: #fff8eb;
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
      border: 1.5px dashed #aac1dd;
      border-radius: 8px;
      background: #f8fbff;
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
      background: #fff;
      padding: 10px;
      cursor: pointer;
    }
    .node-card.active { border-color: var(--blue); box-shadow: 0 0 0 2px rgba(37, 99, 235, .12); }
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
      grid-template-rows: auto minmax(520px, 1fr) auto;
      overflow: hidden;
    }
    .canvas {
      position: relative;
      min-height: 560px;
      background:
        linear-gradient(var(--canvas), var(--canvas)) padding-box,
        repeating-linear-gradient(0deg, transparent 0 35px, rgba(145, 158, 171, .08) 36px),
        repeating-linear-gradient(90deg, transparent 0 35px, rgba(145, 158, 171, .08) 36px);
      overflow: hidden;
    }
    .lane {
      position: absolute;
      top: 18px;
      bottom: 18px;
      width: calc(50% - 25px);
      border: 1px solid #dfe7f1;
      border-radius: 8px;
      background: rgba(255, 255, 255, .72);
      padding: 14px;
    }
    .lane.a { left: 18px; }
    .lane.b { right: 18px; }
    .lane-title {
      color: #344054;
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
      background: #fff;
      padding: 10px;
      margin-bottom: 14px;
      box-shadow: 0 10px 22px rgba(16, 24, 40, .06);
      cursor: pointer;
    }
    .canvas-node.selected { border-color: var(--blue); box-shadow: 0 0 0 2px rgba(37, 99, 235, .12); }
    .canvas-node .path-handle {
      position: absolute;
      right: -13px;
      top: 50%;
      width: 26px;
      height: 26px;
      transform: translateY(-50%);
      border-radius: 999px;
      border: 1px solid #bcd0eb;
      background: #fff;
      color: var(--blue);
      display: grid;
      place-items: center;
      font-weight: 800;
    }
    .lane.b .canvas-node .path-handle { left: -13px; right: auto; }
    .hint {
      position: absolute;
      left: 50%;
      bottom: 22px;
      transform: translateX(-50%);
      color: var(--muted);
      background: rgba(255,255,255,.9);
      border: 1px dashed #b8c7da;
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
      background: rgba(255, 255, 255, .95);
      padding: 6px 8px;
      box-shadow: 0 8px 20px rgba(16, 24, 40, .07);
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
      color: #344054;
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
      background: #fff;
    }
    .seg button.active { background: #eaf1ff; color: var(--blue); }
    .output {
      border-top: 1px solid var(--line);
      background: #fff;
      padding: 12px 14px;
      display: grid;
      gap: 10px;
    }
    .tabs { display: flex; gap: 8px; align-items: center; flex-wrap: wrap; }
    .tabs button.active { border-color: var(--blue); color: var(--blue); background: #f3f7ff; }
    pre {
      max-height: 220px;
      margin: 0;
      overflow: auto;
      border-radius: 8px;
      background: #101828;
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
      background: #eef6ff;
      color: #1d4ed8;
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
      .canvas-panel { grid-template-rows: auto auto auto; }
    }
  </style>
</head>
<body>
  <header>
    <div>
      <h1>中继拓扑配置</h1>
      <div class="muted">在流程图中添加节点、连接通讯路径并生成两端配置</div>
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
      <div id="canvas" class="canvas">
        <svg id="links" class="links"></svg>
        <div class="lane a">
          <div class="lane-title">room-a（主动连接侧）</div>
          <div id="laneA"></div>
        </div>
        <div class="lane b">
          <div class="lane-title">room-b（被连接侧）</div>
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
        </div>
        <pre id="configPreview"></pre>
        <div class="chips">
          <span class="chip" id="checkConfig">配置待校验</span>
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
              <option value="a">room-a（主动连接侧）</option>
              <option value="b">room-b（被连接侧）</option>
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

    function selectedNode() {
      return state.nodes.find((node) => node.id === state.selection.id);
    }

    function selectedPath() {
      return state.paths.find((path) => path.id === state.selection.id);
    }

    function select(type, id) {
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

      for (const path of state.paths) {
        const fromEl = canvas.querySelector(`[data-node="${path.from}"]`);
        const toEl = canvas.querySelector(`[data-node="${path.to}"]`);
        if (!fromEl || !toEl) continue;
        const a = fromEl.getBoundingClientRect();
        const b = toEl.getBoundingClientRect();
        const startX = a.left < b.left ? a.right - box.left : a.left - box.left;
        const startY = a.top + a.height / 2 - box.top;
        const endX = a.left < b.left ? b.left - box.left : b.right - box.left;
        const endY = b.top + b.height / 2 - box.top;
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

    function configFor(role) {
      const relay = role === "relay";
      const header = relay
        ? `role = "relay"\n\n[tunnel]\nid = "${state.tunnelId}"\nnode_id = "change-me-relay-node"\nlisten = "0.0.0.0:9443"\ntoken = "change-me-long-random-token"`
        : `role = "agent"\n\n[tunnel]\nid = "${state.tunnelId}"\nnode_id = "change-me-agent-node"\nrelay_addr = "${state.relayAddr}"\ntoken = "change-me-long-random-token"`;
      const security = relay
        ? `[security]\nmode = "token"\ncert = "/etc/biz-tunnel/certs/server.pem"\nkey = "/etc/biz-tunnel/certs/server.key"`
        : `[security]\nmode = "token"\nca_cert = "/etc/biz-tunnel/certs/ca.pem"\nserver_name = "relay.example.local"`;
      const common = `\n\n[transport]\nmode = "quic"\nfallback = []\nconnect_timeout_secs = 10\nidle_timeout_secs = 300\nmax_frame_bytes = 1048576\n\n${security}\n\n[admin]\nlisten = "${relay ? "0.0.0.0:18080" : "0.0.0.0:18081"}"\n\n[defaults]\ndrain_timeout_secs = 30\ndial_timeout_secs = 5`;
      return `${header}${common}\n\n${state.paths.map(routeToml).join("\n\n")}\n`;
    }

    function renderOutput() {
      $("relayCount").textContent = state.paths.length;
      $("agentCount").textContent = state.paths.length;
      $("configPreview").textContent = configFor(state.activeTab);
      $("serviceCount").textContent = `${state.paths.length} 条路径`;
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
      $("checkConfig").style.background = errors.length ? "#fff1f0" : "#edf9f2";
      $("checkConfig").style.color = errors.length ? "var(--red)" : "var(--green)";
      if (errors.length) alert(errors.join("\n"));
    }

    async function refreshRuntime() {
      try {
        const [health, tunnel, topology, connections] = await Promise.all([
          api("/healthz"),
          api("/v1/tunnel"),
          api("/v1/topology"),
          api("/v1/connections"),
        ]);
        state.role = health.role || state.role;
        state.tunnelId = tunnel.id || state.tunnelId;
        $("statusChip").textContent = tunnel.agent_connected ? "agent 已连接 relay" : "agent 未连接 relay";
        $("statusChip").classList.toggle("offline", !tunnel.agent_connected);
        $("tunnelText").textContent = `${health.role || "node"} · ${state.tunnelId} · ${connections.active_streams || 0} 条活动连接`;
        $("lastRefresh").textContent = new Date().toLocaleTimeString();
        syncTopology(topology);
      } catch {
        $("statusChip").textContent = "管理接口未连接";
        $("statusChip").classList.add("offline");
      }
      render();
    }

    function syncTopology(topology) {
      const runtimeNodes = (topology.nodes || []).map((node) => {
        const agent = node.role === "agent";
        const id = agent ? "agent-proxy" : "relay-proxy";
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
          id,
          lane: agent ? "a" : "b",
          name: agent ? "agent role" : "relay role",
          address,
          note: details.join(" / "),
        };
      });
      const hasAgent = runtimeNodes.some((node) => node.id === "agent-proxy");
      const hasRelay = runtimeNodes.some((node) => node.id === "relay-proxy");
      if (!hasAgent) runtimeNodes.unshift({ id: "agent-proxy", lane: "a", name: "agent role", address: "agent.example.local", note: "主动连接侧代理节点" });
      if (!hasRelay) runtimeNodes.push({ id: "relay-proxy", lane: "b", name: "relay role", address: state.relayAddr, note: "被连接侧中继节点" });
      state.nodes = runtimeNodes;
      if (topology.services) syncRuntimeServices(topology.services);
    }

    function syncRuntimeServices(services) {
      state.paths = services.map((service) => {
        const old = state.paths.find((item) => item.name === service.name);
        const direction = service.direction;
        return {
          id: old?.id || `route-${service.name.replace(/[^a-z0-9_-]/gi, "-")}`,
          name: service.name,
          direction,
          from: direction === "a_to_b" ? "agent-proxy" : "relay-proxy",
          to: direction === "a_to_b" ? "relay-proxy" : "agent-proxy",
          expose: service.expose,
          target: service.target,
          allowed: (service.allowed_sources || []).join(", "),
          note: old?.note || "来自合并配置",
          color: direction === "b_to_a" ? "green" : "blue",
        };
      });
      if (!state.paths.some((path) => path.id === state.selection.id)) {
        state.selection = { type: "path", id: state.paths[0]?.id };
      }
    }

    function render() {
      renderNodes();
      renderInspector();
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
    $("copyConfig").addEventListener("click", () => navigator.clipboard?.writeText(configFor(state.activeTab)));
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
