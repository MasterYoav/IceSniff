import { createServer } from "node:http";
import { createReadStream, statSync } from "node:fs";
import { promises as fs } from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";
import crypto from "node:crypto";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const APP_ROOT = __dirname;
const PUBLIC_ROOT = path.join(APP_ROOT, "public");
const REPO_ROOT = path.resolve(APP_ROOT, "..", "..");
const MAC_APP_ROOT = path.join(REPO_ROOT, "apps", "macos");
const MAC_WORKSPACE_ROOT = process.env.ICESNIFF_RUST_WORKSPACE_ROOT || path.join(MAC_APP_ROOT, "rust-engine");
const BUNDLED_CLI_ROOT = path.join(MAC_APP_ROOT, "Sources", "IceSniffMac", "Resources", "BundledCLI");
const BUNDLED_TSHARK_ROOT = path.join(
  MAC_APP_ROOT,
  "Sources",
  "IceSniffMac",
  "Resources",
  "BundledTShark",
  "Wireshark.app",
  "Contents",
  "MacOS",
  "tshark"
);
const HOST = "127.0.0.1";
const PORT = Number(process.env.PORT || 4318);
const MAX_UPLOAD_BYTES = 250 * 1024 * 1024;

const state = {
  capturePath: "",
  isSniffing: false,
  isCaptureTransitioning: false,
  captureBackendName: "Unavailable",
  captureBackendMessage: "Live capture backend unavailable.",
  statusMessage: "Choose a capture file to begin.",
  availableCaptureInterfaces: ["en0"],
  selectedCaptureInterface: "en0",
  engineCapabilities: {
    supportsLiveCapture: true,
    capture: {
      interfaceDiscovery: true,
      requiresAdminForLiveCapture: true
    }
  }
};

const liveCapture = {
  child: null,
  stderr: "",
  outputPath: "",
  stopRequested: false
};

const MIME_TYPES = {
  ".html": "text/html; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".png": "image/png",
  ".jpg": "image/jpeg",
  ".jpeg": "image/jpeg",
  ".svg": "image/svg+xml"
};

function sendJSON(response, statusCode, payload) {
  response.writeHead(statusCode, {
    "Content-Type": "application/json; charset=utf-8",
    "Cache-Control": "no-store"
  });
  response.end(JSON.stringify(payload));
}

function sendText(response, statusCode, text) {
  response.writeHead(statusCode, {
    "Content-Type": "text/plain; charset=utf-8",
    "Cache-Control": "no-store"
  });
  response.end(text);
}

async function readRequestBody(request, limitBytes = 2 * 1024 * 1024) {
  return new Promise((resolve, reject) => {
    const chunks = [];
    let total = 0;
    request.on("data", (chunk) => {
      total += chunk.length;
      if (total > limitBytes) {
        reject(new Error(`Request body exceeded ${limitBytes} bytes.`));
        request.destroy();
        return;
      }
      chunks.push(chunk);
    });
    request.on("end", () => resolve(Buffer.concat(chunks)));
    request.on("error", reject);
  });
}

async function readJSONBody(request) {
  const body = await readRequestBody(request);
  if (body.length === 0) {
    return {};
  }
  return JSON.parse(body.toString("utf8"));
}

function withPreferredPath(environment) {
  const merged = { ...environment };
  const existing = (environment.PATH || "").split(":").filter(Boolean);
  const preferred = [
    path.join(os.homedir(), ".cargo", "bin"),
    "/opt/homebrew/bin",
    "/usr/local/bin",
    "/usr/bin",
    "/bin"
  ];
  for (const candidate of preferred) {
    if (!existing.includes(candidate)) {
      existing.push(candidate);
    }
  }
  merged.PATH = existing.join(":");
  return merged;
}

function resolveCargoExecutable() {
  const explicitHome = process.env.CARGO_HOME;
  if (explicitHome) {
    const candidate = path.join(explicitHome, "bin", "cargo");
    if (isExecutable(candidate)) {
      return candidate;
    }
  }

  const candidates = [
    path.join(os.homedir(), ".cargo", "bin", "cargo"),
    "/opt/homebrew/bin/cargo",
    "/usr/local/bin/cargo"
  ];
  return candidates.find(isExecutable) || "cargo";
}

function resolveBundledTShark() {
  if (isExecutable(process.env.ICESNIFF_TSHARK_BIN || "")) {
    return process.env.ICESNIFF_TSHARK_BIN;
  }

  const candidates = [
    BUNDLED_TSHARK_ROOT,
    "/Applications/Wireshark.app/Contents/MacOS/tshark",
    "/opt/homebrew/bin/tshark",
    "/usr/local/bin/tshark",
    "/usr/bin/tshark"
  ];
  return candidates.find(isExecutable) || null;
}

function tempTargetRoots() {
  if (process.env.ICESNIFF_CARGO_TARGET_DIR) {
    return [process.env.ICESNIFF_CARGO_TARGET_DIR];
  }
  return [
    path.join(MAC_WORKSPACE_ROOT, "target"),
    "/tmp/icesniff-macos-release-target",
    path.join(os.tmpdir(), "icesniff-macos-release-target")
  ];
}

function resolveCLICommand() {
  const explicit = process.env.ICESNIFF_CLI_BIN;
  if (isExecutable(explicit || "")) {
    return {
      command: explicit,
      preArgs: [],
      cwd: MAC_WORKSPACE_ROOT
    };
  }

  const candidates = [
    path.join(BUNDLED_CLI_ROOT, "icesniff-cli"),
    ...tempTargetRoots().flatMap((root) => [
      path.join(root, "debug", "icesniff-cli"),
      path.join(root, "release", "icesniff-cli")
    ])
  ];
  const binary = candidates.find(isExecutable);
  if (binary) {
    return {
      command: binary,
      preArgs: [],
      cwd: MAC_WORKSPACE_ROOT
    };
  }

  return {
    command: resolveCargoExecutable(),
    preArgs: ["run", "-q", "-p", "icesniff-cli", "--"],
    cwd: MAC_WORKSPACE_ROOT
  };
}

function resolveHelperCommand() {
  const explicit = process.env.ICESNIFF_CAPTURE_HELPER_BIN;
  if (isExecutable(explicit || "")) {
    return {
      command: explicit,
      preArgs: [],
      cwd: MAC_WORKSPACE_ROOT
    };
  }

  const candidates = [
    path.join(BUNDLED_CLI_ROOT, "icesniff-capture-helper"),
    ...tempTargetRoots().flatMap((root) => [
      path.join(root, "debug", "icesniff-capture-helper"),
      path.join(root, "release", "icesniff-capture-helper")
    ])
  ];
  const binary = candidates.find(isExecutable);
  if (binary) {
    return {
      command: binary,
      preArgs: [],
      cwd: MAC_WORKSPACE_ROOT
    };
  }

  return {
    command: resolveCargoExecutable(),
    preArgs: ["run", "-q", "-p", "icesniff-capture-helper", "--"],
    cwd: MAC_WORKSPACE_ROOT
  };
}

function cliEnvironment() {
  const environment = withPreferredPath(process.env);
  const tshark = resolveBundledTShark();
  if (tshark) {
    environment.ICESNIFF_TSHARK_BIN = tshark;
  }
  return environment;
}

function isExecutable(filePath) {
  try {
    if (!filePath) {
      return false;
    }
    return !!requireExecutableStat(filePath);
  } catch {
    return false;
  }
}

function requireExecutableStat(filePath) {
  const stat = fsSyncStat(filePath);
  if (!stat || !stat.isFile()) {
    return null;
  }
  return stat;
}

function fsSyncStat(filePath) {
  try {
    return statSync(filePath);
  } catch {
    return null;
  }
}

function extractJSONObject(text) {
  const start = text.indexOf("{");
  const end = text.lastIndexOf("}");
  if (start === -1 || end === -1 || end < start) {
    throw new Error(`Backend returned non-JSON output: ${text}`);
  }
  return JSON.parse(text.slice(start, end + 1));
}

function runCommand(command, args, options = {}) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      cwd: options.cwd,
      env: options.env,
      detached: Boolean(options.detached),
      stdio: options.stdio || ["ignore", "pipe", "pipe"]
    });

    let stdout = "";
    let stderr = "";

    if (child.stdout) {
      child.stdout.on("data", (chunk) => {
        stdout += chunk.toString("utf8");
      });
    }

    if (child.stderr) {
      child.stderr.on("data", (chunk) => {
        stderr += chunk.toString("utf8");
      });
    }

    child.on("error", reject);
    child.on("close", (code) => {
      if (code === 0) {
        resolve({ stdout, stderr, code });
      } else {
        const message = (stderr || stdout || `Command failed with exit code ${code}.`).trim();
        reject(new Error(message));
      }
    });
  });
}

async function runCLIJSON(args) {
  const runtime = resolveCLICommand();
  const { stdout } = await runCommand(
    runtime.command,
    [...runtime.preArgs, "--json", ...args],
    {
      cwd: runtime.cwd,
      env: cliEnvironment()
    }
  );
  return extractJSONObject(stdout);
}

async function runCLIText(args) {
  const runtime = resolveCLICommand();
  const { stdout } = await runCommand(
    runtime.command,
    [...runtime.preArgs, ...args],
    {
      cwd: runtime.cwd,
      env: cliEnvironment()
    }
  );
  return stdout.trim();
}

async function listCaptureInterfaces() {
  const runtime = resolveHelperCommand();
  const { stdout } = await runCommand(
    runtime.command,
    [...runtime.preArgs, "list-interfaces"],
    {
      cwd: runtime.cwd,
      env: cliEnvironment()
    }
  );
  return stdout
    .split("\n")
    .map((value) => value.trim())
    .filter(Boolean);
}

function buildPublicState() {
  return {
    capturePath: state.capturePath,
    isSniffing: state.isSniffing,
    isCaptureTransitioning: state.isCaptureTransitioning,
    captureBackendName: state.captureBackendName,
    captureBackendMessage: state.captureBackendMessage,
    statusMessage: state.statusMessage,
    availableCaptureInterfaces: state.availableCaptureInterfaces,
    selectedCaptureInterface: state.selectedCaptureInterface,
    engineCapabilities: state.engineCapabilities
  };
}

async function refreshEngineCapabilities() {
  try {
    state.engineCapabilities = await runCLIJSON(["engine-info"]);
  } catch {
    state.engineCapabilities = {
      supportsLiveCapture: true,
      capture: {
        interfaceDiscovery: true,
        requiresAdminForLiveCapture: true
      }
    };
  }
}

async function refreshCaptureInterfaces() {
  try {
    const interfaces = await listCaptureInterfaces();
    if (interfaces.length > 0) {
      state.availableCaptureInterfaces = interfaces;
      if (!interfaces.includes(state.selectedCaptureInterface)) {
        state.selectedCaptureInterface = interfaces[0];
      }
      state.captureBackendName = "IceSniff Capture";
      state.captureBackendMessage = "Using IceSniff Capture for live capture.";
    }
  } catch (error) {
    state.captureBackendName = "Unavailable";
    state.captureBackendMessage = error.message;
  }
}

function safeFilename(name) {
  return name.replace(/[^a-zA-Z0-9._-]/g, "-");
}

function normalizeSimpleFilterToken(token) {
  if (/^\d+$/.test(token)) {
    return `port=${token}`;
  }

  if (/^[a-z][a-z0-9_.-]*$/i.test(token)) {
    return `protocol=${token.toLowerCase()}`;
  }

  return null;
}

function normalizeFilterExpression(filter) {
  const trimmed = typeof filter === "string" ? filter.trim() : "";
  if (!trimmed) {
    return null;
  }

  if (/[=<>!~()&|]/.test(trimmed)) {
    return trimmed;
  }

  const tokens = trimmed.split(/\s+/).filter(Boolean);
  if (!tokens.length) {
    return null;
  }

  const normalizedTokens = tokens.map(normalizeSimpleFilterToken);
  if (normalizedTokens.every(Boolean)) {
    return normalizedTokens.join(" && ");
  }

  return trimmed;
}

async function uploadCapture(request, response, requestURL) {
  const fileName = safeFilename(requestURL.searchParams.get("name") || "capture.pcap");
  const bytes = await readRequestBody(request, MAX_UPLOAD_BYTES);
  if (bytes.length === 0) {
    throw new Error("Uploaded capture was empty.");
  }

  const tempPath = path.join(os.tmpdir(), `icesniff-upload-${Date.now()}-${fileName}`);
  await fs.writeFile(tempPath, bytes);
  state.capturePath = tempPath;
  state.statusMessage = `Opened ${fileName}.`;
  sendJSON(response, 200, { ok: true, capturePath: tempPath, state: buildPublicState() });
}

async function refreshAnalysis(request, response) {
  if (!state.capturePath) {
    sendJSON(response, 400, { ok: false, message: "No capture is open." });
    return;
  }

  const body = await readJSONBody(request);
  const filter = normalizeFilterExpression(body.filter);
  const limit = String(body.limit || "200");

  try {
    const [inspect, list, stats, conversations, streams, transactions] = await Promise.all([
      runCLIJSON(["inspect", state.capturePath]),
      runCLIJSON(["list", state.capturePath, limit, ...(filter ? ["--filter", filter] : [])]),
      runCLIJSON(["stats", state.capturePath, ...(filter ? ["--filter", filter] : [])]),
      runCLIJSON(["conversations", state.capturePath, ...(filter ? ["--filter", filter] : [])]),
      runCLIJSON(["streams", state.capturePath, ...(filter ? ["--filter", filter] : [])]),
      runCLIJSON(["transactions", state.capturePath, ...(filter ? ["--filter", filter] : [])])
    ]);

    state.statusMessage = state.isSniffing
      ? `Live capture running on ${state.selectedCaptureInterface}.`
      : "Loaded capture successfully.";

    sendJSON(response, 200, {
      ok: true,
      inspect,
      list,
      stats,
      conversations,
      streams,
      transactions,
      state: buildPublicState()
    });
  } catch (error) {
    const transient = state.isSniffing && /(waiting|no packets|capture file|end of file|no such file|empty)/i.test(error.message);
    if (transient) {
      state.statusMessage = `Waiting for packets on ${state.selectedCaptureInterface}...`;
      sendJSON(response, 202, {
        ok: false,
        transient: true,
        message: state.statusMessage,
        state: buildPublicState()
      });
      return;
    }

    state.statusMessage = `Request failed: ${error.message}`;
    sendJSON(response, 500, {
      ok: false,
      message: error.message,
      state: buildPublicState()
    });
  }
}

async function packetDetail(response, index) {
  if (!state.capturePath) {
    sendJSON(response, 400, { ok: false, message: "No capture is open." });
    return;
  }

  try {
    const packet = await runCLIJSON(["show-packet", state.capturePath, String(index)]);
    sendJSON(response, 200, { ok: true, packet });
  } catch (error) {
    sendJSON(response, 500, { ok: false, message: error.message });
  }
}

async function startCapture(request, response) {
  if (liveCapture.child) {
    sendJSON(response, 409, { ok: false, message: "Live capture is already running.", state: buildPublicState() });
    return;
  }

  const body = await readJSONBody(request);
  const selectedInterface = typeof body.interface === "string" && body.interface.trim()
    ? body.interface.trim()
    : state.selectedCaptureInterface;

  if (!selectedInterface) {
    sendJSON(response, 400, { ok: false, message: "No capture interface selected.", state: buildPublicState() });
    return;
  }

  state.isCaptureTransitioning = true;
  const runtime = resolveHelperCommand();
  const outputPath = path.join(os.tmpdir(), `icesniff-live-${Date.now()}-${crypto.randomUUID()}.pcap`);
  const child = spawn(
    runtime.command,
    [...runtime.preArgs, "start", "--interface", selectedInterface, "--output", outputPath],
    {
      cwd: runtime.cwd,
      env: cliEnvironment(),
      stdio: ["ignore", "ignore", "pipe"]
    }
  );

  liveCapture.child = child;
  liveCapture.stderr = "";
  liveCapture.outputPath = outputPath;
  liveCapture.stopRequested = false;

  if (child.stderr) {
    child.stderr.on("data", (chunk) => {
      liveCapture.stderr += chunk.toString("utf8");
    });
  }

  child.on("exit", () => {
    liveCapture.child = null;
    state.isSniffing = false;
    state.isCaptureTransitioning = false;
    if (!liveCapture.stopRequested) {
      const suffix = liveCapture.stderr.trim() ? ` ${liveCapture.stderr.trim()}` : "";
      state.statusMessage = `Live capture exited unexpectedly.${suffix}`.trim();
    }
  });

  child.on("error", (error) => {
    liveCapture.child = null;
    state.isSniffing = false;
    state.isCaptureTransitioning = false;
    state.statusMessage = `Live capture failed: ${error.message}`;
  });

  await new Promise((resolve) => setTimeout(resolve, 350));

  if (child.exitCode !== null) {
    liveCapture.child = null;
    state.isCaptureTransitioning = false;
    state.isSniffing = false;
    throw new Error((liveCapture.stderr || "Live capture helper exited immediately.").trim());
  }

  state.capturePath = outputPath;
  state.selectedCaptureInterface = selectedInterface;
  state.isSniffing = true;
  state.isCaptureTransitioning = false;
  state.captureBackendName = "IceSniff Capture";
  state.captureBackendMessage = "Using IceSniff Capture for live capture.";
  state.statusMessage = `Live capture started on ${selectedInterface}. Waiting for packets...`;

  sendJSON(response, 200, { ok: true, state: buildPublicState() });
}

async function selectCaptureInterface(request, response) {
  const body = await readJSONBody(request);
  const selectedInterface = typeof body.interface === "string" ? body.interface.trim() : "";

  if (!selectedInterface) {
    sendJSON(response, 400, { ok: false, message: "No capture interface selected.", state: buildPublicState() });
    return;
  }

  if (state.isSniffing || state.isCaptureTransitioning) {
    sendJSON(response, 409, { ok: false, message: "Stop live capture before changing the interface.", state: buildPublicState() });
    return;
  }

  if (!state.availableCaptureInterfaces.includes(selectedInterface)) {
    sendJSON(response, 400, { ok: false, message: `Unknown capture interface: ${selectedInterface}`, state: buildPublicState() });
    return;
  }

  state.selectedCaptureInterface = selectedInterface;
  if (!state.isSniffing) {
    state.statusMessage = `Selected capture interface: ${selectedInterface}`;
  }

  sendJSON(response, 200, { ok: true, state: buildPublicState() });
}

async function stopCapture(response) {
  if (!liveCapture.child) {
    sendJSON(response, 409, { ok: false, message: "No live capture is running.", state: buildPublicState() });
    return;
  }

  liveCapture.stopRequested = true;
  const child = liveCapture.child;
  await new Promise((resolve) => {
    const timeout = setTimeout(() => {
      try {
        child.kill("SIGKILL");
      } catch {}
      resolve();
    }, 1200);

    child.once("exit", () => {
      clearTimeout(timeout);
      resolve();
    });

    try {
      child.kill("SIGTERM");
    } catch {
      clearTimeout(timeout);
      resolve();
    }
  });

  liveCapture.child = null;
  state.isSniffing = false;
  state.isCaptureTransitioning = false;
  state.statusMessage = "Live capture stopped.";
  sendJSON(response, 200, { ok: true, state: buildPublicState() });
}

async function saveCapture(response, request) {
  if (!state.capturePath) {
    sendJSON(response, 400, { ok: false, message: "No capture is open." });
    return;
  }

  const body = await readJSONBody(request);
  const filter = normalizeFilterExpression(body.filter);
  const outputPath = path.join(os.tmpdir(), `icesniff-export-${Date.now()}.pcap`);

  try {
    await runCLIText(["save", state.capturePath, outputPath, ...(filter ? ["--filter", filter] : [])]);
    const bytes = await fs.readFile(outputPath);
    response.writeHead(200, {
      "Content-Type": "application/vnd.tcpdump.pcap",
      "Content-Disposition": 'attachment; filename="icesniff-export.pcap"',
      "Cache-Control": "no-store"
    });
    response.end(bytes);
  } catch (error) {
    sendJSON(response, 500, { ok: false, message: error.message });
  }
}

async function serveStatic(response, pathname) {
  let targetPath;
  if (pathname === "/") {
    targetPath = path.join(PUBLIC_ROOT, "index.html");
  } else if (pathname.startsWith("/live-media/")) {
    targetPath = path.join(PUBLIC_ROOT, pathname.replace("/live-media/", "media/"));
  } else if (pathname.startsWith("/media/")) {
    targetPath = path.join(REPO_ROOT, pathname);
  } else {
    targetPath = path.join(PUBLIC_ROOT, pathname);
  }

  const normalized = path.normalize(targetPath);
  if (
    !normalized.startsWith(PUBLIC_ROOT) &&
    !normalized.startsWith(path.join(REPO_ROOT, "media"))
  ) {
    sendText(response, 403, "Forbidden");
    return;
  }

  try {
    const stat = await fs.stat(normalized);
    if (!stat.isFile()) {
      sendText(response, 404, "Not found");
      return;
    }
    response.writeHead(200, {
      "Content-Type": MIME_TYPES[path.extname(normalized)] || "application/octet-stream",
      "Cache-Control": "no-store"
    });
    createReadStream(normalized).pipe(response);
  } catch {
    sendText(response, 404, "Not found");
  }
}

const server = createServer(async (request, response) => {
  try {
    const requestURL = new URL(request.url || "/", `http://${HOST}:${PORT}`);

    if (request.method === "GET" && requestURL.pathname === "/api/state") {
      sendJSON(response, 200, { ok: true, state: buildPublicState() });
      return;
    }

    if (request.method === "POST" && requestURL.pathname === "/api/refresh") {
      await refreshAnalysis(request, response);
      return;
    }

    if (request.method === "GET" && requestURL.pathname.startsWith("/api/packet/")) {
      const packetIndex = Number(requestURL.pathname.split("/").pop());
      await packetDetail(response, packetIndex);
      return;
    }

    if (request.method === "POST" && requestURL.pathname === "/api/capture/start") {
      await startCapture(request, response);
      return;
    }

    if (request.method === "POST" && requestURL.pathname === "/api/capture/interface") {
      await selectCaptureInterface(request, response);
      return;
    }

    if (request.method === "POST" && requestURL.pathname === "/api/capture/stop") {
      await stopCapture(response);
      return;
    }

    if (request.method === "POST" && requestURL.pathname === "/api/captures/upload") {
      await uploadCapture(request, response, requestURL);
      return;
    }

    if (request.method === "POST" && requestURL.pathname === "/api/capture/save") {
      await saveCapture(response, request);
      return;
    }

    if (request.method === "GET" && requestURL.pathname === "/api/capture/interfaces") {
      await refreshCaptureInterfaces();
      sendJSON(response, 200, { ok: true, interfaces: state.availableCaptureInterfaces, state: buildPublicState() });
      return;
    }

    if (request.method !== "GET") {
      sendText(response, 405, "Method not allowed");
      return;
    }

    await serveStatic(response, requestURL.pathname);
  } catch (error) {
    sendJSON(response, 500, {
      ok: false,
      message: error instanceof Error ? error.message : String(error)
    });
  }
});

await refreshEngineCapabilities();
await refreshCaptureInterfaces();

server.listen(PORT, HOST, () => {
  console.log(`IceSniff Live listening on http://${HOST}:${PORT}`);
});
