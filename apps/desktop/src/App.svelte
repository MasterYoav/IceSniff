<script>
  import { onDestroy, onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { open, save } from '@tauri-apps/plugin-dialog';

  let capturePath = '';
  let filterExpression = '';
  let packetLimit = 400;

  let samplePaths = [];
  let recentCapturePaths = [];
  let capture = null;
  let stats = null;
  let packetList = null;
  let packetDetail = null;
  let conversationReport = null;
  let streamReport = null;
  let transactionReport = null;
  let saveReport = null;

  let selectedPacketIndex = null;
  let selectedConversationIndex = null;
  let selectedStreamIndex = null;
  let selectedTransactionIndex = null;
  let activeFieldRange = null;

  let loadingCapture = false;
  let loadingPacket = false;
  let loadingAnalysis = false;
  let loadingSave = false;
  let loadingAnalysisExport = false;
  let loadingLiveCapture = false;
  let loadingLivePreview = false;
  let pollingLiveCapture = false;
  let errorMessage = '';
  let saveMessage = '';
  let analysisExportMessage = '';
  let liveCaptureMessage = '';

  let streamFilterExpression = '';
  let transactionFilterExpression = '';
  let saveOutputPath = '';
  let analysisExportFormat = 'json';
  let captureRuntime = null;
  let captureInterfaces = [];
  let liveCaptureStatus = null;
  let liveCaptureInterface = '';
  let liveCaptureStatusTimer = null;
  let followLiveLatest = true;

  const RECENT_CAPTURE_PATHS_KEY = 'icesniff.desktop.recentCapturePaths';
  const LAST_STATE_KEY = 'icesniff.desktop.lastState';
  const appReadyAt = new Date();

  onMount(async () => {
    const restoredState = readLastState();
    if (restoredState) {
      capturePath = restoredState.capturePath;
      filterExpression = restoredState.filterExpression;
      streamFilterExpression = restoredState.streamFilterExpression;
      transactionFilterExpression = restoredState.transactionFilterExpression;
      saveOutputPath = restoredState.saveOutputPath;
      analysisExportFormat = restoredState.analysisExportFormat;
      packetLimit = restoredState.packetLimit;
    }
    recentCapturePaths = readRecentCapturePaths();

    try {
      const paths = await invoke('sample_capture_paths');
      samplePaths = Array.isArray(paths) ? paths : [];
      if (capturePath) {
        await loadCapture();
      } else if (samplePaths.length > 0) {
        capturePath = samplePaths[0];
        await loadCapture();
      }
    } catch (error) {
      errorMessage = normalizeError(error);
    }

    try {
      await refreshCaptureRuntimeInfo();
      await refreshCaptureInterfaces();
      await refreshLiveCaptureStatus();
    } catch (error) {
      errorMessage = normalizeError(error);
    }
  });

  onDestroy(() => {
    stopLiveCaptureStatusPolling();
  });

  async function invokeJson(command, args = {}) {
    const payload = await invoke(command, args);
    if (typeof payload === 'string') {
      return JSON.parse(payload);
    }
    if (payload && typeof payload === 'object') {
      return payload;
    }
    throw new Error(`unexpected payload from ${command}`);
  }

  async function refreshCaptureRuntimeInfo() {
    captureRuntime = await invokeJson('capture_runtime_info');
  }

  async function refreshCaptureInterfaces() {
    const report = await invokeJson('capture_interfaces');
    captureInterfaces = Array.isArray(report.interfaces) ? report.interfaces : [];
    if (
      captureInterfaces.length > 0 &&
      (!liveCaptureInterface || !captureInterfaces.includes(liveCaptureInterface))
    ) {
      liveCaptureInterface = captureInterfaces[0];
    }
  }

  async function refreshCaptureInterfacesSafe() {
    try {
      await refreshCaptureInterfaces();
    } catch (error) {
      errorMessage = normalizeError(error);
    }
  }

  async function refreshLiveCaptureStatus(managePolling = true) {
    liveCaptureStatus = await invokeJson('capture_status');
    if (!managePolling) {
      return;
    }

    const state = liveCaptureStatus?.state;
    if (state === 'running' || state === 'exited') {
      startLiveCaptureStatusPolling();
    } else {
      stopLiveCaptureStatusPolling();
    }
  }

  async function refreshLiveCaptureStatusSafe() {
    try {
      await refreshLiveCaptureStatus();
    } catch (error) {
      errorMessage = normalizeError(error);
    }
  }

  function startLiveCaptureStatusPolling() {
    if (liveCaptureStatusTimer) {
      return;
    }

    liveCaptureStatusTimer = setInterval(async () => {
      if (pollingLiveCapture) {
        return;
      }
      pollingLiveCapture = true;
      try {
        await refreshLiveCaptureStatus(false);
        if (liveCaptureStatus?.state === 'running' || liveCaptureStatus?.state === 'exited') {
          await refreshLivePacketPreview();
        }
      } catch (error) {
        errorMessage = normalizeError(error);
        stopLiveCaptureStatusPolling();
      } finally {
        pollingLiveCapture = false;
      }
    }, 1200);
  }

  function stopLiveCaptureStatusPolling() {
    if (!liveCaptureStatusTimer) {
      return;
    }
    clearInterval(liveCaptureStatusTimer);
    liveCaptureStatusTimer = null;
  }

  async function startLiveCapture() {
    if (loadingLiveCapture) {
      return;
    }

    loadingLiveCapture = true;
    errorMessage = '';
    liveCaptureMessage = '';

    try {
      const report = await invokeJson('capture_start', {
        interface: normalizeFilter(liveCaptureInterface)
      });
      liveCaptureStatus = report;
      capturePath = report.path;
      if (!saveOutputPath.trim()) {
        saveOutputPath = buildDefaultOutputPath(report.path);
      }
      liveCaptureMessage = `Live capture started on ${report.interface}.`;
      startLiveCaptureStatusPolling();
      await refreshLivePacketPreview();
    } catch (error) {
      errorMessage = normalizeError(error);
    } finally {
      loadingLiveCapture = false;
    }
  }

  async function stopLiveCapture() {
    if (loadingLiveCapture) {
      return;
    }

    loadingLiveCapture = true;
    errorMessage = '';
    liveCaptureMessage = '';

    try {
      const report = await invokeJson('capture_stop');
      liveCaptureMessage = `Live capture stopped and saved to ${report.path}.`;
      capturePath = report.path;
      saveOutputPath = buildDefaultOutputPath(report.path);
      await refreshLiveCaptureStatus();
      await loadCapture();
    } catch (error) {
      errorMessage = normalizeError(error);
    } finally {
      loadingLiveCapture = false;
    }
  }

  async function refreshLivePacketPreview() {
    const livePath = liveCaptureStatus?.path?.trim?.() ?? '';
    if (!livePath || loadingCapture || loadingPacket || loadingLivePreview) {
      return;
    }

    loadingLivePreview = true;
    try {
      const normalizedFilter = normalizeFilter(filterExpression);
      const [captureReport, statsReport, packetReport] = await Promise.all([
        invokeJson('inspect_capture', { path: livePath }),
        invokeJson('capture_stats', {
          path: livePath,
          filter: normalizedFilter
        }),
        invokeJson('list_packets', {
          path: livePath,
          limit: packetLimit,
          filter: normalizedFilter
        })
      ]);

      capture = captureReport;
      stats = statsReport;
      packetList = packetReport;

      const selectedStillExists =
        selectedPacketIndex !== null &&
        packetReport.packets.some((packet) => packet.index === selectedPacketIndex);
      if (!selectedStillExists) {
        selectedPacketIndex = null;
        packetDetail = null;
        activeFieldRange = null;
      }

      if (followLiveLatest && packetReport.packets.length > 0) {
        const lastPacketIndex = packetReport.packets[packetReport.packets.length - 1].index;
        if (selectedPacketIndex !== lastPacketIndex) {
          await selectPacket(lastPacketIndex, {
            suppressErrors: liveCaptureStatus?.state === 'running'
          });
        }
      }
    } catch (error) {
      if (liveCaptureStatus?.state !== 'running') {
        errorMessage = normalizeError(error);
      }
    } finally {
      loadingLivePreview = false;
    }
  }

  async function loadCapture() {
    const trimmedPath = capturePath.trim();
    if (!trimmedPath) {
      errorMessage = 'Enter a capture path.';
      return;
    }

    loadingCapture = true;
    errorMessage = '';
    saveMessage = '';
    analysisExportMessage = '';
    packetDetail = null;
    conversationReport = null;
    streamReport = null;
    transactionReport = null;
    saveReport = null;
    selectedPacketIndex = null;
    selectedConversationIndex = null;
    activeFieldRange = null;

    try {
      const normalizedFilter = normalizeFilter(filterExpression);
      const [captureReport, statsReport, packetReport] = await Promise.all([
        invokeJson('inspect_capture', { path: trimmedPath }),
        invokeJson('capture_stats', {
          path: trimmedPath,
          filter: normalizedFilter
        }),
        invokeJson('list_packets', {
          path: trimmedPath,
          limit: packetLimit,
          filter: normalizedFilter
        })
      ]);

      capture = captureReport;
      stats = statsReport;
      packetList = packetReport;
      pushRecentCapturePath(trimmedPath);
      if (!saveOutputPath.trim()) {
        saveOutputPath = buildDefaultOutputPath(trimmedPath);
      }
      persistLastState();

      if (packetReport.packets.length > 0) {
        await selectPacket(packetReport.packets[0].index);
      }
      await loadAnalysis();
    } catch (error) {
      capture = null;
      stats = null;
      packetList = null;
      conversationReport = null;
      streamReport = null;
      transactionReport = null;
      errorMessage = normalizeError(error);
    } finally {
      loadingCapture = false;
    }
  }

  async function selectPacket(index, options = {}) {
    const suppressErrors = options.suppressErrors === true;
    if (selectedPacketIndex === index || !capturePath.trim()) {
      return;
    }

    loadingPacket = true;
    errorMessage = '';

    try {
      const report = await invokeJson('inspect_packet', {
        path: capturePath.trim(),
        packetIndex: index
      });
      packetDetail = report;
      selectedPacketIndex = index;
      activeFieldRange = null;
    } catch (error) {
      if (!suppressErrors) {
        errorMessage = normalizeError(error);
      }
    } finally {
      loadingPacket = false;
    }
  }

  async function loadAnalysis() {
    const trimmedPath = capturePath.trim();
    if (!trimmedPath) {
      return;
    }

    loadingAnalysis = true;
    errorMessage = '';

    try {
      const normalizedFilter = normalizeFilter(filterExpression);
      const [conversations, streams, transactions] = await Promise.all([
        invokeJson('list_conversations', {
          path: trimmedPath,
          filter: normalizedFilter
        }),
        invokeJson('list_streams', {
          path: trimmedPath,
          filter: normalizedFilter,
          streamFilter: normalizeFilter(streamFilterExpression)
        }),
        invokeJson('list_transactions', {
          path: trimmedPath,
          filter: normalizedFilter,
          transactionFilter: normalizeFilter(transactionFilterExpression)
        })
      ]);
      conversationReport = conversations;
      streamReport = streams;
      transactionReport = transactions;
      selectedConversationIndex =
        conversations.conversations?.length > 0 ? 0 : null;
      selectedStreamIndex = streams.streams?.length > 0 ? 0 : null;
      selectedTransactionIndex =
        transactions.transactions?.length > 0 ? 0 : null;
      persistLastState();
    } catch (error) {
      conversationReport = null;
      streamReport = null;
      transactionReport = null;
      selectedConversationIndex = null;
      selectedStreamIndex = null;
      selectedTransactionIndex = null;
      errorMessage = normalizeError(error);
    } finally {
      loadingAnalysis = false;
    }
  }

  async function runSaveExport() {
    const sourcePath = capturePath.trim();
    const outputPath = saveOutputPath.trim();

    if (!sourcePath || !outputPath) {
      errorMessage = 'Source and output paths are required for save/export.';
      return;
    }

    persistLastState();
    loadingSave = true;
    errorMessage = '';
    saveMessage = '';

    try {
      const report = await invokeJson('save_capture', {
        sourcePath,
        outputPath,
        filter: normalizeFilter(filterExpression),
        streamFilter: normalizeFilter(streamFilterExpression)
      });
      saveReport = report;
      saveMessage = `Wrote ${report.packets_written.toLocaleString()} packets to ${report.output_path}`;
    } catch (error) {
      saveReport = null;
      errorMessage = normalizeError(error);
    } finally {
      loadingSave = false;
    }
  }

  async function runAnalysisExport(kind) {
    const sourcePath = capturePath.trim();
    if (!sourcePath) {
      errorMessage = 'Load a capture before exporting analysis rows.';
      return;
    }

    const normalizedKind = normalizeExportKind(kind);
    const format = analysisExportFormat;
    const defaultPath = buildAnalysisExportPath(sourcePath, normalizedKind, format);

    let outputPath = '';
    try {
      const selected = await save({
        defaultPath,
        filters: [{ name: format.toUpperCase(), extensions: [format] }]
      });
      if (typeof selected !== 'string' || !selected.trim()) {
        return;
      }
      outputPath = selected;
    } catch (error) {
      errorMessage = normalizeError(error);
      return;
    }

    loadingAnalysisExport = true;
    errorMessage = '';
    analysisExportMessage = '';
    persistLastState();

    try {
      if (normalizedKind === 'conversations') {
        analysisExportMessage = await invoke('export_conversations', {
          path: sourcePath,
          outputPath,
          filter: normalizeFilter(filterExpression),
          format
        });
      } else if (normalizedKind === 'streams') {
        analysisExportMessage = await invoke('export_streams', {
          path: sourcePath,
          outputPath,
          filter: normalizeFilter(filterExpression),
          streamFilter: normalizeFilter(streamFilterExpression),
          format
        });
      } else {
        analysisExportMessage = await invoke('export_transactions', {
          path: sourcePath,
          outputPath,
          filter: normalizeFilter(filterExpression),
          transactionFilter: normalizeFilter(transactionFilterExpression),
          format
        });
      }
    } catch (error) {
      analysisExportMessage = '';
      errorMessage = normalizeError(error);
    } finally {
      loadingAnalysisExport = false;
    }
  }

  async function pickCapturePath() {
    try {
      const selected = await open({
        multiple: false,
        directory: false,
        filters: [
          { name: 'Capture Files', extensions: ['pcap', 'pcapng'] },
          { name: 'All Files', extensions: ['*'] }
        ]
      });
      if (typeof selected === 'string' && selected.trim()) {
        capturePath = selected;
        saveOutputPath = buildDefaultOutputPath(selected);
        persistLastState();
      }
    } catch (error) {
      errorMessage = normalizeError(error);
    }
  }

  async function pickSaveOutputPath() {
    try {
      const selected = await save({
        defaultPath: saveOutputPath.trim() || buildDefaultOutputPath(capturePath.trim()),
        filters: [{ name: 'PCAP', extensions: ['pcap'] }]
      });
      if (typeof selected === 'string' && selected.trim()) {
        saveOutputPath = selected;
        persistLastState();
      }
    } catch (error) {
      errorMessage = normalizeError(error);
    }
  }

  function selectStream(index) {
    selectedStreamIndex = index;
    const stream = streamRows[index];
    if (!stream) {
      return;
    }

    const conversationIndex = conversationRows.findIndex((row) =>
      isConversationMatchForStream(row, stream)
    );
    if (conversationIndex >= 0) {
      selectedConversationIndex = conversationIndex;
    }

    const transactionIndex = transactionRows.findIndex((row) =>
      isSameDirectionalEndpoints(
        row.client,
        row.server,
        stream.client,
        stream.server
      )
    );
    if (transactionIndex >= 0) {
      selectedTransactionIndex = transactionIndex;
    }
  }

  function selectConversation(index) {
    selectedConversationIndex = index;
    const conversation = conversationRows[index];
    if (!conversation) {
      return;
    }

    const streamIndex = streamRows.findIndex((row) =>
      isConversationMatchForStream(conversation, row)
    );
    if (streamIndex >= 0) {
      selectedStreamIndex = streamIndex;
    }

    const transactionIndex = transactionRows.findIndex((row) =>
      isConversationMatchForTransaction(conversation, row)
    );
    if (transactionIndex >= 0) {
      selectedTransactionIndex = transactionIndex;
    }
  }

  function selectTransaction(index) {
    selectedTransactionIndex = index;
    const transaction = transactionRows[index];
    if (!transaction) {
      return;
    }

    const streamIndex = streamRows.findIndex((row) =>
      isSameDirectionalEndpoints(
        row.client,
        row.server,
        transaction.client,
        transaction.server
      )
    );
    if (streamIndex >= 0) {
      selectedStreamIndex = streamIndex;
    }

    const conversationIndex = conversationRows.findIndex((row) =>
      isConversationMatchForTransaction(row, transaction)
    );
    if (conversationIndex >= 0) {
      selectedConversationIndex = conversationIndex;
    }
  }

  function normalizeFilter(value) {
    const trimmed = value.trim();
    return trimmed ? trimmed : null;
  }

  async function focusConversationSelection() {
    if (!selectedConversationRow || loadingCapture) {
      return;
    }
    const expression = buildEndpointFocusExpression(
      selectedConversationRow.endpoint_a,
      selectedConversationRow.endpoint_b,
      selectedConversationRow.protocol
    );
    filterExpression = expression;
    await loadCapture();
  }

  async function focusStreamSelection() {
    if (!selectedStreamRow || loadingCapture) {
      return;
    }
    const expression = buildEndpointFocusExpression(
      selectedStreamRow.client,
      selectedStreamRow.server,
      selectedStreamRow.protocol
    );
    filterExpression = expression;
    await loadCapture();
  }

  async function focusTransactionSelection() {
    if (!selectedTransactionRow || loadingCapture) {
      return;
    }
    const expression = buildEndpointFocusExpression(
      selectedTransactionRow.client,
      selectedTransactionRow.server,
      selectedTransactionRow.protocol
    );
    filterExpression = expression;
    await loadCapture();
  }

  function buildEndpointFocusExpression(endpointA, endpointB, protocol) {
    const tokenA = normalizeFilterToken(endpointA);
    const tokenB = normalizeFilterToken(endpointB);
    const protocolToken = normalizeFilterToken(protocol);
    const endpointsClause =
      tokenA && tokenB
        ? `(endpoint=${tokenA} || endpoint=${tokenB})`
        : tokenA
          ? `endpoint=${tokenA}`
          : tokenB
            ? `endpoint=${tokenB}`
            : '';

    if (endpointsClause && protocolToken) {
      return `protocol=${protocolToken} && ${endpointsClause}`;
    }
    if (endpointsClause) {
      return endpointsClause;
    }
    if (protocolToken) {
      return `protocol=${protocolToken}`;
    }
    return filterExpression;
  }

  function normalizeFilterToken(value) {
    if (typeof value !== 'string') {
      return '';
    }
    return value.trim().replace(/\s+/g, '');
  }

  function isSameDirectionalEndpoints(
    leftClient,
    leftServer,
    rightClient,
    rightServer
  ) {
    return (
      normalizeFilterToken(leftClient) === normalizeFilterToken(rightClient) &&
      normalizeFilterToken(leftServer) === normalizeFilterToken(rightServer)
    );
  }

  function isBidirectionalEndpointPair(leftA, leftB, rightA, rightB) {
    const leftFirst = normalizeFilterToken(leftA);
    const leftSecond = normalizeFilterToken(leftB);
    const rightFirst = normalizeFilterToken(rightA);
    const rightSecond = normalizeFilterToken(rightB);
    return (
      (leftFirst === rightFirst && leftSecond === rightSecond) ||
      (leftFirst === rightSecond && leftSecond === rightFirst)
    );
  }

  function isConversationMatchForStream(conversation, stream) {
    return isBidirectionalEndpointPair(
      conversation.endpoint_a,
      conversation.endpoint_b,
      stream.client,
      stream.server
    );
  }

  function isConversationMatchForTransaction(conversation, transaction) {
    return isBidirectionalEndpointPair(
      conversation.endpoint_a,
      conversation.endpoint_b,
      transaction.client,
      transaction.server
    );
  }

  function normalizeExportKind(value) {
    if (value === 'conversations' || value === 'streams' || value === 'transactions') {
      return value;
    }
    return 'conversations';
  }

  function buildDefaultOutputPath(path) {
    const lower = path.toLowerCase();
    if (lower.endsWith('.pcapng')) {
      return `${path.slice(0, -7)}-filtered.pcap`;
    }
    if (lower.endsWith('.pcap')) {
      return `${path.slice(0, -5)}-filtered.pcap`;
    }
    return `${path}.filtered.pcap`;
  }

  function buildAnalysisExportPath(path, kind, format) {
    const extension = format === 'csv' ? 'csv' : 'json';
    const base = stripCaptureExtension(path);
    return `${base}-${kind}.${extension}`;
  }

  function stripCaptureExtension(path) {
    const lower = path.toLowerCase();
    if (lower.endsWith('.pcapng')) {
      return path.slice(0, -7);
    }
    if (lower.endsWith('.pcap')) {
      return path.slice(0, -5);
    }
    return path;
  }

  function readRecentCapturePaths() {
    try {
      const raw = window.localStorage.getItem(RECENT_CAPTURE_PATHS_KEY);
      if (!raw) {
        return [];
      }
      const parsed = JSON.parse(raw);
      if (!Array.isArray(parsed)) {
        return [];
      }
      return parsed.filter((entry) => typeof entry === 'string').slice(0, 8);
    } catch {
      return [];
    }
  }

  function pushRecentCapturePath(path) {
    const trimmed = path.trim();
    if (!trimmed) {
      return;
    }
    recentCapturePaths = [
      trimmed,
      ...recentCapturePaths.filter((entry) => entry !== trimmed)
    ].slice(0, 8);
    window.localStorage.setItem(
      RECENT_CAPTURE_PATHS_KEY,
      JSON.stringify(recentCapturePaths)
    );
  }

  function readLastState() {
    try {
      const raw = window.localStorage.getItem(LAST_STATE_KEY);
      if (!raw) {
        return null;
      }
      const parsed = JSON.parse(raw);
      if (!parsed || typeof parsed !== 'object') {
        return null;
      }
      return {
        capturePath:
          typeof parsed.capturePath === 'string' ? parsed.capturePath : '',
        filterExpression:
          typeof parsed.filterExpression === 'string'
            ? parsed.filterExpression
            : '',
        streamFilterExpression:
          typeof parsed.streamFilterExpression === 'string'
            ? parsed.streamFilterExpression
            : '',
        transactionFilterExpression:
          typeof parsed.transactionFilterExpression === 'string'
            ? parsed.transactionFilterExpression
            : '',
        saveOutputPath:
          typeof parsed.saveOutputPath === 'string' ? parsed.saveOutputPath : '',
        analysisExportFormat:
          parsed.analysisExportFormat === 'csv' ? 'csv' : 'json',
        packetLimit:
          typeof parsed.packetLimit === 'number' && parsed.packetLimit > 0
            ? parsed.packetLimit
            : 400
      };
    } catch {
      return null;
    }
  }

  function persistLastState() {
    window.localStorage.setItem(
      LAST_STATE_KEY,
      JSON.stringify({
        capturePath,
        filterExpression,
        streamFilterExpression,
        transactionFilterExpression,
        saveOutputPath,
        analysisExportFormat,
        packetLimit
      })
    );
  }

  function normalizeError(error) {
    if (error instanceof Error) {
      return error.message;
    }
    if (typeof error === 'string') {
      return error;
    }
    return JSON.stringify(error);
  }

  function formatTimestamp(packet) {
    if (!packet) {
      return 'n/a';
    }
    const digits = packet.timestamp_precision === 'nanoseconds' ? 9 : 6;
    return `${packet.timestamp_seconds}.${String(packet.timestamp_fraction).padStart(digits, '0')}`;
  }

  function formatBytes(value) {
    if (typeof value !== 'number') {
      return '0 B';
    }

    if (value < 1024) {
      return `${value} B`;
    }

    if (value < 1024 * 1024) {
      return `${(value / 1024).toFixed(1)} KB`;
    }

    return `${(value / (1024 * 1024)).toFixed(1)} MB`;
  }

  function flattenFields(fields, depth = 0, output = []) {
    for (const field of fields) {
      output.push({
        id: `${depth}-${field.name}-${field.value}-${output.length}`,
        depth,
        name: field.name,
        value: field.value,
        range: field.byte_range
      });
      if (field.children?.length) {
        flattenFields(field.children, depth + 1, output);
      }
    }
    return output;
  }

  function rangeLabel(range) {
    if (!range) {
      return 'byte range n/a';
    }
    return `bytes ${range.start}-${range.end - 1}`;
  }

  function toHex(value) {
    return value.toString(16).padStart(2, '0').toUpperCase();
  }

  function toAscii(value) {
    return value >= 32 && value <= 126 ? String.fromCharCode(value) : '.';
  }

  function buildHexRows(rawBytes, activeRange) {
    const rows = [];

    for (let offset = 0; offset < rawBytes.length; offset += 16) {
      const segment = rawBytes.slice(offset, offset + 16);
      rows.push({
        offset,
        bytes: segment.map((value, index) => {
          const absoluteIndex = offset + index;
          const highlighted =
            activeRange &&
            absoluteIndex >= activeRange.start &&
            absoluteIndex < activeRange.end;
          return {
            value,
            hex: toHex(value),
            ascii: toAscii(value),
            highlighted
          };
        })
      });
    }

    return rows;
  }

  $: statsSummary = stats
    ? [
        { label: 'Packets', value: stats.total_packets.toLocaleString() },
        { label: 'Captured', value: formatBytes(stats.total_captured_bytes) },
        {
          label: 'Average',
          value: `${stats.average_captured_bytes.toLocaleString()} B`
        }
      ]
    : [];

  $: flatFields = packetDetail?.packet?.fields
    ? flattenFields(packetDetail.packet.fields)
    : [];

  $: hexRows = packetDetail?.packet?.raw_bytes
    ? buildHexRows(packetDetail.packet.raw_bytes, activeFieldRange)
    : [];

  $: streamRows = streamReport?.streams ?? [];
  $: conversationRows = conversationReport?.conversations ?? [];
  $: transactionRows = transactionReport?.transactions ?? [];
  $: liveCaptureState = liveCaptureStatus?.state ?? 'idle';
  $: liveCaptureActive = liveCaptureState === 'running' || liveCaptureState === 'exited';
  $: selectedConversationRow =
    selectedConversationIndex !== null
      ? conversationRows[selectedConversationIndex] ?? null
      : null;
  $: selectedStreamRow =
    selectedStreamIndex !== null ? streamRows[selectedStreamIndex] ?? null : null;
  $: selectedTransactionRow =
    selectedTransactionIndex !== null
      ? transactionRows[selectedTransactionIndex] ?? null
      : null;
</script>

<div class="shell">
  <header class="topbar reveal">
    <div>
      <p class="eyebrow">IceSniff Desktop Prototype</p>
      <h1>Packet Workflow Slice</h1>
      <p class="subline">
        Fast prototype mode. Shared Rust services, thin UI shell.
      </p>
    </div>
    <div class="status-card">
      <p>Session started</p>
      <strong>{appReadyAt.toLocaleTimeString()}</strong>
    </div>
  </header>

  <section class="controls reveal reveal-delay-1">
    <label>
      Capture Path
      <input
        type="text"
        bind:value={capturePath}
        placeholder="/absolute/path/to/file.pcap"
        autocomplete="off"
      />
    </label>

    <label>
      Filter
      <input
        type="text"
        bind:value={filterExpression}
        placeholder="protocol=http && host~=example"
        autocomplete="off"
      />
    </label>

    <label class="limit-field">
      Max Rows
      <input type="number" min="1" max="5000" bind:value={packetLimit} />
    </label>

    <button class="secondary" on:click={pickCapturePath} disabled={loadingCapture}>
      Browse...
    </button>

    <button class="primary" on:click={loadCapture} disabled={loadingCapture}>
      {loadingCapture ? 'Loading...' : 'Load Capture'}
    </button>
  </section>

  <section class="live-capture reveal reveal-delay-2">
    <label>
      Live Capture Interface
      <select bind:value={liveCaptureInterface} disabled={loadingLiveCapture || liveCaptureActive}>
        <option value="">auto/default</option>
        {#each captureInterfaces as iface}
          <option value={iface}>{iface}</option>
        {/each}
      </select>
    </label>

    <button
      class="secondary"
      on:click={startLiveCapture}
      disabled={loadingLiveCapture || liveCaptureActive}
    >
      {loadingLiveCapture && liveCaptureState === 'idle'
        ? 'Starting...'
        : 'Start Live Capture'}
    </button>

    <button
      class="accent"
      on:click={stopLiveCapture}
      disabled={loadingLiveCapture || !liveCaptureActive}
    >
      {loadingLiveCapture && liveCaptureActive ? 'Stopping...' : 'Stop + Load'}
    </button>

    <button
      class="secondary"
      on:click={refreshCaptureInterfacesSafe}
      disabled={loadingLiveCapture || liveCaptureActive}
    >
      Refresh Interfaces
    </button>

    <button class="secondary" on:click={refreshLiveCaptureStatusSafe} disabled={loadingLiveCapture}>
      Refresh Status
    </button>

    <label class="live-follow">
      <span>Follow Latest Packet</span>
      <input type="checkbox" bind:checked={followLiveLatest} />
    </label>

    <div class="live-runtime">
      <p><strong>State:</strong> {liveCaptureState}</p>
      <p><strong>Live preview:</strong> {loadingLivePreview ? 'refreshing...' : 'idle'}</p>
      <p><strong>Backend:</strong> {captureRuntime?.backend ?? liveCaptureStatus?.backend ?? 'n/a'}</p>
      <p class="mono"><strong>Tool:</strong> {captureRuntime?.tool_path ?? liveCaptureStatus?.tool_path ?? 'n/a'}</p>
      {#if liveCaptureStatus?.interface}
        <p><strong>Interface:</strong> {liveCaptureStatus.interface}</p>
      {/if}
      {#if liveCaptureStatus?.path}
        <p class="mono"><strong>Capture path:</strong> {liveCaptureStatus.path}</p>
      {/if}
    </div>

    {#if liveCaptureMessage}
      <p class="save-ok">{liveCaptureMessage}</p>
    {/if}
  </section>

  {#if samplePaths.length > 0}
    <section class="samples reveal reveal-delay-2">
      <span>Quick samples</span>
      {#each samplePaths as sample}
        <button
          on:click={async () => {
            capturePath = sample;
            await loadCapture();
          }}
        >
          {sample.split('/').pop()}
        </button>
      {/each}
    </section>
  {/if}

  {#if recentCapturePaths.length > 0}
    <section class="recent reveal reveal-delay-2">
      <span>Recent captures</span>
      {#each recentCapturePaths as recentPath}
        <button
          title={recentPath}
          on:click={async () => {
            capturePath = recentPath;
            if (!saveOutputPath.trim()) {
              saveOutputPath = buildDefaultOutputPath(recentPath);
            }
            await loadCapture();
          }}
        >
          {recentPath.split('/').pop()}
        </button>
      {/each}
    </section>
  {/if}

  {#if errorMessage}
    <section class="error-banner">
      <strong>Request failed:</strong> {errorMessage}
    </section>
  {/if}

  <section class="stats reveal reveal-delay-2">
    {#if capture}
      <div class="capture-meta">
        <p>{capture.path}</p>
        <span>{capture.format.toUpperCase()}</span>
      </div>
    {/if}

    {#if statsSummary.length > 0}
      {#each statsSummary as entry}
        <article>
          <p>{entry.label}</p>
          <strong>{entry.value}</strong>
        </article>
      {/each}
    {/if}
  </section>

  <section class="workspace reveal reveal-delay-3">
    <article class="panel">
      <div class="panel-header">
        <h2>Packets</h2>
        {#if packetList}
          <p>
            showing {packetList.packets_shown.toLocaleString()} of {packetList.total_packets.toLocaleString()}
          </p>
        {/if}
      </div>

      <div class="table-wrap">
        {#if packetList?.packets?.length}
          <table>
            <thead>
              <tr>
                <th>#</th>
                <th>Time</th>
                <th>Source</th>
                <th>Destination</th>
                <th>Protocol</th>
                <th>Info</th>
              </tr>
            </thead>
            <tbody>
              {#each packetList.packets as row}
                <tr
                  class:selected={selectedPacketIndex === row.index}
                  on:click={() => selectPacket(row.index)}
                >
                  <td>{row.index}</td>
                  <td>{formatTimestamp(row)}</td>
                  <td title={row.source}>{row.source}</td>
                  <td title={row.destination}>{row.destination}</td>
                  <td class="mono">{row.protocol}</td>
                  <td title={row.info}>{row.info}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        {:else}
          <p class="empty">No packets loaded yet.</p>
        {/if}
      </div>
    </article>

    <article class="panel details">
      <div class="panel-header">
        <h2>Packet Detail</h2>
        {#if loadingPacket}
          <p>Loading packet...</p>
        {:else if packetDetail}
          <p>packet #{packetDetail.packet.index}</p>
        {/if}
      </div>

      {#if packetDetail}
        <div class="packet-summary">
          <div>
            <p>Timestamp</p>
            <strong>{formatTimestamp(packetDetail.packet)}</strong>
          </div>
          <div>
            <p>Captured</p>
            <strong>{packetDetail.packet.captured_length} B</strong>
          </div>
          <div>
            <p>Original</p>
            <strong>{packetDetail.packet.original_length} B</strong>
          </div>
        </div>

        <div class="layer-grid">
          <article>
            <p>Link</p>
            <pre>{JSON.stringify(packetDetail.packet.link, null, 2)}</pre>
          </article>
          <article>
            <p>Network</p>
            <pre>{JSON.stringify(packetDetail.packet.network, null, 2)}</pre>
          </article>
          <article>
            <p>Transport</p>
            <pre>{JSON.stringify(packetDetail.packet.transport, null, 2)}</pre>
          </article>
          <article>
            <p>Application</p>
            <pre>{JSON.stringify(packetDetail.packet.application, null, 2)}</pre>
          </article>
        </div>

        <div class="field-pane">
          <h3>Decoded Fields</h3>
          {#if flatFields.length > 0}
            <div class="field-list">
              {#each flatFields as field}
                <button
                  class:active={
                    activeFieldRange &&
                    field.range &&
                    activeFieldRange.start === field.range.start &&
                    activeFieldRange.end === field.range.end
                  }
                  style={`padding-left: ${12 + field.depth * 18}px`}
                  on:click={() => {
                    activeFieldRange = field.range;
                  }}
                >
                  <strong>{field.name}</strong>
                  <span>{field.value}</span>
                  <em>{rangeLabel(field.range)}</em>
                </button>
              {/each}
            </div>
          {:else}
            <p class="empty">No decoded fields available.</p>
          {/if}
        </div>

        <div class="hex-pane">
          <h3>Bytes / Hex</h3>
          {#if hexRows.length > 0}
            <div class="hex-grid">
              {#each hexRows as row}
                <div class="hex-row">
                  <span class="offset">{row.offset.toString(16).padStart(4, '0')}</span>
                  <div class="hex-bytes">
                    {#each row.bytes as cell}
                      <span class:highlight={cell.highlighted}>{cell.hex}</span>
                    {/each}
                  </div>
                  <div class="ascii-bytes">
                    {#each row.bytes as cell}
                      <span class:highlight={cell.highlighted}>{cell.ascii}</span>
                    {/each}
                  </div>
                </div>
              {/each}
            </div>
          {:else}
            <p class="empty">No bytes available.</p>
          {/if}
        </div>
      {:else}
        <p class="empty">Select a packet to inspect decoded fields and byte ranges.</p>
      {/if}
    </article>
  </section>

  <section class="analysis-controls reveal reveal-delay-3">
    <label>
      Stream Filter
      <input
        type="text"
        bind:value={streamFilterExpression}
        placeholder="session.has_pipelining=true"
        autocomplete="off"
      />
    </label>
    <label>
      Transaction Filter
      <input
        type="text"
        bind:value={transactionFilterExpression}
        placeholder="http.status>=400"
        autocomplete="off"
      />
    </label>
    <button class="secondary" on:click={loadAnalysis} disabled={loadingAnalysis || loadingCapture}>
      {loadingAnalysis ? 'Refreshing...' : 'Refresh Analysis'}
    </button>
  </section>

  <section class="analysis-export reveal reveal-delay-3">
    <label>
      Analysis Export Format
      <select
        bind:value={analysisExportFormat}
        on:change={() => {
          persistLastState();
        }}
      >
        <option value="json">JSON</option>
        <option value="csv">CSV</option>
      </select>
    </label>
    <button
      class="secondary"
      on:click={() => runAnalysisExport('conversations')}
      disabled={loadingAnalysisExport || loadingCapture}
    >
      Export Conversations
    </button>
    <button
      class="secondary"
      on:click={() => runAnalysisExport('streams')}
      disabled={loadingAnalysisExport || loadingCapture}
    >
      Export Streams
    </button>
    <button
      class="secondary"
      on:click={() => runAnalysisExport('transactions')}
      disabled={loadingAnalysisExport || loadingCapture}
    >
      Export Transactions
    </button>
    {#if analysisExportMessage}
      <p class="save-ok">{analysisExportMessage}</p>
    {/if}
  </section>

  <section class="conversations-grid reveal reveal-delay-3">
    <article class="panel analysis-panel">
      <div class="panel-header">
        <h2>Conversations</h2>
        {#if conversationReport}
          <p>{conversationReport.total_conversations.toLocaleString()} rows</p>
        {/if}
      </div>

      <div class="table-wrap">
        {#if conversationRows.length > 0}
          <table>
            <thead>
              <tr>
                <th>Service</th>
                <th>Protocol</th>
                <th>Endpoint A</th>
                <th>Endpoint B</th>
                <th>Packets</th>
                <th>Req</th>
                <th>Res</th>
              </tr>
            </thead>
            <tbody>
              {#each conversationRows as row, rowIndex}
                <tr
                  class:selected={selectedConversationIndex === rowIndex}
                  on:click={() => selectConversation(rowIndex)}
                >
                  <td>{row.service}</td>
                  <td class="mono">{row.protocol}</td>
                  <td title={row.endpoint_a}>{row.endpoint_a}</td>
                  <td title={row.endpoint_b}>{row.endpoint_b}</td>
                  <td>{row.packets}</td>
                  <td>{row.request_count}</td>
                  <td>{row.response_count}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        {:else}
          <p class="empty">No conversations for current filters.</p>
        {/if}
      </div>

      {#if selectedConversationRow}
        <div class="analysis-detail">
          <h3>Selected Conversation Detail</h3>
          <button
            class="tertiary"
            on:click={focusConversationSelection}
            disabled={loadingCapture}
          >
            Focus Capture View
          </button>
          <div class="analysis-metrics">
            <article>
              <p>Packets</p>
              <strong>{selectedConversationRow.packets}</strong>
            </article>
            <article>
              <p>A to B</p>
              <strong>{selectedConversationRow.packets_a_to_b}</strong>
            </article>
            <article>
              <p>B to A</p>
              <strong>{selectedConversationRow.packets_b_to_a}</strong>
            </article>
            <article>
              <p>Bytes</p>
              <strong>{formatBytes(selectedConversationRow.total_captured_bytes)}</strong>
            </article>
          </div>
          <p class="detail-line">
            <strong>Endpoints:</strong> {selectedConversationRow.endpoint_a} {" <-> "}
            {selectedConversationRow.endpoint_b}
          </p>
          <p class="detail-line">
            <strong>Requests:</strong> {selectedConversationRow.request_count} |
            <strong>Responses:</strong> {selectedConversationRow.response_count}
          </p>
          <p class="detail-line">
            <strong>Packet index range:</strong> {selectedConversationRow.first_packet_index}-
            {selectedConversationRow.last_packet_index}
          </p>
        </div>
      {/if}
    </article>
  </section>

  <section class="analysis-grid reveal reveal-delay-3">
    <article class="panel analysis-panel">
      <div class="panel-header">
        <h2>Streams</h2>
        {#if streamReport}
          <p>{streamReport.total_streams.toLocaleString()} rows</p>
        {/if}
      </div>

      <div class="table-wrap">
        {#if streamRows.length > 0}
          <table>
            <thead>
              <tr>
                <th>Service</th>
                <th>Protocol</th>
                <th>Client</th>
                <th>Server</th>
                <th>Packets</th>
                <th>Matched</th>
                <th>TLS</th>
                <th>State</th>
              </tr>
            </thead>
            <tbody>
              {#each streamRows as row, rowIndex}
                <tr
                  class:selected={selectedStreamIndex === rowIndex}
                  on:click={() => selectStream(rowIndex)}
                >
                  <td>{row.service}</td>
                  <td class="mono">{row.protocol}</td>
                  <td title={row.client}>{row.client}</td>
                  <td title={row.server}>{row.server}</td>
                  <td>{row.packets}</td>
                  <td>{row.matched_transactions}</td>
                  <td>{row.tls_handshake_state}</td>
                  <td>{row.session_state}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        {:else}
          <p class="empty">No stream rows for current filters.</p>
        {/if}
      </div>

      {#if selectedStreamRow}
        <div class="analysis-detail">
          <h3>Selected Stream Detail</h3>
          <button
            class="tertiary"
            on:click={focusStreamSelection}
            disabled={loadingCapture}
          >
            Focus Capture View
          </button>
          <div class="analysis-metrics">
            <article>
              <p>Packets</p>
              <strong>{selectedStreamRow.packets}</strong>
            </article>
            <article>
              <p>Requests</p>
              <strong>{selectedStreamRow.request_count}</strong>
            </article>
            <article>
              <p>Responses</p>
              <strong>{selectedStreamRow.response_count}</strong>
            </article>
            <article>
              <p>TLS Alerts</p>
              <strong>{selectedStreamRow.tls_alert_count}</strong>
            </article>
          </div>
          <p class="detail-line">
            <strong>Endpoints:</strong> {selectedStreamRow.client} -> {selectedStreamRow.server}
          </p>
          <p class="detail-line">
            <strong>Session:</strong> {selectedStreamRow.session_state} | <strong>TLS:</strong>
            {selectedStreamRow.tls_handshake_state}
          </p>
          <div class="detail-columns">
            <article>
              <p>Timeline</p>
              {#if selectedStreamRow.transaction_timeline.length > 0}
                <ul class="compact-list">
                  {#each selectedStreamRow.transaction_timeline as event}
                    <li>{event}</li>
                  {/each}
                </ul>
              {:else}
                <p class="empty">No timeline entries.</p>
              {/if}
            </article>
            <article>
              <p>Notes</p>
              {#if selectedStreamRow.notes.length > 0}
                <ul class="compact-list">
                  {#each selectedStreamRow.notes as note}
                    <li>{note}</li>
                  {/each}
                </ul>
              {:else}
                <p class="empty">No notes.</p>
              {/if}
            </article>
          </div>
        </div>
      {/if}
    </article>

    <article class="panel analysis-panel">
      <div class="panel-header">
        <h2>Transactions</h2>
        {#if transactionReport}
          <p>{transactionReport.total_transactions.toLocaleString()} rows</p>
        {/if}
      </div>

      <div class="table-wrap">
        {#if transactionRows.length > 0}
          <table>
            <thead>
              <tr>
                <th>Seq</th>
                <th>Service</th>
                <th>Client</th>
                <th>Server</th>
                <th>Request</th>
                <th>Response</th>
                <th>State</th>
              </tr>
            </thead>
            <tbody>
              {#each transactionRows as row, rowIndex}
                <tr
                  class:selected={selectedTransactionIndex === rowIndex}
                  on:click={() => selectTransaction(rowIndex)}
                >
                  <td>{row.sequence}</td>
                  <td>{row.service}</td>
                  <td title={row.client}>{row.client}</td>
                  <td title={row.server}>{row.server}</td>
                  <td title={row.request_summary}>{row.request_summary}</td>
                  <td title={row.response_summary}>{row.response_summary}</td>
                  <td>{row.state}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        {:else}
          <p class="empty">No transaction rows for current filters.</p>
        {/if}
      </div>

      {#if selectedTransactionRow}
        <div class="analysis-detail">
          <h3>Selected Transaction Detail</h3>
          <button
            class="tertiary"
            on:click={focusTransactionSelection}
            disabled={loadingCapture}
          >
            Focus Capture View
          </button>
          <p class="detail-line">
            <strong>Flow:</strong> {selectedTransactionRow.client} -> {selectedTransactionRow.server}
          </p>
          <p class="detail-line">
            <strong>Request:</strong> {selectedTransactionRow.request_summary}
          </p>
          <p class="detail-line">
            <strong>Response:</strong> {selectedTransactionRow.response_summary}
          </p>
          <p class="detail-line">
            <strong>State:</strong> {selectedTransactionRow.state}
          </p>
          <div class="detail-columns">
            <article>
              <p>Request Details</p>
              {#if selectedTransactionRow.request_details.length > 0}
                <ul class="compact-list">
                  {#each selectedTransactionRow.request_details as detail}
                    <li>{detail.key}: {detail.value}</li>
                  {/each}
                </ul>
              {:else}
                <p class="empty">No request details.</p>
              {/if}
            </article>
            <article>
              <p>Response Details</p>
              {#if selectedTransactionRow.response_details.length > 0}
                <ul class="compact-list">
                  {#each selectedTransactionRow.response_details as detail}
                    <li>{detail.key}: {detail.value}</li>
                  {/each}
                </ul>
              {:else}
                <p class="empty">No response details.</p>
              {/if}
            </article>
          </div>
          <article>
            <p>Notes</p>
            {#if selectedTransactionRow.notes.length > 0}
              <ul class="compact-list">
                {#each selectedTransactionRow.notes as note}
                  <li>{note}</li>
                {/each}
              </ul>
            {:else}
              <p class="empty">No notes.</p>
            {/if}
          </article>
        </div>
      {/if}
    </article>
  </section>

  <section class="save-panel reveal reveal-delay-3">
    <label>
      Export Output Path
      <input
        type="text"
        bind:value={saveOutputPath}
        placeholder="/absolute/path/export-filtered.pcap"
        autocomplete="off"
      />
    </label>
    <button class="secondary" on:click={pickSaveOutputPath} disabled={loadingSave || loadingCapture}>
      Browse Output
    </button>
    <button class="accent" on:click={runSaveExport} disabled={loadingSave || loadingCapture}>
      {loadingSave ? 'Saving...' : 'Save Filtered Capture'}
    </button>
    {#if saveMessage}
      <p class="save-ok">{saveMessage}</p>
    {/if}
    {#if saveReport}
      <p class="save-hint">
        format {saveReport.format}, packets {saveReport.packets_written.toLocaleString()}
      </p>
    {/if}
  </section>
</div>

<style>
  .shell {
    max-width: 1600px;
    margin: 0 auto;
    display: grid;
    gap: 14px;
  }

  .topbar {
    display: flex;
    justify-content: space-between;
    align-items: flex-end;
    gap: 20px;
    background: linear-gradient(130deg, rgba(255, 255, 255, 0.93), rgba(230, 241, 255, 0.88));
    border: 1px solid var(--line);
    border-radius: 18px;
    padding: 22px;
    backdrop-filter: blur(8px);
  }

  .eyebrow {
    margin: 0;
    text-transform: uppercase;
    letter-spacing: 0.14em;
    font-size: 0.73rem;
    color: var(--faint);
  }

  h1 {
    margin: 0.28rem 0;
    font-size: clamp(1.4rem, 2.2vw, 2.3rem);
    color: var(--ink);
  }

  .subline {
    margin: 0;
    color: var(--ink-soft);
  }

  .status-card {
    min-width: 200px;
    text-align: right;
    border-radius: 14px;
    padding: 14px;
    border: 1px solid rgba(26, 123, 255, 0.22);
    background: linear-gradient(145deg, rgba(255, 255, 255, 0.93), rgba(216, 232, 252, 0.9));
  }

  .status-card p {
    margin: 0;
    color: var(--faint);
    font-size: 0.78rem;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  .status-card strong {
    display: block;
    font-family: "Menlo", "Monaco", "Consolas", monospace;
    margin-top: 0.3rem;
    font-size: 1.1rem;
  }

  .controls {
    display: grid;
    grid-template-columns: 2fr 1.5fr 130px 150px 190px;
    gap: 10px;
    align-items: end;
  }

  .live-capture {
    border: 1px solid var(--line);
    border-radius: 14px;
    background: rgba(255, 255, 255, 0.9);
    padding: 12px;
    display: grid;
    grid-template-columns: 1.6fr repeat(4, minmax(0, 170px));
    gap: 10px;
    align-items: end;
  }

  .live-runtime {
    grid-column: 1 / -1;
    border: 1px solid #dce6f5;
    border-radius: 10px;
    background: #f8fbff;
    padding: 8px 10px;
    display: grid;
    gap: 5px;
  }

  .live-runtime p {
    margin: 0;
    color: var(--ink-soft);
    font-size: 0.8rem;
  }

  .live-follow {
    display: flex;
    gap: 10px;
    align-items: center;
    justify-content: center;
    border: 1px solid var(--line);
    border-radius: 12px;
    padding: 10px 12px;
    background: #f8fbff;
    text-transform: none;
    letter-spacing: normal;
    font-size: 0.8rem;
    color: var(--ink-soft);
  }

  .live-follow input {
    width: auto;
    margin: 0;
    transform: scale(1.05);
  }

  label {
    display: grid;
    gap: 6px;
    font-size: 0.78rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--faint);
  }

  input,
  select {
    width: 100%;
    border: 1px solid var(--line);
    border-radius: 12px;
    padding: 11px 12px;
    background: rgba(255, 255, 255, 0.86);
    color: var(--ink);
    transition: border-color 140ms ease;
  }

  input:focus,
  select:focus {
    outline: none;
    border-color: var(--accent);
  }

  .primary {
    border: 0;
    border-radius: 12px;
    padding: 12px 16px;
    background: linear-gradient(130deg, #1a7bff, #2a91ff);
    color: #fff;
    font-weight: 700;
    letter-spacing: 0.02em;
    cursor: pointer;
  }

  .primary:disabled {
    opacity: 0.65;
    cursor: not-allowed;
  }

  .secondary,
  .accent,
  .tertiary {
    border: 0;
    border-radius: 12px;
    padding: 12px 16px;
    color: #fff;
    font-weight: 700;
    letter-spacing: 0.02em;
    cursor: pointer;
  }

  .secondary {
    background: linear-gradient(130deg, #225fa8, #2d79cc);
  }

  .accent {
    background: linear-gradient(130deg, #e76827, #ff8b3f);
  }

  .tertiary {
    justify-self: start;
    padding: 8px 12px;
    font-size: 0.78rem;
    background: linear-gradient(130deg, #0f8a64, #2aa577);
  }

  .secondary:disabled,
  .accent:disabled,
  .tertiary:disabled {
    opacity: 0.65;
    cursor: not-allowed;
  }

  .samples {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
  }

  .samples span,
  .recent span {
    color: var(--faint);
    font-size: 0.83rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }

  .samples button,
  .recent button {
    border: 1px solid var(--line);
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.9);
    color: var(--ink-soft);
    padding: 6px 12px;
    cursor: pointer;
  }

  .recent {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
  }

  .error-banner {
    border: 1px solid rgba(154, 61, 25, 0.25);
    color: var(--warn);
    border-radius: 12px;
    padding: 10px 12px;
    background: rgba(255, 236, 227, 0.88);
  }

  .stats {
    display: grid;
    grid-template-columns: 1.5fr repeat(3, minmax(130px, 1fr));
    gap: 10px;
  }

  .capture-meta,
  .stats article {
    background: rgba(255, 255, 255, 0.9);
    border: 1px solid var(--line);
    border-radius: 14px;
    padding: 12px 14px;
  }

  .capture-meta p {
    margin: 0;
    font-size: 0.84rem;
    color: var(--ink-soft);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .capture-meta span {
    display: inline-block;
    margin-top: 6px;
    font-weight: 700;
    font-size: 0.77rem;
    letter-spacing: 0.07em;
    color: var(--accent);
  }

  .stats article p {
    margin: 0;
    color: var(--faint);
    font-size: 0.8rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .stats article strong {
    display: block;
    margin-top: 5px;
    font-size: 1.2rem;
  }

  .workspace {
    display: grid;
    grid-template-columns: 1.25fr 1fr;
    gap: 12px;
    min-height: 560px;
  }

  .analysis-controls {
    display: grid;
    grid-template-columns: 1.4fr 1.4fr 200px;
    gap: 10px;
    align-items: end;
  }

  .conversations-grid {
    display: grid;
    grid-template-columns: 1fr;
    gap: 12px;
  }

  .analysis-export {
    border: 1px solid var(--line);
    border-radius: 14px;
    background: rgba(255, 255, 255, 0.9);
    padding: 12px;
    display: grid;
    grid-template-columns: 220px repeat(3, minmax(0, 1fr));
    gap: 10px;
    align-items: end;
  }

  .analysis-export p {
    margin: 0;
    grid-column: 1 / -1;
  }

  .analysis-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 12px;
  }

  .analysis-panel {
    min-height: 360px;
  }

  .analysis-detail {
    margin-top: 10px;
    border: 1px solid var(--line);
    border-radius: 12px;
    background: #f8fbff;
    padding: 10px;
    display: grid;
    gap: 8px;
  }

  .analysis-metrics {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 6px;
  }

  .analysis-metrics article {
    border: 1px solid #dce6f5;
    border-radius: 9px;
    background: #fff;
    padding: 7px;
  }

  .analysis-metrics article p {
    margin: 0;
    color: var(--faint);
    font-size: 0.68rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .analysis-metrics article strong {
    display: block;
    margin-top: 4px;
    font-size: 0.95rem;
  }

  .detail-line {
    margin: 0;
    color: var(--ink-soft);
    font-size: 0.82rem;
  }

  .detail-columns {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 8px;
  }

  .detail-columns article,
  .analysis-detail article {
    border: 1px solid #dce6f5;
    border-radius: 9px;
    background: #fff;
    padding: 8px;
  }

  .detail-columns article p,
  .analysis-detail article p {
    margin: 0;
    color: var(--ink-soft);
    font-size: 0.74rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .compact-list {
    margin: 6px 0 0;
    padding-left: 16px;
    display: grid;
    gap: 4px;
    color: var(--ink);
    font-size: 0.78rem;
  }

  .save-panel {
    border: 1px solid var(--line);
    border-radius: 14px;
    background: rgba(255, 255, 255, 0.9);
    padding: 12px;
    display: grid;
    grid-template-columns: 1fr 170px 230px;
    gap: 10px;
    align-items: end;
  }

  .save-ok,
  .save-hint {
    margin: 0;
    grid-column: 1 / -1;
    font-size: 0.84rem;
  }

  .save-ok {
    color: var(--ok);
    font-weight: 700;
  }

  .save-hint {
    color: var(--faint);
  }

  .panel {
    background: rgba(255, 255, 255, 0.92);
    border: 1px solid var(--line);
    border-radius: 18px;
    padding: 14px;
    display: grid;
    grid-template-rows: auto 1fr;
    min-height: 420px;
  }

  .panel-header {
    display: flex;
    justify-content: space-between;
    gap: 10px;
    align-items: baseline;
    margin-bottom: 10px;
  }

  h2 {
    margin: 0;
    font-size: 1rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .panel-header p {
    margin: 0;
    color: var(--faint);
    font-size: 0.82rem;
  }

  .table-wrap {
    overflow: auto;
    border: 1px solid var(--line);
    border-radius: 12px;
  }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.82rem;
  }

  thead th {
    position: sticky;
    top: 0;
    z-index: 1;
    text-align: left;
    background: linear-gradient(180deg, #f2f7ff, #ecf4ff);
    color: var(--ink-soft);
    letter-spacing: 0.04em;
    text-transform: uppercase;
    font-size: 0.72rem;
  }

  th,
  td {
    border-bottom: 1px solid #e3ebf7;
    padding: 8px;
    white-space: nowrap;
    max-width: 220px;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  tr {
    cursor: pointer;
    transition: background-color 110ms ease;
  }

  tr:hover {
    background: rgba(26, 123, 255, 0.06);
  }

  tr.selected {
    background: rgba(26, 123, 255, 0.14);
  }

  .mono {
    font-family: "Menlo", "Monaco", "Consolas", monospace;
  }

  .details {
    display: grid;
    grid-template-rows: auto auto auto auto 1fr;
    gap: 10px;
  }

  .packet-summary {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 8px;
  }

  .packet-summary div {
    border: 1px solid var(--line);
    border-radius: 12px;
    padding: 8px 10px;
    background: var(--surface-0);
  }

  .packet-summary p {
    margin: 0;
    color: var(--faint);
    font-size: 0.72rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .packet-summary strong {
    display: block;
    margin-top: 4px;
  }

  .layer-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 8px;
  }

  .layer-grid article {
    border: 1px solid var(--line);
    border-radius: 10px;
    background: #f8fbff;
    padding: 8px;
    min-height: 82px;
  }

  .layer-grid p,
  h3 {
    margin: 0;
    color: var(--ink-soft);
    font-size: 0.74rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .layer-grid pre {
    margin: 6px 0 0;
    font-size: 0.72rem;
    white-space: pre-wrap;
    word-break: break-word;
    color: var(--ink);
  }

  .field-pane,
  .hex-pane {
    border: 1px solid var(--line);
    border-radius: 12px;
    padding: 10px;
    background: #f8fbff;
  }

  .field-list {
    margin-top: 8px;
    max-height: 180px;
    overflow: auto;
    display: grid;
    gap: 5px;
  }

  .field-list button {
    display: grid;
    justify-items: start;
    gap: 2px;
    border: 1px solid #d6e1f0;
    border-radius: 8px;
    background: #fff;
    padding: 7px 10px;
    cursor: pointer;
    text-align: left;
  }

  .field-list button.active {
    border-color: #2b7be6;
    background: #e8f2ff;
  }

  .field-list span {
    color: var(--ink-soft);
    font-size: 0.76rem;
    font-family: "Menlo", "Monaco", "Consolas", monospace;
  }

  .field-list em {
    color: var(--faint);
    font-size: 0.68rem;
    font-style: normal;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .hex-grid {
    margin-top: 8px;
    max-height: 220px;
    overflow: auto;
    display: grid;
    gap: 4px;
    font-family: "Menlo", "Monaco", "Consolas", monospace;
    font-size: 0.76rem;
  }

  .hex-row {
    display: grid;
    grid-template-columns: 52px 1fr 160px;
    gap: 8px;
    align-items: center;
  }

  .offset {
    color: var(--faint);
  }

  .hex-bytes,
  .ascii-bytes {
    display: grid;
    grid-template-columns: repeat(16, minmax(0, 1fr));
    gap: 2px;
  }

  .hex-bytes span,
  .ascii-bytes span {
    text-align: center;
    padding: 2px 0;
    border-radius: 4px;
  }

  span.highlight {
    background: rgba(26, 123, 255, 0.24);
  }

  .empty {
    margin: 0;
    color: var(--faint);
    font-size: 0.85rem;
  }

  .reveal {
    animation: riseIn 320ms ease both;
  }

  .reveal-delay-1 {
    animation-delay: 40ms;
  }

  .reveal-delay-2 {
    animation-delay: 80ms;
  }

  .reveal-delay-3 {
    animation-delay: 130ms;
  }

  @keyframes riseIn {
    from {
      opacity: 0;
      transform: translateY(10px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  @media (max-width: 1200px) {
    .controls {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .live-capture {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .stats {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .workspace {
      grid-template-columns: 1fr;
    }

    .analysis-controls {
      grid-template-columns: 1fr;
    }

    .analysis-grid {
      grid-template-columns: 1fr;
    }

    .analysis-export {
      grid-template-columns: 1fr;
    }

    .analysis-metrics,
    .detail-columns {
      grid-template-columns: 1fr;
    }

    .save-panel {
      grid-template-columns: 1fr;
    }

    .details {
      grid-template-rows: auto;
    }
  }

  @media (max-width: 760px) {
    .topbar {
      flex-direction: column;
      align-items: flex-start;
    }

    .status-card {
      width: 100%;
      text-align: left;
    }

    .controls {
      grid-template-columns: 1fr;
    }

    .live-capture {
      grid-template-columns: 1fr;
    }

    .samples {
      align-items: flex-start;
      flex-direction: column;
    }

    .recent {
      align-items: flex-start;
      flex-direction: column;
    }

    .layer-grid {
      grid-template-columns: 1fr;
    }

    .packet-summary {
      grid-template-columns: 1fr;
    }

    .hex-row {
      grid-template-columns: 52px 1fr;
    }

    .ascii-bytes {
      display: none;
    }
  }
</style>
