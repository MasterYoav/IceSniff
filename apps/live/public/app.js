const app = document.querySelector("#app");

const icons = {
  packets: "▤",
  stats: "▥",
  conversations: "◎",
  streams: "⇄",
  transactions: "↝",
  settings: "⚙",
  profile: "◉"
};

const initialUI = {
  activeSection: localStorage.getItem("icesniff.live.section") || "packets",
  appTheme: localStorage.getItem("icesniff.live.theme") || "defaultDark",
  fontChoice: localStorage.getItem("icesniff.live.font") || "rounded",
  sidebarCollapsed: localStorage.getItem("icesniff.live.sidebar") === "1",
  chatCollapsed: localStorage.getItem("icesniff.live.chat") !== "0",
  filterExpression: "",
  packetLimit: "200",
  selectedPacketIndex: null,
  packetJSON: "Select a packet to inspect details.",
  packets: [],
  statsRows: [],
  conversations: [],
  streams: [],
  transactions: [],
  schemaVersion: "",
  captureFormat: "",
  packetCountHint: 0,
  totalPackets: 0,
  capturePath: "",
  availableCaptureInterfaces: ["en0"],
  selectedCaptureInterface: "en0",
  isSniffing: false,
  isCaptureTransitioning: false,
  captureBackendName: "Unavailable",
  captureBackendMessage: "Live capture backend unavailable.",
  statusMessage: "Choose a capture file to begin."
};

const ui = { ...initialUI };
let refreshTimer = null;
let stateTimer = null;
let refreshAbortController = null;

const hiddenUploadInput = document.createElement("input");
hiddenUploadInput.type = "file";
hiddenUploadInput.accept = ".pcap,.pcapng,.cap,.dmp,.pcapngng";
hiddenUploadInput.className = "hidden";
document.body.appendChild(hiddenUploadInput);

hiddenUploadInput.addEventListener("change", async () => {
  const [file] = hiddenUploadInput.files || [];
  if (!file) {
    return;
  }
  await uploadCapture(file);
  hiddenUploadInput.value = "";
});

function persistUI() {
  localStorage.setItem("icesniff.live.section", ui.activeSection);
  localStorage.setItem("icesniff.live.theme", ui.appTheme);
  localStorage.setItem("icesniff.live.font", ui.fontChoice);
  localStorage.setItem("icesniff.live.sidebar", ui.sidebarCollapsed ? "1" : "0");
  localStorage.setItem("icesniff.live.chat", ui.chatCollapsed ? "1" : "0");
}

function applyBodyClasses() {
  document.body.className = "";
  document.body.classList.add(`theme-${ui.appTheme}`);
  document.body.classList.add(`font-${ui.fontChoice}`);
}

function setUIState(patch) {
  Object.assign(ui, patch);
  persistUI();
  applyBodyClasses();
  render();
}

async function api(pathname, options = {}) {
  const response = await fetch(pathname, {
    headers: {
      "Content-Type": options.body instanceof Blob || options.body instanceof ArrayBuffer ? "application/octet-stream" : "application/json"
    },
    ...options
  });

  if (response.headers.get("content-type")?.includes("application/json")) {
    const payload = await response.json();
    if (!response.ok) {
      throw new Error(payload.message || "Request failed.");
    }
    return payload;
  }

  if (!response.ok) {
    throw new Error(await response.text());
  }

  return response;
}

async function loadServerState() {
  const payload = await api("/api/state");
  Object.assign(ui, payload.state);
  applyBodyClasses();
  render();
}

async function refreshAll() {
  if (!ui.capturePath) {
    return;
  }

  refreshAbortController?.abort();
  refreshAbortController = new AbortController();

  try {
    const response = await fetch("/api/refresh", {
      method: "POST",
      headers: {
        "Content-Type": "application/json"
      },
      body: JSON.stringify({
        filter: ui.filterExpression,
        limit: ui.packetLimit
      }),
      signal: refreshAbortController.signal
    });

    const payload = await response.json();

    if (response.status === 202 && payload.transient) {
      ui.statusMessage = payload.message;
      render();
      return;
    }

    if (!response.ok || !payload.ok) {
      throw new Error(payload.message || "Refresh failed.");
    }

    const { inspect, list, stats, conversations, streams, transactions, state } = payload;
    Object.assign(ui, state);
    ui.schemaVersion = inspect.schema_version || "";
    ui.captureFormat = inspect.format || "";
    ui.packetCountHint = inspect.packet_count_hint || 0;
    ui.totalPackets = list.total_packets || 0;
    ui.packets = list.packets || [];
    ui.statsRows = [
      ...(stats.link_layer_counts || []).map((row) => ({ bucket: "Link", ...row })),
      ...(stats.network_layer_counts || []).map((row) => ({ bucket: "Network", ...row })),
      ...(stats.transport_layer_counts || []).map((row) => ({ bucket: "Transport", ...row }))
    ];
    ui.conversations = conversations.conversations || [];
    ui.streams = streams.streams || [];
    ui.transactions = transactions.transactions || [];

    const selectedStillExists = ui.packets.some((packet) => packet.index === ui.selectedPacketIndex);
    if (!selectedStillExists) {
      ui.selectedPacketIndex = null;
      ui.packetJSON = "Select a packet to inspect details.";
    }

    render();
  } catch (error) {
    if (error.name === "AbortError") {
      return;
    }
    ui.statusMessage = error.message;
    render();
  }
}

async function loadPacket(index) {
  try {
    const payload = await api(`/api/packet/${index}`);
    ui.selectedPacketIndex = index;
    ui.packetJSON = JSON.stringify(payload.packet, null, 2);
    render();
  } catch (error) {
    ui.packetJSON = `Request failed: ${error.message}`;
    render();
  }
}

async function uploadCapture(file) {
  ui.statusMessage = `Uploading ${file.name}...`;
  render();

  const bytes = await file.arrayBuffer();
  const response = await fetch(`/api/captures/upload?name=${encodeURIComponent(file.name)}`, {
    method: "POST",
    headers: {
      "Content-Type": "application/octet-stream"
    },
    body: bytes
  });
  const payload = await response.json();
  if (!response.ok || !payload.ok) {
    throw new Error(payload.message || "Upload failed.");
  }
  Object.assign(ui, payload.state);
  await refreshAll();
}

async function toggleCapture() {
  try {
    if (ui.isSniffing) {
      const payload = await api("/api/capture/stop", { method: "POST", body: JSON.stringify({}) });
      Object.assign(ui, payload.state);
      await refreshAll();
    } else {
      const payload = await api("/api/capture/start", {
        method: "POST",
        body: JSON.stringify({ interface: ui.selectedCaptureInterface })
      });
      Object.assign(ui, payload.state);
    }
    render();
  } catch (error) {
    ui.statusMessage = error.message;
    render();
  }
}

async function downloadCapture() {
  try {
    const response = await fetch("/api/capture/save", {
      method: "POST",
      headers: {
        "Content-Type": "application/json"
      },
      body: JSON.stringify({ filter: ui.filterExpression })
    });
    if (!response.ok) {
      const payload = await response.json();
      throw new Error(payload.message || "Save failed.");
    }
    const blob = await response.blob();
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = "icesniff-export.pcap";
    anchor.click();
    URL.revokeObjectURL(url);
  } catch (error) {
    ui.statusMessage = error.message;
    render();
  }
}

function scheduleRefresh() {
  window.clearTimeout(refreshTimer);
  refreshTimer = window.setTimeout(() => {
    refreshAll();
  }, 350);
}

function startPolling() {
  window.clearInterval(stateTimer);
  stateTimer = window.setInterval(async () => {
    try {
      await loadServerState();
      if (ui.isSniffing && ui.capturePath) {
        await refreshAll();
      }
    } catch (error) {
      ui.statusMessage = error.message;
      render();
    }
  }, 1200);
}

function packetRowTemplate(packet) {
  const active = packet.index === ui.selectedPacketIndex;
  return `
    <button class="packet-row ${active ? "active" : ""}" data-packet-index="${packet.index}">
      <div class="packet-top">
        <span class="packet-id">#${packet.index}</span>
        <span class="protocol-pill">${escapeHTML((packet.protocol || "").toUpperCase())}</span>
        <span class="packet-time">${escapeHTML(`${packet.timestamp_seconds ?? ""}.${packet.timestamp_fraction ?? ""}`)}</span>
      </div>
      <div class="packet-path mono">${escapeHTML(packet.source || "")} → ${escapeHTML(packet.destination || "")}</div>
      <div class="packet-info">${escapeHTML(packet.info || "")}</div>
    </button>
  `;
}

function genericRowTemplate(row, type) {
  if (type === "stats") {
    return `
      <div class="generic-row">
        <div class="generic-top">
          <span class="protocol-pill">${escapeHTML(row.bucket.toUpperCase())}</span>
          <span class="generic-main mono">${escapeHTML(row.name || "")}</span>
          <span class="generic-stat">${escapeHTML(String(row.count ?? 0))}</span>
        </div>
      </div>
    `;
  }

  if (type === "conversations") {
    return `
      <div class="generic-row">
        <div class="generic-top">
          <span class="protocol-pill">${escapeHTML((row.protocol || "").toUpperCase())}</span>
          <span class="generic-main mono">${escapeHTML(row.endpoint_a || "")} ↔ ${escapeHTML(row.endpoint_b || "")}</span>
          <span class="generic-stat">${escapeHTML(String(row.packets ?? 0))}</span>
        </div>
      </div>
    `;
  }

  if (type === "streams") {
    return `
      <div class="generic-row">
        <div class="generic-top">
          <span class="protocol-pill">${escapeHTML((row.protocol || "").toUpperCase())}</span>
          <span class="generic-main">${escapeHTML(row.session_state || "")}</span>
          <span class="generic-stat mono">packets: ${escapeHTML(String(row.packets ?? 0))}</span>
        </div>
        <div class="detail-line mono">${escapeHTML(row.client || "")} → ${escapeHTML(row.server || "")}</div>
      </div>
    `;
  }

  return `
    <div class="generic-row">
      <div class="generic-top">
        <span class="protocol-pill">${escapeHTML((row.protocol || "").toUpperCase())}</span>
        <span class="generic-main">${escapeHTML(row.state || "")}</span>
      </div>
      <div class="detail-line mono">REQ: ${escapeHTML(row.request_summary || "")}</div>
      <div class="detail-line mono secondary">RES: ${escapeHTML(row.response_summary || "")}</div>
    </div>
  `;
}

function renderPacketsSection() {
  return `
    <div class="packets-section">
      <div class="packets-top">
        <section class="panel">
          <div class="panel-title">Filter</div>
          <input id="filter-input" class="field-input mono" placeholder="protocol & port" value="${escapeAttribute(ui.filterExpression)}">
        </section>
        <section class="panel">
          <div class="panel-title">Live Capture</div>
          <div class="capture-subtitle muted">${escapeHTML(ui.captureBackendMessage)}</div>
          <div class="settings-row" style="margin-top: 10px;">
            <select id="capture-interface" class="select">
              ${ui.availableCaptureInterfaces.map((value) => `<option value="${escapeAttribute(value)}" ${value === ui.selectedCaptureInterface ? "selected" : ""}>${escapeHTML(value)}</option>`).join("")}
            </select>
          </div>
          <div class="capture-actions" style="margin-top: 12px;">
            <button id="toggle-capture" class="primary-button ${ui.isSniffing ? "danger" : ""}">${ui.isSniffing ? "Stop Sniffing" : "Start Sniffing"}</button>
            <button id="save-capture" class="ghost-button" ${ui.capturePath ? "" : "disabled"}>Save Capture</button>
            <button id="open-capture-main" class="ghost-button">Open Capture</button>
          </div>
        </section>
      </div>
      <div class="counter-row">
        <span class="counter-label">Packets</span>
        <span class="counter-value">${escapeHTML(String(ui.totalPackets || 0))}</span>
      </div>
      <div class="packets-content">
        <section class="panel list-pane">
          <div class="list-shell">
            <div class="list-title">Packets</div>
            <div class="packet-list">
              ${ui.packets.length ? ui.packets.map(packetRowTemplate).join("") : '<div class="empty-state">Open a capture or start sniffing to populate packets.</div>'}
            </div>
          </div>
        </section>
        <section class="panel json-pane">
          <div class="json-shell">
            <div class="json-title">Packet JSON</div>
            <pre class="json-content">${escapeHTML(ui.packetJSON)}</pre>
          </div>
        </section>
      </div>
    </div>
  `;
}

function renderGenericSection(title, rows, type) {
  return `
    <section class="panel section-pane">
      <div class="generic-shell">
        <div class="section-title">${title}</div>
        <div class="generic-list">
          ${rows.length ? rows.map((row) => genericRowTemplate(row, type)).join("") : `<div class="empty-state">No ${title.toLowerCase()} available for the current capture.</div>`}
        </div>
      </div>
    </section>
  `;
}

function renderProfileSection() {
  return `
    <section class="panel profile-pane">
      <div class="section-title">Profile</div>
      <div class="detail-line">Local web app profile placeholder.</div>
      <div class="detail-line secondary">The macOS app already owns real auth. This web track keeps the same visual shell first.</div>
    </section>
  `;
}

function renderSettingsSection() {
  const themes = [
    ["defaultDark", "Default Dark"],
    ["defaultLight", "Default Light"],
    ["ocean", "Ocean"],
    ["ember", "Ember"],
    ["forest", "Forest"]
  ];
  const fonts = [
    ["rounded", "Rounded"],
    ["system", "System"],
    ["serif", "Serif"],
    ["monospaced", "Monospaced"]
  ];

  return `
    <section class="panel settings-pane">
      <div class="settings-grid">
        <div>
          <div class="section-title">Theme</div>
          <div class="theme-pills">
            ${themes.map(([value, label]) => `<button class="choice-pill ${ui.appTheme === value ? "active" : ""}" data-theme-choice="${value}">${label}</button>`).join("")}
          </div>
        </div>
        <div>
          <div class="section-title">Font</div>
          <div class="font-pills">
            ${fonts.map(([value, label]) => `<button class="choice-pill ${ui.fontChoice === value ? "active" : ""}" data-font-choice="${value}">${label}</button>`).join("")}
          </div>
        </div>
        <div class="detail-line secondary">
          The web app uses the same Rust capture and analysis backend as the macOS app. Theme and font are currently browser-local preferences.
        </div>
      </div>
    </section>
  `;
}

function renderMainSection() {
  if (ui.activeSection === "packets") {
    return renderPacketsSection();
  }
  if (ui.activeSection === "stats") {
    return renderGenericSection("Protocol Distribution", ui.statsRows, "stats");
  }
  if (ui.activeSection === "conversations") {
    return renderGenericSection("Conversations", ui.conversations, "conversations");
  }
  if (ui.activeSection === "streams") {
    return renderGenericSection("Streams", ui.streams, "streams");
  }
  if (ui.activeSection === "transactions") {
    return renderGenericSection("Transactions", ui.transactions, "transactions");
  }
  if (ui.activeSection === "profile") {
    return renderProfileSection();
  }
  return renderSettingsSection();
}

function render() {
  const sections = [
    ["packets", "Packets"],
    ["stats", "Stats"],
    ["conversations", "Conversations"],
    ["streams", "Streams"],
    ["transactions", "Transactions"]
  ];

  app.innerHTML = `
    <div class="app-shell ${ui.sidebarCollapsed ? "sidebar-collapsed" : ""} ${ui.chatCollapsed ? "chat-collapsed" : ""}">
      <aside class="sidebar ${ui.sidebarCollapsed ? "collapsed" : ""}">
        <div class="sidebar-top">
          <img class="app-icon" src="${ui.appTheme === "defaultLight" ? "/media/icons/icon-iOS-Default-1024x1024@1x.png" : "/media/icons/icon-iOS-Dark-1024x1024@1x.png"}" alt="IceSniff">
          ${ui.sidebarCollapsed ? "" : `<div class="brand-copy"><div class="brand-title">IceSniff</div><div class="brand-subtitle">Capture Browser</div></div>`}
        </div>
        <nav class="sidebar-nav">
          ${sections.map(([key, label]) => `
            <button class="nav-button ${ui.activeSection === key ? "active" : "inactive"}" data-section="${key}">
              <span class="nav-icon">${icons[key]}</span>
              ${ui.sidebarCollapsed ? "" : `<span class="nav-label">${label}</span>`}
            </button>
          `).join("")}
        </nav>
        <div class="sidebar-spacer"></div>
        <div class="sidebar-footer">
          ${ui.sidebarCollapsed ? "" : `<button id="sidebar-open-button" class="open-button"><span>⊞</span><span>Open Capture</span></button>`}
          <button class="nav-button ${ui.activeSection === "settings" ? "active" : "inactive"}" data-section="settings">
            <span class="nav-icon">${icons.settings}</span>
            ${ui.sidebarCollapsed ? "" : `<span class="nav-label">Settings</span>`}
          </button>
          <button class="profile-button ${ui.activeSection === "profile" ? "active" : "inactive"}" data-section="profile">
            <span class="nav-icon">${icons.profile}</span>
            ${ui.sidebarCollapsed ? "" : `<span class="profile-copy">Profile</span>`}
          </button>
        </div>
      </aside>
      <main class="main-column">
        <header class="header">
          <div class="header-title">${escapeHTML(titleForSection(ui.activeSection))}</div>
          <div class="header-status">
            <span>${escapeHTML(ui.statusMessage)}</span>
            <button id="copy-status" class="icon-button" title="Copy status">⧉</button>
            ${ui.chatCollapsed ? `<button id="toggle-chat-open" class="icon-button" title="Show AI panel">✦</button>` : ""}
            <button id="toggle-sidebar" class="icon-button" title="${ui.sidebarCollapsed ? "Show sidebar" : "Hide sidebar"}">${ui.sidebarCollapsed ? "⇥" : "⇤"}</button>
          </div>
        </header>
        <section class="main-card">
          <div class="main-scroll">
            ${renderMainSection()}
          </div>
        </section>
      </main>
      ${ui.chatCollapsed ? "" : `
        <aside class="chat-panel">
          <div class="chat-header">
            <div class="section-title">AI Chat</div>
            <button id="toggle-chat-close" class="icon-button" title="Hide AI panel">✕</button>
          </div>
          <div class="chat-card chat-messages">
            <div class="chat-bubble assistant">The macOS app already ships the full AI panel. The live web app is mirroring the layout first while staying focused on the Rust capture surface.</div>
            <div class="chat-bubble user">Use the same backend. Keep the look aligned.</div>
            <div class="chat-bubble assistant">That is the current setup here: same icesniff-cli, same icesniff-capture-helper, same tshark path resolution.</div>
          </div>
          <div class="chat-card composer">
            <textarea placeholder="Web AI compose surface will follow the macOS panel." disabled></textarea>
            <button class="primary-button" disabled>Send</button>
          </div>
        </aside>
      `}
    </div>
  `;

  wireEvents();
}

function wireEvents() {
  app.querySelectorAll("[data-section]").forEach((button) => {
    button.addEventListener("click", () => {
      setUIState({ activeSection: button.dataset.section });
    });
  });

  app.querySelector("#toggle-sidebar")?.addEventListener("click", () => {
    setUIState({ sidebarCollapsed: !ui.sidebarCollapsed });
  });

  app.querySelector("#toggle-chat-open")?.addEventListener("click", () => {
    setUIState({ chatCollapsed: false });
  });

  app.querySelector("#toggle-chat-close")?.addEventListener("click", () => {
    setUIState({ chatCollapsed: true });
  });

  app.querySelector("#copy-status")?.addEventListener("click", async () => {
    try {
      await navigator.clipboard.writeText(ui.statusMessage);
    } catch {}
  });

  app.querySelector("#sidebar-open-button")?.addEventListener("click", () => {
    hiddenUploadInput.click();
  });

  app.querySelector("#open-capture-main")?.addEventListener("click", () => {
    hiddenUploadInput.click();
  });

  app.querySelector("#filter-input")?.addEventListener("input", (event) => {
    ui.filterExpression = event.target.value;
    scheduleRefresh();
  });

  app.querySelector("#capture-interface")?.addEventListener("change", (event) => {
    ui.selectedCaptureInterface = event.target.value;
    render();
  });

  app.querySelector("#toggle-capture")?.addEventListener("click", () => {
    toggleCapture();
  });

  app.querySelector("#save-capture")?.addEventListener("click", () => {
    downloadCapture();
  });

  app.querySelectorAll("[data-packet-index]").forEach((button) => {
    button.addEventListener("click", () => {
      loadPacket(Number(button.dataset.packetIndex));
    });
  });

  app.querySelectorAll("[data-theme-choice]").forEach((button) => {
    button.addEventListener("click", () => {
      setUIState({ appTheme: button.dataset.themeChoice });
    });
  });

  app.querySelectorAll("[data-font-choice]").forEach((button) => {
    button.addEventListener("click", () => {
      setUIState({ fontChoice: button.dataset.fontChoice });
    });
  });
}

function titleForSection(section) {
  switch (section) {
    case "packets":
      return "Packets";
    case "stats":
      return "Stats";
    case "conversations":
      return "Conversations";
    case "streams":
      return "Streams";
    case "transactions":
      return "Transactions";
    case "profile":
      return "Profile";
    case "settings":
      return "Settings";
    default:
      return "IceSniff";
  }
}

function escapeHTML(value) {
  return String(value ?? "")
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

function escapeAttribute(value) {
  return escapeHTML(value).replaceAll("'", "&#39;");
}

applyBodyClasses();
render();
await loadServerState();
startPolling();
