const app = document.querySelector("#app");

function symbol(paths, viewBox = "0 0 24 24") {
  return `
    <svg viewBox="${viewBox}" aria-hidden="true" focusable="false">
      ${paths}
    </svg>
  `;
}

const icons = {
  packets: symbol('<rect x="4.5" y="6" width="15" height="3" rx="1.5"></rect><rect x="4.5" y="10.5" width="15" height="3" rx="1.5"></rect><rect x="4.5" y="15" width="15" height="3" rx="1.5"></rect>'),
  stats: symbol('<path d="M5 18.5h14"></path><rect x="6" y="11" width="2.6" height="5.5" rx="1"></rect><rect x="10.7" y="8" width="2.6" height="8.5" rx="1"></rect><rect x="15.4" y="5.5" width="2.6" height="11" rx="1"></rect>'),
  conversations: symbol('<path d="M7.5 16.5c-2.3 0-4-1.8-4-4.5s1.7-4.5 4-4.5c1.3 0 2.5.6 3.2 1.7"></path><path d="M16.5 7.5c2.3 0 4 1.8 4 4.5s-1.7 4.5-4 4.5c-1.3 0-2.5-.6-3.2-1.7"></path><path d="M9.5 14.5 14.5 9.5"></path>'),
  streams: symbol('<path d="M5 8.5h10"></path><path d="M11.5 5 15 8.5 11.5 12"></path><path d="M19 15.5H9"></path><path d="M12.5 12 9 15.5 12.5 19"></path>'),
  transactions: symbol('<path d="M7 7h7"></path><path d="M11 3l4 4-4 4"></path><path d="M17 17h-7"></path><path d="M13 13l-4 4 4 4"></path>'),
  settings: symbol('<path d="M12 3.5v2"></path><path d="M12 18.5v2"></path><path d="M20.5 12h-2"></path><path d="M5.5 12h-2"></path><path d="m17.66 6.34-1.42 1.42"></path><path d="m7.76 16.24-1.42 1.42"></path><path d="m17.66 17.66-1.42-1.42"></path><path d="m7.76 7.76-1.42-1.42"></path><circle cx="12" cy="12" r="3.2"></circle>'),
  profile: symbol('<circle cx="12" cy="8.2" r="3.1"></circle><path d="M6 18.3c1.5-2.4 3.5-3.6 6-3.6s4.5 1.2 6 3.6"></path>'),
  sparkles: symbol('<path d="M12 3.8 13.7 8l4.3 1.7-4.3 1.7-1.7 4.3-1.7-4.3L6 9.7 10.3 8z"></path><path d="M18.5 15.5 19.2 17l1.5.7-1.5.7-.7 1.5-.7-1.5-1.5-.7 1.5-.7z"></path>'),
  copy: symbol('<rect x="9" y="8" width="9" height="11" rx="2"></rect><path d="M7 15H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h7a2 2 0 0 1 2 2v1"></path>')
};

const initialUI = {
  activeSection: localStorage.getItem("icesniff.live.section") || "packets",
  appTheme: localStorage.getItem("icesniff.live.theme") || "defaultDark",
  fontChoice: localStorage.getItem("icesniff.live.font") || "rounded",
  panelBackgrounds: localStorage.getItem("icesniff.live.panelBackgrounds") !== "0",
  navOpen: localStorage.getItem("icesniff.live.navOpen") !== "0",
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
  try {
    await uploadCapture(file);
  } catch (error) {
    ui.statusMessage = error.message;
    render();
  }
  hiddenUploadInput.value = "";
});

function persistUI() {
  localStorage.setItem("icesniff.live.section", ui.activeSection);
  localStorage.setItem("icesniff.live.theme", ui.appTheme);
  localStorage.setItem("icesniff.live.font", ui.fontChoice);
  localStorage.setItem("icesniff.live.panelBackgrounds", ui.panelBackgrounds ? "1" : "0");
  localStorage.setItem("icesniff.live.navOpen", ui.navOpen ? "1" : "0");
  localStorage.setItem("icesniff.live.chat", ui.chatCollapsed ? "1" : "0");
}

function applyBodyClasses() {
  document.body.className = "";
  document.body.classList.add(`theme-${ui.appTheme}`);
  document.body.classList.add(`font-${ui.fontChoice}`);
  document.body.classList.toggle("panel-backgrounds-off", !ui.panelBackgrounds);
}

function setUIState(patch) {
  Object.assign(ui, patch);
  persistUI();
  applyBodyClasses();
  render();
}

function syncNavOpenUI() {
  const shell = app.querySelector(".workspace-shell");
  const switcher = app.querySelector(".workspace-switcher");
  const toggleInput = app.querySelector("#toggle-switcher");
  const toggleWrap = app.querySelector(".switcher-visibility-toggle");
  if (!shell || !switcher || !toggleInput || !toggleWrap) {
    render();
    return;
  }

  shell.classList.toggle("switcher-collapsed", !ui.navOpen);
  switcher.setAttribute("aria-hidden", ui.navOpen ? "false" : "true");
  toggleInput.checked = ui.navOpen;
  toggleWrap.title = ui.navOpen ? "Hide sections" : "Show sections";
  toggleWrap.setAttribute("aria-label", ui.navOpen ? "Hide sections" : "Show sections");
}

function syncChatCollapsedUI() {
  const shell = app.querySelector(".window-shell");
  const chatRail = app.querySelector(".chat-rail");
  const toggleButton = app.querySelector("#toggle-chat");
  if (!shell || !chatRail || !toggleButton) {
    render();
    return;
  }

  shell.classList.toggle("chat-collapsed", ui.chatCollapsed);
  chatRail.classList.toggle("collapsed", ui.chatCollapsed);
  chatRail.setAttribute("aria-hidden", ui.chatCollapsed ? "true" : "false");
  toggleButton.title = ui.chatCollapsed ? "Show AI panel" : "Hide AI panel";
  toggleButton.setAttribute("aria-label", ui.chatCollapsed ? "Show AI panel" : "Hide AI panel");
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

function currentAppIconPath() {
  return ui.appTheme === "defaultLight"
    ? "/live-media/icons/icon-light.png"
    : "/live-media/icons/icon-dark.png";
}

function statusLabel() {
  if (ui.isCaptureTransitioning) {
    return "Transitioning";
  }
  if (ui.isSniffing) {
    return `Running (${escapeHTML(ui.captureBackendName)})`;
  }
  return `Idle (${escapeHTML(ui.captureBackendName)})`;
}

function packetRowTemplate(packet) {
  const active = packet.index === ui.selectedPacketIndex;
  return `
    <button class="packet-row ${active ? "active" : ""}" data-packet-index="${packet.index}">
      <div class="packet-row-top">
        <span class="packet-number">#${packet.index}</span>
        <span class="protocol-pill">${escapeHTML((packet.protocol || "").toUpperCase())}</span>
        <span class="packet-time">${escapeHTML(`${packet.timestamp_seconds ?? ""}.${packet.timestamp_fraction ?? ""}`)}</span>
      </div>
      <div class="packet-route mono">${escapeHTML(packet.source || "")} <span class="arrow">→</span> ${escapeHTML(packet.destination || "")}</div>
      <div class="packet-info">${escapeHTML(packet.info || "")}</div>
    </button>
  `;
}

function genericRowTemplate(row, type) {
  if (type === "stats") {
    return `
      <div class="generic-row">
        <div class="generic-row-top">
          <span class="protocol-pill">${escapeHTML(row.bucket.toUpperCase())}</span>
          <span class="generic-main mono">${escapeHTML(row.name || "")}</span>
          <span class="generic-stat mono">${escapeHTML(String(row.count ?? 0))}</span>
        </div>
      </div>
    `;
  }

  if (type === "conversations") {
    return `
      <div class="generic-row">
        <div class="generic-row-top">
          <span class="protocol-pill">${escapeHTML((row.protocol || "").toUpperCase())}</span>
          <span class="generic-main mono">${escapeHTML(row.endpoint_a || "")} ↔ ${escapeHTML(row.endpoint_b || "")}</span>
          <span class="generic-stat mono">${escapeHTML(String(row.packets ?? 0))}</span>
        </div>
      </div>
    `;
  }

  if (type === "streams") {
    return `
      <div class="generic-row">
        <div class="generic-row-top">
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
      <div class="generic-row-top">
        <span class="protocol-pill">${escapeHTML((row.protocol || "").toUpperCase())}</span>
        <span class="generic-main">${escapeHTML(row.state || "")}</span>
      </div>
      <div class="detail-line mono">REQ: ${escapeHTML(row.request_summary || "")}</div>
      <div class="detail-line mono secondary">RES: ${escapeHTML(row.response_summary || "")}</div>
    </div>
  `;
}

function liquidPanel(title, body, extraClass = "") {
  return `
    <section class="liquid-panel ${extraClass}">
      <div class="panel-title">${title}</div>
      ${body}
    </section>
  `;
}

function renderPacketsSection() {
  return `
    <div class="section-root packets-root">
      <div class="packets-top-row">
        <section class="surface-panel utility-panel">
          <div class="panel-title">Filter</div>
          <input id="filter-input" class="field-input mono" placeholder="protocol &amp; port" value="${escapeAttribute(ui.filterExpression)}">
        </section>

        <section class="surface-panel utility-panel">
          <div class="capture-header-row">
            <div>
              <div class="panel-title">Capture</div>
              <div class="capture-subtitle">${escapeHTML(ui.captureBackendMessage)}</div>
            </div>
            <div class="capture-state-pill ${ui.isSniffing ? "running" : "idle"}">${statusLabel()}</div>
          </div>
          <select id="capture-interface" class="select">
            ${ui.availableCaptureInterfaces.map((value) => `<option value="${escapeAttribute(value)}" ${value === ui.selectedCaptureInterface ? "selected" : ""}>${escapeHTML(value)}</option>`).join("")}
          </select>
          <div class="capture-actions">
            <label class="neo-toggle-container ${ui.isCaptureTransitioning ? "disabled" : ""}">
              <input id="toggle-capture" class="neo-toggle-input" type="checkbox" ${ui.isSniffing ? "checked" : ""} ${ui.isCaptureTransitioning ? "disabled" : ""}>
              <span class="neo-toggle">
                <span class="neo-track">
                  <span class="neo-background-layer"></span>
                  <span class="neo-grid-layer"></span>
                  <span class="neo-track-highlight"></span>
                  <span class="neo-spectrum-analyzer">
                    <span class="neo-spectrum-bar"></span>
                    <span class="neo-spectrum-bar"></span>
                    <span class="neo-spectrum-bar"></span>
                    <span class="neo-spectrum-bar"></span>
                    <span class="neo-spectrum-bar"></span>
                  </span>
                </span>
                <span class="neo-thumb">
                  <span class="neo-thumb-ring"></span>
                  <span class="neo-thumb-core">
                    <span class="neo-thumb-icon">
                      <span class="neo-thumb-wave"></span>
                      <span class="neo-thumb-pulse"></span>
                    </span>
                  </span>
                </span>
                <span class="neo-gesture-area"></span>
                <span class="neo-interaction-feedback">
                  <span class="neo-ripple"></span>
                  <span class="neo-progress-arc"></span>
                </span>
                <span class="neo-status">
                  <span class="neo-status-indicator">
                    <span class="neo-status-dot"></span>
                    <span class="neo-status-text"></span>
                  </span>
                </span>
              </span>
              <span class="neo-value-display">
                <span class="neo-value-text">${ui.isSniffing ? "Running" : "Standby"}</span>
              </span>
            </label>
            <button id="save-capture" class="action_has has_saved" type="button" aria-label="Save Capture" title="Save Capture" ${ui.capturePath ? "" : "disabled"}>
              <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
                <path data-path="box" d="M5 4.5h11l3 3v12a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-13a2 2 0 0 1 2-2z"></path>
                <path data-path="line-top" d="M8 4.5h7"></path>
                <path data-path="line-bottom" d="M17 20 L17 13 L7 13 L7 20"></path>
              </svg>
            </button>
          </div>
        </section>
      </div>

      <div class="packet-counter-row">
        <span class="packet-counter-label">Packets</span>
        <span class="packet-counter-value">${escapeHTML(String(ui.totalPackets || 0))}</span>
      </div>

      <div class="packets-split-view">
        ${liquidPanel(
          "Packets",
          `<div class="scroll-frame packet-list-frame">${ui.packets.length ? ui.packets.map(packetRowTemplate).join("") : '<div class="empty-state">Open a capture or start sniffing to populate packets.</div>'}</div>`,
          "packet-list-panel"
        )}
        ${liquidPanel(
          "Packet JSON",
          `<div class="json-frame"><pre class="json-content">${escapeHTML(ui.packetJSON)}</pre></div>`,
          "packet-json-panel"
        )}
      </div>
    </div>
  `;
}

function renderGenericSection(title, rows, type) {
  return `
    <div class="section-root single-panel-root">
      ${liquidPanel(
        title,
        `<div class="scroll-frame generic-list">${rows.length ? rows.map((row) => genericRowTemplate(row, type)).join("") : `<div class="empty-state">No ${title.toLowerCase()} available for the current capture.</div>`}</div>`,
        "generic-panel"
      )}
    </div>
  `;
}

function renderProfileSection() {
  return `
    <div class="section-root single-panel-root">
      <section class="liquid-panel profile-panel">
        <div class="settings-heading-block">
          <div class="settings-page-title">Profile</div>
          <div class="settings-page-subtitle">Account identity and local preferences</div>
        </div>
        <div class="profile-placeholder-card">
          <div class="profile-avatar-fallback">${icons.profile}</div>
          <div>
            <div class="profile-title">Local Web Profile</div>
            <div class="detail-line secondary">The browser track is mirroring the native shell first. Auth and cloud profile behavior still belong to the macOS surface.</div>
          </div>
        </div>
      </section>
    </div>
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
  const panelBackgroundOptions = [
    ["on", "On"],
    ["off", "Off"]
  ];

  return `
    <div class="section-root single-panel-root">
      <section class="liquid-panel settings-panel">
        <div class="settings-heading-block">
          <div class="settings-page-title">Settings</div>
        </div>

        <div class="settings-block">
          <div class="settings-block-title">Theme</div>
          <div class="theme-preview-grid">
            ${themes.map(([value, label]) => `
              <button class="theme-preview ${ui.appTheme === value ? "active" : ""}" data-theme-choice="${value}">
                <span class="theme-preview-art theme-preview-${value}"></span>
                <span class="theme-preview-label">${label}</span>
              </button>
            `).join("")}
          </div>
        </div>

        <div class="settings-block">
          <div class="settings-block-title">Font</div>
          <div class="choice-pills-row">
            ${fonts.map(([value, label]) => `<button class="choice-pill ${ui.fontChoice === value ? "active" : ""}" data-font-choice="${value}">${label}</button>`).join("")}
          </div>
        </div>

        <div class="settings-block">
          <div class="settings-block-title">Panel Backgrounds</div>
          <div class="choice-pills-row">
            ${panelBackgroundOptions.map(([value, label]) => `<button class="choice-pill ${ui.panelBackgrounds === (value === "on") ? "active" : ""}" data-panel-backgrounds="${value}">${label}</button>`).join("")}
          </div>
        </div>

        <div class="settings-note">The live web app uses the same Rust capture and analysis backend as the macOS app. Theme, font, and panel background style stay local to this browser.</div>
      </section>
    </div>
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

function renderSectionSwitcher(items) {
  return `
    <nav class="panel-switcher" aria-label="Sections">
      ${items.map(([key, label, iconMarkup]) => `
        <button class="panel-switcher-button ${ui.activeSection === key ? "active" : ""}" data-section="${key}" type="button" aria-pressed="${ui.activeSection === key ? "true" : "false"}">
          <span class="panel-switcher-icon">${iconMarkup}</span>
          <span class="panel-switcher-label">${label}</span>
        </button>
      `).join("")}
    </nav>
  `;
}

function renderHeaderOpenCaptureButton() {
  return `
    <button id="open-capture-header" class="open-file" type="button">
      <span>Open Capture</span>
      <span class="file-wrapper" aria-hidden="true">
        <svg viewBox="0 0 24 24" focusable="false">
          <path d="M14 3.5H7a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8.5z"></path>
          <path d="M14 3.5v5h5"></path>
        </svg>
        <span class="file-front"></span>
      </span>
    </button>
  `;
}

function renderChatPanel() {
  return `
    <aside class="chat-rail ${ui.chatCollapsed ? "collapsed" : ""}" aria-hidden="${ui.chatCollapsed ? "true" : "false"}">
      <div class="chat-rail-inner">
        <section class="liquid-panel chat-model-panel compact-panel">
          <div class="chat-model-row">
            <div>
              <div class="panel-title">Model</div>
              <div class="capture-subtitle">Native AI panel parity pass</div>
            </div>
            <div class="chat-model-tag">Codex</div>
          </div>
        </section>

        <section class="liquid-panel chat-main-panel">
          <div class="chat-panel-header">
            <div class="chat-panel-title">AI Chat</div>
          </div>

          <div class="chat-scroll-area">
            <div class="chat-bubble assistant">The live app is now using the same local icesniff-cli, icesniff-capture-helper, and tshark path resolution as the macOS app.</div>
            <div class="chat-bubble user">Keep the browser shell visually aligned with the native app.</div>
            <div class="chat-bubble assistant">That is the current focus: tighter material, spacing, and hierarchy parity.</div>
          </div>

          <div class="chat-divider"></div>

          <div class="chat-compose-region">
            <div class="chat-status-line">Web AI compose remains a shell while capture and analysis stay on the shared Rust backend.</div>
            <div class="chat-composer-shell">
              <textarea placeholder="Web AI compose surface will follow the macOS panel." disabled></textarea>
            </div>
            <div class="chat-send-row">
              <button class="primary-button" disabled>Send</button>
            </div>
          </div>
        </section>
      </div>
    </aside>
  `;
}

function render() {
  const sections = [
    ["packets", "Packets", icons.packets],
    ["stats", "Stats", icons.stats],
    ["conversations", "Conversations", icons.conversations],
    ["streams", "Streams", icons.streams],
    ["transactions", "Transactions", icons.transactions],
    ["settings", "Settings", icons.settings],
    ["profile", "Profile", icons.profile]
  ];

  app.innerHTML = `
    <div class="window-shell ${ui.chatCollapsed ? "chat-collapsed" : ""}">
      <div class="aurora-orb orb-a"></div>
      <div class="aurora-orb orb-b"></div>
      <div class="aurora-orb orb-c"></div>

      <div class="app-frame">
        <main class="detail-column">
          <div class="detail-stack">
            <header class="detail-header">
              <div class="detail-title-cluster">
                <div class="detail-brand-row">
                  <img class="detail-app-icon" src="${currentAppIconPath()}" alt="IceSniff">
                  <div>
                    <div class="detail-overline">IceSniff Live</div>
                    <div class="detail-title">${escapeHTML(titleForSection(ui.activeSection))}</div>
                  </div>
                  <label class="switcher-visibility-toggle checkBox" title="${ui.navOpen ? "Hide sections" : "Show sections"}" aria-label="${ui.navOpen ? "Hide sections" : "Show sections"}">
                    <input id="toggle-switcher" type="checkbox" ${ui.navOpen ? "checked" : ""}>
                    <div class="transition"></div>
                  </label>
                </div>
              </div>

              <div class="detail-header-actions">
                ${renderHeaderOpenCaptureButton()}
                <div class="status-pill">
                  <span class="status-text">${escapeHTML(ui.statusMessage)}</span>
                  <button id="copy-status" class="header-icon-button" title="Copy status">${icons.copy}</button>
                </div>
              </div>

              <button id="toggle-chat" class="edge-toggle edge-toggle-right" title="${ui.chatCollapsed ? "Show AI panel" : "Hide AI panel"}" aria-label="${ui.chatCollapsed ? "Show AI panel" : "Hide AI panel"}">${icons.sparkles}</button>
            </header>

            <section class="detail-card">
              <div class="detail-card-body">
                <div class="workspace-shell ${ui.navOpen ? "" : "switcher-collapsed"}">
                  <aside class="workspace-switcher" aria-hidden="${ui.navOpen ? "false" : "true"}">
                    ${renderSectionSwitcher(sections)}
                  </aside>
                  <div class="workspace-main">
                    ${renderMainSection()}
                  </div>
                </div>
              </div>
            </section>
          </div>
        </main>

        ${renderChatPanel()}
      </div>
    </div>
  `;

  wireEvents();
}

function wireEvents() {
  app.querySelectorAll("[data-section]").forEach((button) => {
    button.addEventListener("click", () => {
      setUIState({ activeSection: button.dataset.section, navOpen: false });
    });
  });

  app.querySelector("#toggle-switcher")?.addEventListener("change", (event) => {
    ui.navOpen = event.target.checked;
    persistUI();
    syncNavOpenUI();
  });

  app.querySelector("#toggle-chat")?.addEventListener("click", () => {
    ui.chatCollapsed = !ui.chatCollapsed;
    persistUI();
    syncChatCollapsedUI();
  });

  app.querySelector("#copy-status")?.addEventListener("click", async () => {
    try {
      await navigator.clipboard.writeText(ui.statusMessage);
    } catch {}
  });

  app.querySelector("#open-capture-header")?.addEventListener("click", () => {
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

  app.querySelector("#toggle-capture")?.addEventListener("change", () => {
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

  app.querySelectorAll("[data-panel-backgrounds]").forEach((button) => {
    button.addEventListener("click", () => {
      setUIState({ panelBackgrounds: button.dataset.panelBackgrounds === "on" });
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
