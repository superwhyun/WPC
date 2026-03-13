pub const INDEX_HTML: &str = r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>WinParentalControl</title>
  <style>
    @import url('https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&family=JetBrains+Mono:wght@500;700&display=swap');
    
    :root {
      --bg-deep: #05080f;
      --bg-panel: #0a0f1c;
      --bg-card: #0f1623;
      --bg-input: #111827;
      --bg-hover: #1a2332;
      
      --border-subtle: rgba(100, 116, 139, 0.15);
      --border-default: rgba(100, 116, 139, 0.25);
      --border-accent: rgba(56, 189, 248, 0.4);
      
      --text-primary: #f1f5f9;
      --text-secondary: #94a3b8;
      --text-muted: #64748b;
      
      --accent-cyan: #38bdf8;
      --accent-cyan-glow: rgba(56, 189, 248, 0.3);
      --accent-blue: #60a5fa;
      --accent-steel: #475569;
      
      --danger: #ef4444;
      --danger-bg: rgba(239, 68, 68, 0.1);
      --ok: #22d3ee;
      --ok-bg: rgba(34, 211, 238, 0.1);
      --warning: #f59e0b;
      
      --shadow-sm: 0 2px 8px rgba(0, 0, 0, 0.4);
      --shadow-md: 0 8px 32px rgba(0, 0, 0, 0.5);
      --shadow-glow: 0 0 40px rgba(56, 189, 248, 0.08);
      
      --radius-sm: 8px;
      --radius-md: 12px;
      --radius-lg: 16px;
    }
    
    * { box-sizing: border-box; }
    
    body {
      margin: 0;
      font-family: 'Inter', -apple-system, BlinkMacSystemFont, sans-serif;
      color: var(--text-primary);
      background: var(--bg-deep);
      min-height: 100vh;
      display: flex;
      align-items: center;
      justify-content: center;
      padding: 24px;
    }
    
    body::before {
      content: '';
      position: fixed;
      top: -50%; left: -50%;
      width: 200%; height: 200%;
      background: 
        radial-gradient(ellipse at 30% 20%, rgba(56, 189, 248, 0.08) 0%, transparent 50%),
        radial-gradient(ellipse at 70% 80%, rgba(96, 165, 250, 0.05) 0%, transparent 50%),
        radial-gradient(ellipse at 50% 50%, rgba(15, 23, 42, 0.8) 0%, transparent 70%);
      pointer-events: none; z-index: -1;
    }
    
    .container {
      width: 100%; max-width: 900px;
      display: flex; flex-direction: column; gap: 24px;
    }
    
    .header {
      display: flex; align-items: flex-start; justify-content: space-between; gap: 24px;
    }
    
    .header-actions {
      display: flex; align-items: center; gap: 12px;
    }
    
    .btn-icon {
      width: 40px; height: 40px;
      background: var(--bg-card);
      border: 1px solid var(--border-default);
      border-radius: var(--radius-md);
      display: grid; place-items: center;
      font-size: 20px; cursor: pointer;
      transition: all 0.15s ease;
      color: var(--text-secondary);
    }
    .btn-icon:hover {
      background: var(--bg-hover);
      border-color: var(--border-accent);
      color: var(--accent-cyan);
    }
    
    .brand { display: flex; align-items: center; gap: 16px; }
    .brand-icon {
      width: 48px; height: 48px;
      background: linear-gradient(145deg, var(--accent-cyan), var(--accent-blue));
      border-radius: var(--radius-md);
      display: grid; place-items: center;
      font-size: 24px; box-shadow: 0 4px 20px var(--accent-cyan-glow);
    }
    .brand-text h1 {
      margin: 0; font-size: 24px; font-weight: 700; letter-spacing: -0.02em;
      background: linear-gradient(180deg, var(--text-primary), var(--text-secondary));
      -webkit-background-clip: text; -webkit-text-fill-color: transparent; background-clip: text;
    }
    .brand-text p { margin: 4px 0 0; font-size: 13px; color: var(--text-muted); }
    
    /* Countdown Display */
    .countdown-display {
      font-family: 'JetBrains Mono', monospace;
      font-size: 42px;
      font-weight: 700;
      color: var(--accent-cyan);
      text-shadow: 0 0 20px var(--accent-cyan-glow);
      letter-spacing: -0.02em;
      text-align: center;
      padding: 16px 24px;
      background: var(--bg-card);
      border: 1px solid var(--border-subtle);
      border-radius: var(--radius-md);
      min-width: 200px;
    }
    .countdown-display.locked {
      color: var(--danger);
      text-shadow: 0 0 20px rgba(239, 68, 68, 0.3);
    }
    
    .status-grid {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
      gap: 12px;
    }
    .status-card {
      background: var(--bg-card);
      border: 1px solid var(--border-subtle);
      border-radius: var(--radius-md);
      padding: 16px;
      display: flex; flex-direction: column; gap: 8px;
      transition: all 0.2s ease;
    }
    .status-card:hover { border-color: var(--border-default); background: var(--bg-hover); }
    .status-label { font-size: 11px; font-weight: 600; text-transform: uppercase; letter-spacing: 0.08em; color: var(--text-muted); }
    .status-value { font-size: 18px; font-weight: 600; color: var(--text-primary); display: flex; align-items: center; gap: 8px; }
    .status-value.locked { color: var(--danger); }
    .status-value.unlocked { color: var(--ok); }
    
    .indicator { width: 8px; height: 8px; border-radius: 50%; background: var(--text-muted); }
    .indicator.online { background: var(--ok); box-shadow: 0 0 8px var(--ok); }
    .indicator.offline { background: var(--danger); }
    .indicator.warning { background: var(--warning); }
    
    .panel {
      background: var(--bg-panel);
      border: 1px solid var(--border-default);
      border-radius: var(--radius-lg);
      box-shadow: var(--shadow-md), var(--shadow-glow);
      overflow: hidden;
    }
    .panel-header {
      background: linear-gradient(180deg, rgba(255,255,255,0.03), transparent);
      border-bottom: 1px solid var(--border-subtle);
      padding: 20px 24px;
      display: flex; align-items: center; gap: 12px;
    }
    .panel-header h2 { margin: 0; font-size: 14px; font-weight: 600; text-transform: uppercase; letter-spacing: 0.06em; color: var(--text-secondary); }
    .panel-header .icon { font-size: 16px; opacity: 0.7; }
    .panel-body { padding: 20px 24px; display: flex; flex-direction: column; gap: 16px; }
    
    .section { display: flex; flex-direction: column; gap: 12px; }
    .section-title {
      font-size: 12px; font-weight: 600; text-transform: uppercase; letter-spacing: 0.05em;
      color: var(--text-muted); display: flex; align-items: center; gap: 8px;
    }
    .section-title::before { content: ''; width: 3px; height: 14px; background: var(--accent-cyan); border-radius: 2px; }
    
    /* Single row form layout */
    .timed-access-row {
      display: flex;
      flex-wrap: wrap;
      gap: 12px;
      align-items: flex-end;
    }
    .change-pin-form {
      display: flex;
      flex-direction: column;
      gap: 12px;
      max-width: 400px;
    }
    .form-group {
      display: flex; flex-direction: column; gap: 6px;
    }
    .form-group.pin { flex: 1; min-width: 120px; }
    .form-group.duration { flex: 0 0 100px; }
    .form-group.timeout { flex: 0 0 140px; }
    
    .form-label { font-size: 12px; font-weight: 500; color: var(--text-secondary); }
    
    input, select {
      font: inherit; font-size: 14px; padding: 10px 12px;
      background: var(--bg-input); border: 1px solid var(--border-default);
      border-radius: var(--radius-sm); color: var(--text-primary);
      transition: all 0.15s ease; width: 100%;
    }
    input:hover, select:hover { border-color: var(--border-accent); }
    input:focus, select:focus { outline: none; border-color: var(--accent-cyan); box-shadow: 0 0 0 3px var(--accent-cyan-glow); }
    input::placeholder { color: var(--text-muted); }
    
    .btn {
      font: inherit; font-size: 14px; font-weight: 600;
      padding: 10px 18px; border: none; border-radius: var(--radius-sm);
      cursor: pointer; transition: all 0.15s ease;
      display: inline-flex; align-items: center; justify-content: center; gap: 8px;
      white-space: nowrap;
    }
    .btn-primary {
      background: linear-gradient(145deg, var(--accent-cyan), var(--accent-blue));
      color: var(--bg-deep);
    }
    .btn-primary:hover { transform: translateY(-1px); box-shadow: 0 4px 20px var(--accent-cyan-glow); }
    .btn-secondary {
      background: var(--bg-hover); color: var(--text-primary); border: 1px solid var(--border-default);
    }
    .btn-secondary:hover { background: var(--bg-card); border-color: var(--border-accent); }
    .btn-danger { background: var(--danger-bg); color: var(--danger); border: 1px solid var(--danger); }
    .btn-danger:hover { background: var(--danger); color: white; box-shadow: 0 4px 20px rgba(239, 68, 68, 0.3); }
    
    .actions-grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 12px; }
    .action-card {
      background: var(--bg-card); border: 1px solid var(--border-subtle);
      border-radius: var(--radius-md); padding: 16px; text-align: center;
      cursor: pointer; transition: all 0.2s ease;
    }
    .action-card:hover { border-color: var(--border-default); background: var(--bg-hover); transform: translateY(-2px); }
    .action-card.danger:hover { border-color: var(--danger); background: var(--danger-bg); }
    .action-icon { font-size: 24px; margin-bottom: 8px; opacity: 0.8; }
    .action-label { font-size: 13px; font-weight: 500; color: var(--text-secondary); }
    
    .message {
      padding: 12px 16px; border-radius: var(--radius-sm);
      font-size: 13px; font-weight: 500; min-height: 44px;
      display: flex; align-items: center; gap: 10px;
    }
    .message.ok { background: var(--ok-bg); color: var(--ok); border: 1px solid rgba(34, 211, 238, 0.2); }
    .message.error { background: var(--danger-bg); color: var(--danger); border: 1px solid rgba(239, 68, 68, 0.2); }
    .message:empty { display: none; }
    
    .snapshot-frame {
      background: var(--bg-card); border: 1px solid var(--border-subtle);
      border-radius: var(--radius-md); padding: 4px;
      min-height: 240px; display: flex; flex-direction: column;
    }
    .snapshot-empty {
      flex: 1; display: grid; place-items: center;
      border-radius: var(--radius-sm); border: 2px dashed var(--border-default);
      color: var(--text-muted); font-size: 14px; text-align: center; padding: 40px;
      background: var(--bg-input);
    }
    .snapshot-empty .icon { font-size: 48px; opacity: 0.3; margin-bottom: 16px; }
    .snapshot-img { width: 100%; max-height: 480px; object-fit: contain; border-radius: var(--radius-sm); background: var(--bg-deep); }
    
    .divider { height: 1px; background: linear-gradient(90deg, transparent, var(--border-default), transparent); margin: 4px 0; }
    .hidden { display: none !important; }
    
    /* Modal */
    .modal-overlay {
      position: fixed; top: 0; left: 0; right: 0; bottom: 0;
      background: rgba(5, 8, 15, 0.8);
      backdrop-filter: blur(4px);
      display: grid; place-items: center;
      z-index: 1000;
      opacity: 0; visibility: hidden;
      transition: all 0.2s ease;
    }
    .modal-overlay.active {
      opacity: 1; visibility: visible;
    }
    .modal {
      background: var(--bg-panel);
      border: 1px solid var(--border-default);
      border-radius: var(--radius-lg);
      box-shadow: var(--shadow-md), var(--shadow-glow);
      width: 90%; max-width: 420px;
      transform: scale(0.95);
      transition: transform 0.2s ease;
    }
    .modal-overlay.active .modal {
      transform: scale(1);
    }
    .modal-header {
      background: linear-gradient(180deg, rgba(255,255,255,0.03), transparent);
      border-bottom: 1px solid var(--border-subtle);
      padding: 20px 24px;
      display: flex; align-items: center; justify-content: space-between; gap: 12px;
    }
    .modal-header h2 { 
      margin: 0; font-size: 16px; font-weight: 600; 
      display: flex; align-items: center; gap: 10px;
    }
    .modal-header .icon { font-size: 20px; }
    .modal-close {
      background: none; border: none;
      font-size: 24px; color: var(--text-muted);
      cursor: pointer; padding: 0; width: 32px; height: 32px;
      display: grid; place-items: center;
      border-radius: var(--radius-sm);
      transition: all 0.15s ease;
    }
    .modal-close:hover {
      background: var(--bg-hover);
      color: var(--text-primary);
    }
    .modal-body { padding: 24px; display: flex; flex-direction: column; gap: 16px; }
    
    @media (max-width: 640px) {
      .timed-access-row { flex-direction: column; align-items: stretch; }
      .form-group.pin, .form-group.duration, .form-group.timeout { flex: 1; }
      .actions-grid { grid-template-columns: 1fr; }
      .status-grid { grid-template-columns: repeat(2, 1fr); }
      .countdown-display { font-size: 32px; }
    }
  </style>
</head>
<body>
  <div class="container">
    <header class="header">
      <div class="brand">
        <div class="brand-icon">🛡️</div>
        <div class="brand-text">
          <h1>WinParentalControl</h1>
          <p>Secure session management & enforcement</p>
        </div>
      </div>
      <div class="header-actions">
        <div class="countdown-display" id="countdown">00:00:00</div>
        <button type="button" class="btn-icon" id="settings-btn" title="Security Settings">⚙️</button>
      </div>
    </header>
    
    <div class="status-grid">
      <div class="status-card">
        <div class="status-label">Current Mode</div>
        <div class="status-value" id="mode">
          <span class="indicator" id="mode-indicator"></span>
          <span id="mode-text">-</span>
        </div>
      </div>
      <div class="status-card">
        <div class="status-label">Agent Status</div>
        <div class="status-value">
          <span class="indicator" id="agent-indicator"></span>
          <span id="agent">-</span>
        </div>
      </div>
      <div class="status-card">
        <div class="status-label">User Session</div>
        <div class="status-value">
          <span class="indicator" id="session-indicator"></span>
          <span id="session">-</span>
        </div>
      </div>
      <div class="status-card">
        <div class="status-label">Timeout Action</div>
        <div class="status-value" id="expiry-action-status">-</div>
      </div>
    </div>
    
    <div class="panel">
      <div class="panel-header">
        <span class="icon">🔐</span>
        <h2>Session Control</h2>
      </div>
      
      <div class="panel-body">
        <div class="section">
          <div class="section-title">Timed Access</div>
          <div class="timed-access-row">
            <div class="form-group pin">
              <label class="form-label">Parent PIN</label>
              <input type="password" id="pin" placeholder="Enter PIN" />
            </div>
            <div class="form-group duration">
              <label class="form-label">Duration (min)</label>
              <input type="number" id="duration" min="-480" max="480" value="30" />
            </div>
            <div class="form-group timeout">
              <label class="form-label">On Timeout</label>
              <select id="expiry-action">
                <option value="app_lock">App Lock</option>
                <option value="windows_lock">Windows Lock</option>
                <option value="shutdown">Windows Shutdown</option>
              </select>
            </div>
            <button type="button" class="btn btn-primary" id="unlock">Unlock</button>
            <button type="button" class="btn btn-secondary hidden" id="extend">Extend</button>
          </div>
        </div>
        
        <div class="divider"></div>
        
        <div class="section">
          <div class="section-title">Immediate Enforcement</div>
          <div class="actions-grid">
            <div class="action-card danger" id="lock-card">
              <div class="action-icon">🔒</div>
              <div class="action-label">Lock App Now</div>
            </div>
            <div class="action-card" id="windows-lock-card">
              <div class="action-icon">🖥️</div>
              <div class="action-label">Lock Windows</div>
            </div>
            <div class="action-card danger" id="shutdown-card">
              <div class="action-icon">⚡</div>
              <div class="action-label">Shutdown PC</div>
            </div>
          </div>
        </div>
        
        <div class="message" id="message"></div>
      </div>
    </div>

    <div class="modal-overlay" id="settings-modal">
      <div class="modal">
        <div class="modal-header">
          <h2><span class="icon">🔑</span> Security Settings</h2>
          <button type="button" class="modal-close" id="modal-close">&times;</button>
        </div>
        <div class="modal-body">
          <div class="section">
            <div class="section-title">Change PIN</div>
            <div class="change-pin-form">
              <div class="form-group">
                <label class="form-label">Current PIN</label>
                <input type="password" id="current-pin" placeholder="Enter current PIN" />
              </div>
              <div class="form-group">
                <label class="form-label">New PIN</label>
                <input type="password" id="new-pin" placeholder="Enter new PIN" />
              </div>
              <div class="form-group">
                <label class="form-label">Confirm New PIN</label>
                <input type="password" id="confirm-new-pin" placeholder="Re-enter new PIN" />
              </div>
              <button type="button" class="btn btn-primary" id="change-pin-btn">Change PIN</button>
            </div>
          </div>
          <div class="message" id="pin-message"></div>
        </div>
      </div>
    </div>

    <div class="panel">
      <div class="panel-header">
        <span class="icon">📷</span>
        <h2>Live Snapshot</h2>
        <button type="button" class="btn btn-secondary" id="snapshot-button" style="margin-left: auto; padding: 8px 16px; font-size: 13px;">Capture</button>
      </div>
      <div class="panel-body">
        <div class="snapshot-frame">
          <div class="snapshot-empty" id="snapshot-empty">
            <div>
              <div class="icon">📷</div>
              <div>No snapshot captured yet</div>
              <div style="font-size: 12px; margin-top: 8px; opacity: 0.7;">Click Capture to view current session</div>
            </div>
          </div>
          <img id="snapshot-image" class="snapshot-img hidden" alt="Session snapshot" />
        </div>
        <div id="snapshot-meta" style="font-size: 12px; color: var(--text-muted); text-align: center; margin-top: 8px;"></div>
      </div>
    </div>
  </div>
  
  <script>
    let token = null;
    let currentModeState = { mode: "locked" };
    let snapshotUrl = null;
    let countdownInterval = null;
    let serverExpiresAt = null;
    
    function expiryActionLabel(action) {
      switch (action) {
        case "app_lock": case "agent_lock": return "App Lock";
        case "windows_lock": return "Windows Lock";
        case "shutdown": return "Windows Shutdown";
        default: return "App Lock";
      }
    }
    
    function formatCountdown(seconds) {
      if (seconds <= 0) return "00:00:00";
      const hrs = Math.floor(seconds / 3600);
      const mins = Math.floor((seconds % 3600) / 60);
      const secs = seconds % 60;
      const mm = mins.toString().padStart(2,'0');
      const ss = secs.toString().padStart(2,'0');
      if (hrs > 0) {
        const hh = hrs.toString().padStart(2,'0');
        return `${hh}:${mm}:${ss}`;
      }
      return `${mm}:${ss}`;
    }
    
    function updateCountdownDisplay() {
      const el = document.getElementById("countdown");
      if (!serverExpiresAt || currentModeState.mode === "locked") {
        el.textContent = "00:00:00";
        el.classList.add("locked");
        return;
      }
      const now = Date.now();
      const remaining = Math.max(0, Math.floor((serverExpiresAt - now) / 1000));
      el.textContent = formatCountdown(remaining);
      el.classList.remove("locked");
      if (remaining <= 0) {
        el.classList.add("locked");
        el.textContent = "00:00:00";
      }
    }
    
    function startCountdown(expiresAtUtc) {
      serverExpiresAt = new Date(expiresAtUtc).getTime();
      updateCountdownDisplay();
      if (countdownInterval) clearInterval(countdownInterval);
      countdownInterval = setInterval(updateCountdownDisplay, 1000);
    }
    
    function stopCountdown() {
      if (countdownInterval) {
        clearInterval(countdownInterval);
        countdownInterval = null;
      }
      serverExpiresAt = null;
      updateCountdownDisplay();
    }
    
    async function request(path, options = {}) {
      const headers = Object.assign({"Content-Type": "application/json"}, options.headers || {});
      if (token) headers["Authorization"] = `Bearer ${token}`;
      const response = await fetch(path, {...options, headers});
      const body = await response.json().catch(() => ({}));
      if (!response.ok) throw new Error(body.error || `HTTP ${response.status}`);
      return body;
    }
    
    async function refresh() {
      try {
        const status = await request("/api/device/status", { method: "GET", headers: {} });
        currentModeState.mode = status.mode;
        
        const modeText = document.getElementById("mode-text");
        const modeIndicator = document.getElementById("mode-indicator");
        modeText.textContent = status.mode;
        document.getElementById("mode").className = `status-value ${status.mode}`;
        modeIndicator.className = `indicator ${status.mode === "unlocked" ? "online" : "offline"}`;
        
        const agentIndicator = document.getElementById("agent-indicator");
        agentIndicator.className = `indicator ${status.agentHealthy ? "online" : "warning"}`;
        document.getElementById("agent").textContent = status.agentHealthy ? "Healthy" : "Stale";
        
        const sessionIndicator = document.getElementById("session-indicator");
        sessionIndicator.className = `indicator ${status.protectedUserLoggedIn ? "online" : "offline"}`;
        document.getElementById("session").textContent = status.protectedUserLoggedIn ? "Online" : "Offline";
        
        document.getElementById("expiry-action-status").textContent = status.unlockExpiryAction
          ? expiryActionLabel(status.unlockExpiryAction)
          : (status.mode === "locked" ? "Locked" : "App Lock");
        
        if (status.unlockExpiryAction) {
          document.getElementById("expiry-action").value = status.unlockExpiryAction;
        }
        
        // Manage countdown
        if (status.mode === "unlocked" && status.unlockExpiresAtUtc) {
          startCountdown(status.unlockExpiresAtUtc);
        } else {
          stopCountdown();
        }
        
        syncActions(status);
      } catch (error) {
        setMessage(error.message, true);
      }
    }
    
    function syncActions(status) {
      const unlockBtn = document.getElementById("unlock");
      const extendBtn = document.getElementById("extend");
      if (status.mode === "unlocked") {
        unlockBtn.classList.add("hidden");
        extendBtn.classList.remove("hidden");
      } else {
        unlockBtn.classList.remove("hidden");
        extendBtn.classList.add("hidden");
      }
    }
    
    function setMessage(message, isError = false) {
      const node = document.getElementById("message");
      node.className = `message ${isError ? "error" : "ok"}`;
      node.textContent = message;
    }
    
    function setSnapshotMeta(message) {
      document.getElementById("snapshot-meta").textContent = message;
    }
    
    async function auth(pin) {
      const body = await request("/api/auth/pin", { method: "POST", body: JSON.stringify({ pin }) });
      token = body.token;
    }
    
    function readDurationMinutes() {
      const raw = document.getElementById("duration").value.trim();
      const parsed = Number.parseInt(raw || "30", 10);
      if (!Number.isFinite(parsed) || parsed < -480 || parsed > 480) {
        throw new Error("Duration must be between -480 and 480 minutes.");
      }
      return parsed;
    }
    
    function readExpiryAction() {
      return document.getElementById("expiry-action").value;
    }
    
    async function action(path) {
      const isImmediateAppLock = path === "/api/device/lock";
      const options = { method: "POST" };
      if (!isImmediateAppLock) {
        options.body = JSON.stringify({ durationMinutes: readDurationMinutes(), expiryAction: readExpiryAction() });
      }
      const result = await request(path, options);
      if (isImmediateAppLock) setMessage("✓ App lock applied successfully");
      else setMessage(`✓ Mode: ${result.status.mode} · Timeout: ${expiryActionLabel(result.status.unlockExpiryAction || "app_lock")}`);
      return result;
    }
    
    async function ensureAuth() {
      if (!token) await auth(document.getElementById("pin").value);
    }
    
    async function captureSnapshot() {
      await ensureAuth();
      const response = await fetch("/api/device/snapshot", { method: "GET", headers: { Authorization: `Bearer ${token}` } });
      if (!response.ok) {
        let message = `HTTP ${response.status}`;
        try { const body = await response.json(); message = body.error || message; } catch {}
        throw new Error(message);
      }
      const blob = await response.blob();
      if (snapshotUrl) URL.revokeObjectURL(snapshotUrl);
      snapshotUrl = URL.createObjectURL(blob);
      document.getElementById("snapshot-image").src = snapshotUrl;
      document.getElementById("snapshot-image").classList.remove("hidden");
      document.getElementById("snapshot-empty").classList.add("hidden");
      setSnapshotMeta(`Captured at ${new Date().toLocaleTimeString()}`);
    }
    
    document.getElementById("unlock").addEventListener("click", async () => {
      if (currentModeState.mode === "unlocked") {
        setMessage("Already unlocked."); return;
      }
      try {
        await auth(document.getElementById("pin").value);
        await action("/api/device/unlock");
        await refresh();
      } catch (error) { setMessage(error.message, true); }
    });
    
    document.getElementById("extend").addEventListener("click", async () => {
      try {
        await ensureAuth();
        await action("/api/device/extend");
        await refresh();
      } catch (error) { setMessage(error.message, true); }
    });
    
    document.getElementById("expiry-action").addEventListener("change", async () => {
      if (currentModeState.mode !== "unlocked") return;
      try {
        await ensureAuth();
        const expiryAction = readExpiryAction();
        await request("/api/device/expiry-action", { method: "POST", body: JSON.stringify({ expiryAction }) });
        setMessage(`✓ Timeout action updated to ${expiryAction.replace('_', ' ')}`);
        await refresh();
      } catch (error) { setMessage(error.message, true); }
    });
    
    document.getElementById("lock-card").addEventListener("click", async () => {
      try {
        await ensureAuth();
        await request("/api/device/unlock", { method: "POST", body: JSON.stringify({ durationMinutes: 0, expiryAction: "app_lock" }) });
        setMessage("✓ App lock applied successfully");
        await refresh();
      } catch (error) { setMessage(error.message, true); }
    });
    
    document.getElementById("windows-lock-card").addEventListener("click", async () => {
      try {
        await ensureAuth();
        await request("/api/device/unlock", { method: "POST", body: JSON.stringify({ durationMinutes: 0, expiryAction: "windows_lock" }) });
        setMessage("✓ Windows lock applied successfully");
        await refresh();
      } catch (error) { setMessage(error.message, true); }
    });
    
    document.getElementById("shutdown-card").addEventListener("click", async () => {
      try {
        await ensureAuth();
        await request("/api/device/unlock", { method: "POST", body: JSON.stringify({ durationMinutes: 0, expiryAction: "shutdown" }) });
        setMessage("✓ Windows shutdown requested");
        await refresh();
      } catch (error) { setMessage(error.message, true); }
    });

    const settingsModal = document.getElementById("settings-modal");
    
    function openModal() {
      settingsModal.classList.add("active");
      document.getElementById("pin-message").textContent = "";
      document.getElementById("pin-message").className = "message";
    }
    
    function closeModal() {
      settingsModal.classList.remove("active");
      document.getElementById("current-pin").value = "";
      document.getElementById("new-pin").value = "";
      document.getElementById("confirm-new-pin").value = "";
      document.getElementById("pin-message").textContent = "";
      document.getElementById("pin-message").className = "message";
    }
    
    document.getElementById("settings-btn").addEventListener("click", openModal);
    document.getElementById("modal-close").addEventListener("click", closeModal);
    
    settingsModal.addEventListener("click", (e) => {
      if (e.target === settingsModal) closeModal();
    });
    
    document.addEventListener("keydown", (e) => {
      if (e.key === "Escape" && settingsModal.classList.contains("active")) {
        closeModal();
      }
    });

    document.getElementById("change-pin-btn").addEventListener("click", async () => {
      const currentPin = document.getElementById("current-pin").value;
      const newPin = document.getElementById("new-pin").value;
      const confirmNewPin = document.getElementById("confirm-new-pin").value;
      const pinMessage = document.getElementById("pin-message");

      pinMessage.textContent = "";
      pinMessage.className = "message";

      if (!currentPin || !newPin || !confirmNewPin) {
        pinMessage.textContent = "⚠ All fields are required";
        pinMessage.classList.add("error");
        return;
      }

      if (newPin !== confirmNewPin) {
        pinMessage.textContent = "⚠ New PINs do not match";
        pinMessage.classList.add("error");
        return;
      }

      if (newPin.length < 4) {
        pinMessage.textContent = "⚠ PIN must be at least 4 characters";
        pinMessage.classList.add("error");
        return;
      }

      try {
        await request("/api/auth/change-pin", {
          method: "POST",
          body: JSON.stringify({ currentPin, newPin })
        });
        pinMessage.textContent = "✓ PIN changed successfully";
        pinMessage.classList.add("ok");
        document.getElementById("current-pin").value = "";
        document.getElementById("new-pin").value = "";
        document.getElementById("confirm-new-pin").value = "";
        sessionStorage.removeItem("authToken");
        setTimeout(closeModal, 1500);
      } catch (error) {
        pinMessage.textContent = `✗ ${error.message}`;
        pinMessage.classList.add("error");
      }
    });

    document.getElementById("snapshot-button").addEventListener("click", async () => {
      try { await captureSnapshot(); } catch (error) { setSnapshotMeta(error.message); }
    });
    
    refresh();
    setInterval(refresh, 5000);
  </script>
</body>
</html>
"#;
