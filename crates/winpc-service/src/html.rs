pub const INDEX_HTML: &str = r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>WinParentalControl</title>
  <style>
    @import url('https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700;800&family=JetBrains+Mono:wght@400;500;700&display=swap');

    :root {
      --bg-deep: #030712;
      --bg-panel: #0a0f1c;
      --bg-card: #0f1623;
      --bg-card-elevated: #131b2e;
      --bg-input: #111827;
      --bg-hover: #1a2332;

      --border-subtle: rgba(100, 116, 139, 0.12);
      --border-default: rgba(100, 116, 139, 0.2);
      --border-accent: rgba(56, 189, 248, 0.35);

      --text-primary: #f1f5f9;
      --text-secondary: #94a3b8;
      --text-muted: #64748b;
      --text-dim: #475569;

      --accent-cyan: #38bdf8;
      --accent-cyan-glow: rgba(56, 189, 248, 0.25);
      --accent-blue: #60a5fa;
      --accent-violet: #a78bfa;
      --accent-emerald: #34d399;

      --danger: #ef4444;
      --danger-bg: rgba(239, 68, 68, 0.08);
      --danger-glow: rgba(239, 68, 68, 0.25);
      --ok: #22d3ee;
      --ok-bg: rgba(34, 211, 238, 0.08);
      --warning: #f59e0b;
      --warning-bg: rgba(245, 158, 11, 0.08);

      --shadow-sm: 0 2px 8px rgba(0, 0, 0, 0.4);
      --shadow-md: 0 8px 32px rgba(0, 0, 0, 0.5);
      --shadow-lg: 0 16px 64px rgba(0, 0, 0, 0.6);
      --shadow-glow-cyan: 0 0 60px rgba(56, 189, 248, 0.06);
      --shadow-glow-red: 0 0 60px rgba(239, 68, 68, 0.06);

      --radius-sm: 8px;
      --radius-md: 12px;
      --radius-lg: 16px;
      --radius-xl: 20px;
      --radius-full: 9999px;
    }

    * { box-sizing: border-box; margin: 0; padding: 0; }

    body {
      font-family: 'Inter', -apple-system, BlinkMacSystemFont, sans-serif;
      color: var(--text-primary);
      background: var(--bg-deep);
      min-height: 100vh;
      display: flex;
      align-items: flex-start;
      justify-content: center;
      padding: 24px;
      overflow-x: hidden;
    }

    body::before {
      content: '';
      position: fixed; inset: 0;
      background:
        radial-gradient(ellipse 80% 60% at 20% 10%, rgba(56, 189, 248, 0.06) 0%, transparent 60%),
        radial-gradient(ellipse 60% 80% at 80% 90%, rgba(96, 165, 250, 0.04) 0%, transparent 60%),
        radial-gradient(ellipse 100% 100% at 50% 50%, rgba(15, 23, 42, 0.5) 0%, transparent 70%);
      pointer-events: none; z-index: -1;
    }

    .app { width: 100%; max-width: 920px; display: flex; flex-direction: column; gap: 20px; }

    /* ── HEADER ────────────────────────────────────────── */
    .header {
      display: flex; align-items: center; justify-content: space-between; gap: 16px;
      padding: 4px 0;
    }
    .brand { display: flex; align-items: center; gap: 14px; }
    .brand-icon {
      width: 42px; height: 42px;
      background: linear-gradient(145deg, var(--accent-cyan), var(--accent-blue));
      border-radius: var(--radius-md);
      display: grid; place-items: center;
      font-size: 20px; box-shadow: 0 4px 20px var(--accent-cyan-glow);
      flex-shrink: 0;
    }
    .brand-text h1 {
      font-size: 20px; font-weight: 700; letter-spacing: -0.02em;
      background: linear-gradient(180deg, var(--text-primary) 30%, var(--text-secondary));
      -webkit-background-clip: text; -webkit-text-fill-color: transparent; background-clip: text;
    }
    .brand-text p { font-size: 12px; color: var(--text-muted); margin-top: 2px; }
    .header-actions { display: flex; align-items: center; gap: 8px; }

    .btn-icon {
      width: 36px; height: 36px;
      background: var(--bg-card);
      border: 1px solid var(--border-subtle);
      border-radius: var(--radius-sm);
      display: grid; place-items: center;
      font-size: 16px; cursor: pointer;
      transition: all 0.2s ease;
      color: var(--text-muted);
    }
    .btn-icon:hover {
      background: var(--bg-hover); border-color: var(--border-accent);
      color: var(--accent-cyan); transform: translateY(-1px);
    }

    /* ── STATUS STRIP ─────────────────────────────────── */
    .status-strip {
      display: flex; gap: 8px; flex-wrap: wrap;
    }
    .status-chip {
      display: flex; align-items: center; gap: 6px;
      padding: 6px 12px;
      background: var(--bg-card);
      border: 1px solid var(--border-subtle);
      border-radius: var(--radius-full);
      font-size: 12px; font-weight: 500;
      color: var(--text-secondary);
      transition: all 0.2s ease;
    }
    .status-chip:hover { background: var(--bg-hover); }
    .status-dot {
      width: 6px; height: 6px; border-radius: 50%;
      background: var(--text-muted); flex-shrink: 0;
    }
    .status-dot.online { background: var(--accent-emerald); box-shadow: 0 0 6px var(--accent-emerald); }
    .status-dot.offline { background: var(--danger); }
    .status-dot.warning { background: var(--warning); }

    /* ── HERO: LOCK / UNLOCK STATE ────────────────────── */
    .hero {
      position: relative;
      border-radius: var(--radius-xl);
      overflow: hidden;
      transition: all 0.4s ease;
    }
    .hero::before {
      content: ''; position: absolute; inset: 0;
      border-radius: var(--radius-xl);
      pointer-events: none; z-index: 1;
    }
    .hero.locked {
      background: var(--bg-panel);
      border: 1px solid rgba(239, 68, 68, 0.15);
      box-shadow: var(--shadow-lg), var(--shadow-glow-red);
    }
    .hero.locked::before {
      background: linear-gradient(180deg, rgba(239, 68, 68, 0.04) 0%, transparent 40%);
    }
    .hero.unlocked {
      background: var(--bg-panel);
      border: 1px solid rgba(56, 189, 248, 0.15);
      box-shadow: var(--shadow-lg), var(--shadow-glow-cyan);
    }
    .hero.unlocked::before {
      background: linear-gradient(180deg, rgba(56, 189, 248, 0.04) 0%, transparent 40%);
    }
    .hero-inner { position: relative; z-index: 2; padding: 32px; }

    /* ── LOCKED STATE ─────────────────────────────────── */
    .lock-view { display: flex; flex-direction: column; align-items: center; gap: 28px; }

    .lock-icon-wrap {
      width: 80px; height: 80px;
      border-radius: 50%;
      background: var(--danger-bg);
      border: 2px solid rgba(239, 68, 68, 0.2);
      display: grid; place-items: center;
      font-size: 36px;
      animation: pulse-red 3s ease-in-out infinite;
    }
    @keyframes pulse-red {
      0%, 100% { box-shadow: 0 0 0 0 rgba(239, 68, 68, 0.15); }
      50% { box-shadow: 0 0 0 16px rgba(239, 68, 68, 0); }
    }

    .lock-title {
      font-size: 28px; font-weight: 800; letter-spacing: -0.03em;
      color: var(--danger);
      text-transform: uppercase;
    }

    .lock-form {
      width: 100%; max-width: 360px;
      display: flex; flex-direction: column; gap: 16px;
    }

    .pin-input-wrap {
      position: relative;
    }
    .pin-input-wrap input {
      width: 100%;
      font-size: 18px; font-weight: 500;
      padding: 14px 16px 14px 44px;
      background: var(--bg-input);
      border: 1px solid var(--border-default);
      border-radius: var(--radius-md);
      color: var(--text-primary);
      text-align: center;
      letter-spacing: 0.15em;
      transition: all 0.2s ease;
      font-family: 'JetBrains Mono', monospace;
    }
    .pin-input-wrap input:focus {
      outline: none; border-color: var(--accent-cyan);
      box-shadow: 0 0 0 3px var(--accent-cyan-glow), 0 4px 20px rgba(56, 189, 248, 0.1);
    }
    .pin-input-wrap input::placeholder {
      color: var(--text-muted); letter-spacing: 0.02em; font-family: 'Inter', sans-serif; font-size: 14px;
    }
    .pin-input-icon {
      position: absolute; left: 14px; top: 50%; transform: translateY(-50%);
      font-size: 18px; opacity: 0.5; pointer-events: none;
    }

    .unlock-options {
      display: grid; grid-template-columns: 1fr 1fr; gap: 10px;
    }
    .opt-group { display: flex; flex-direction: column; gap: 5px; }
    .opt-label {
      font-size: 11px; font-weight: 600; text-transform: uppercase;
      letter-spacing: 0.06em; color: var(--text-muted); padding-left: 2px;
    }
    .opt-group input, .opt-group select {
      font: inherit; font-size: 13px; padding: 10px 12px;
      background: var(--bg-input); border: 1px solid var(--border-default);
      border-radius: var(--radius-sm); color: var(--text-primary);
      transition: all 0.15s ease; width: 100%;
    }
    .opt-group input:focus, .opt-group select:focus {
      outline: none; border-color: var(--accent-cyan);
      box-shadow: 0 0 0 3px var(--accent-cyan-glow);
    }

    .btn-unlock {
      font: inherit; font-size: 15px; font-weight: 700;
      padding: 14px 24px; border: none; border-radius: var(--radius-md);
      cursor: pointer; transition: all 0.2s ease;
      display: flex; align-items: center; justify-content: center; gap: 10px;
      background: linear-gradient(145deg, var(--accent-cyan), var(--accent-blue));
      color: var(--bg-deep);
      text-transform: uppercase; letter-spacing: 0.04em;
    }
    .btn-unlock:hover {
      transform: translateY(-2px);
      box-shadow: 0 8px 32px var(--accent-cyan-glow);
    }
    .btn-unlock:active { transform: translateY(0); }

    /* ── UNLOCKED STATE ───────────────────────────────── */
    .unlock-view { display: flex; flex-direction: column; gap: 24px; }

    .unlock-hero-row {
      display: flex; align-items: center; gap: 24px;
    }

    .countdown-block {
      flex: 1;
      display: flex; flex-direction: column; align-items: center; gap: 8px;
    }
    .countdown-label {
      font-size: 11px; font-weight: 600; text-transform: uppercase;
      letter-spacing: 0.1em; color: var(--text-muted);
    }
    .countdown-timer {
      font-family: 'JetBrains Mono', monospace;
      font-size: 56px; font-weight: 700;
      color: var(--accent-cyan);
      text-shadow: 0 0 40px var(--accent-cyan-glow);
      letter-spacing: -0.02em;
      line-height: 1;
      transition: color 0.3s ease, text-shadow 0.3s ease;
    }
    .countdown-timer.urgent {
      color: var(--warning);
      text-shadow: 0 0 40px rgba(245, 158, 11, 0.3);
      animation: blink-warn 1s ease-in-out infinite;
    }
    .countdown-timer.critical {
      color: var(--danger);
      text-shadow: 0 0 40px var(--danger-glow);
      animation: blink-crit 0.5s ease-in-out infinite;
    }
    @keyframes blink-warn {
      0%, 100% { opacity: 1; }
      50% { opacity: 0.7; }
    }
    @keyframes blink-crit {
      0%, 100% { opacity: 1; }
      50% { opacity: 0.5; }
    }

    .timeout-action-badge {
      display: flex; align-items: center; gap: 8px;
      padding: 8px 16px;
      border-radius: var(--radius-full);
      font-size: 13px; font-weight: 600;
      background: var(--warning-bg);
      border: 1px solid rgba(245, 158, 11, 0.2);
      color: var(--warning);
    }
    .timeout-action-badge .badge-icon { font-size: 14px; }

    .unlock-status-icon {
      width: 64px; height: 64px;
      border-radius: 50%;
      background: var(--ok-bg);
      border: 2px solid rgba(34, 211, 238, 0.2);
      display: grid; place-items: center;
      font-size: 28px; flex-shrink: 0;
      animation: pulse-cyan 3s ease-in-out infinite;
    }
    @keyframes pulse-cyan {
      0%, 100% { box-shadow: 0 0 0 0 rgba(56, 189, 248, 0.15); }
      50% { box-shadow: 0 0 0 12px rgba(56, 189, 248, 0); }
    }

    .unlock-controls {
      display: flex; gap: 10px; flex-wrap: wrap;
    }
    .unlock-controls .ctrl-group {
      display: flex; flex-direction: column; gap: 5px;
    }
    .unlock-controls .ctrl-group.pin { flex: 1; min-width: 140px; }
    .unlock-controls .ctrl-group.dur { width: 100px; }
    .unlock-controls .ctrl-group.act { width: 150px; }
    .ctrl-label {
      font-size: 11px; font-weight: 600; text-transform: uppercase;
      letter-spacing: 0.06em; color: var(--text-muted); padding-left: 2px;
    }

    input, select {
      font: inherit; font-size: 14px; padding: 10px 12px;
      background: var(--bg-input); border: 1px solid var(--border-default);
      border-radius: var(--radius-sm); color: var(--text-primary);
      transition: all 0.15s ease; width: 100%;
    }
    input:hover, select:hover { border-color: var(--border-accent); }
    input:focus, select:focus { outline: none; border-color: var(--accent-cyan); box-shadow: 0 0 0 3px var(--accent-cyan-glow); }
    input::placeholder { color: var(--text-muted); }

    .btn-row { display: flex; gap: 8px; align-items: flex-end; }

    .btn {
      font: inherit; font-size: 13px; font-weight: 600;
      padding: 10px 16px; border: none; border-radius: var(--radius-sm);
      cursor: pointer; transition: all 0.15s ease;
      display: inline-flex; align-items: center; justify-content: center; gap: 6px;
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
    .btn-danger { background: var(--danger-bg); color: var(--danger); border: 1px solid rgba(239, 68, 68, 0.3); }
    .btn-danger:hover { background: var(--danger); color: white; box-shadow: 0 4px 20px var(--danger-glow); }

    /* ── ENFORCEMENT CARDS ────────────────────────────── */
    .panel {
      background: var(--bg-panel);
      border: 1px solid var(--border-subtle);
      border-radius: var(--radius-lg);
      box-shadow: var(--shadow-md);
      overflow: hidden;
    }
    .panel-header {
      background: linear-gradient(180deg, rgba(255,255,255,0.02), transparent);
      border-bottom: 1px solid var(--border-subtle);
      padding: 16px 20px;
      display: flex; align-items: center; gap: 10px;
    }
    .panel-header h2 {
      margin: 0; font-size: 12px; font-weight: 600; text-transform: uppercase;
      letter-spacing: 0.08em; color: var(--text-muted);
    }
    .panel-header .icon { font-size: 14px; opacity: 0.6; }
    .panel-body { padding: 16px 20px; display: flex; flex-direction: column; gap: 12px; }

    .actions-grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 10px; }
    .action-card {
      background: var(--bg-card); border: 1px solid var(--border-subtle);
      border-radius: var(--radius-md); padding: 16px 12px; text-align: center;
      cursor: pointer; transition: all 0.2s ease;
      display: flex; flex-direction: column; align-items: center; gap: 8px;
    }
    .action-card:hover {
      border-color: var(--border-default); background: var(--bg-hover);
      transform: translateY(-2px); box-shadow: var(--shadow-sm);
    }
    .action-card.danger:hover { border-color: var(--danger); background: var(--danger-bg); }
    .action-icon {
      width: 40px; height: 40px; border-radius: var(--radius-sm);
      display: grid; place-items: center;
      font-size: 20px;
      background: rgba(100, 116, 139, 0.08);
    }
    .action-card.danger .action-icon { background: var(--danger-bg); }
    .action-label { font-size: 12px; font-weight: 600; color: var(--text-secondary); }

    .message {
      padding: 10px 14px; border-radius: var(--radius-sm);
      font-size: 13px; font-weight: 500;
      display: flex; align-items: center; gap: 8px;
    }
    .message.ok { background: var(--ok-bg); color: var(--ok); border: 1px solid rgba(34, 211, 238, 0.15); }
    .message.error { background: var(--danger-bg); color: var(--danger); border: 1px solid rgba(239, 68, 68, 0.15); }
    .message:empty { display: none; }

    /* ── SNAPSHOT ──────────────────────────────────────── */
    .snapshot-frame {
      background: var(--bg-card); border: 1px solid var(--border-subtle);
      border-radius: var(--radius-md); padding: 4px;
      min-height: 200px; display: flex; flex-direction: column;
    }
    .snapshot-empty {
      flex: 1; display: grid; place-items: center;
      border-radius: var(--radius-sm); border: 2px dashed var(--border-default);
      color: var(--text-muted); font-size: 13px; text-align: center; padding: 32px;
      background: var(--bg-input);
    }
    .snapshot-empty .icon { font-size: 36px; opacity: 0.25; margin-bottom: 12px; }
    .snapshot-img {
      width: 100%; max-height: 480px; object-fit: contain;
      border-radius: var(--radius-sm); background: var(--bg-deep);
    }

    .hidden { display: none !important; }

    /* ── MODAL ─────────────────────────────────────────── */
    .modal-overlay {
      position: fixed; inset: 0;
      background: rgba(3, 7, 18, 0.85);
      backdrop-filter: blur(8px);
      display: grid; place-items: center;
      z-index: 1000;
      opacity: 0; visibility: hidden;
      transition: all 0.25s ease;
    }
    .modal-overlay.active { opacity: 1; visibility: visible; }
    .modal {
      background: var(--bg-panel);
      border: 1px solid var(--border-default);
      border-radius: var(--radius-xl);
      box-shadow: var(--shadow-lg), var(--shadow-glow-cyan);
      width: 90%; max-width: 400px;
      transform: scale(0.95) translateY(10px);
      transition: transform 0.25s ease;
    }
    .modal-overlay.active .modal { transform: scale(1) translateY(0); }
    .modal-header {
      background: linear-gradient(180deg, rgba(255,255,255,0.03), transparent);
      border-bottom: 1px solid var(--border-subtle);
      padding: 20px 24px;
      display: flex; align-items: center; justify-content: space-between;
    }
    .modal-header h2 {
      font-size: 15px; font-weight: 600;
      display: flex; align-items: center; gap: 8px;
    }
    .modal-close {
      background: none; border: none;
      font-size: 22px; color: var(--text-muted);
      cursor: pointer; width: 32px; height: 32px;
      display: grid; place-items: center;
      border-radius: var(--radius-sm); transition: all 0.15s ease;
    }
    .modal-close:hover { background: var(--bg-hover); color: var(--text-primary); }
    .modal-body { padding: 20px 24px; display: flex; flex-direction: column; gap: 14px; }
    .change-pin-form { display: flex; flex-direction: column; gap: 12px; }
    .form-group { display: flex; flex-direction: column; gap: 5px; }
    .form-label { font-size: 12px; font-weight: 500; color: var(--text-secondary); }

    /* ── PROGRESS BAR ─────────────────────────────────── */
    .progress-bar-wrap {
      width: 100%; height: 4px;
      background: rgba(100, 116, 139, 0.1);
      border-radius: 2px; overflow: hidden;
    }
    .progress-bar {
      height: 100%; border-radius: 2px;
      background: linear-gradient(90deg, var(--accent-cyan), var(--accent-blue));
      transition: width 1s linear, background 0.3s ease;
      min-width: 0;
    }
    .progress-bar.urgent {
      background: linear-gradient(90deg, var(--warning), #f97316);
    }
    .progress-bar.critical {
      background: linear-gradient(90deg, var(--danger), #dc2626);
    }

    /* ── RESPONSIVE ────────────────────────────────────── */
    @media (max-width: 640px) {
      body { padding: 12px; }
      .hero-inner { padding: 24px 16px; }
      .countdown-timer { font-size: 40px; }
      .unlock-hero-row { flex-direction: column; gap: 16px; }
      .unlock-controls { flex-direction: column; }
      .unlock-controls .ctrl-group.pin,
      .unlock-controls .ctrl-group.dur,
      .unlock-controls .ctrl-group.act { width: 100%; }
      .actions-grid { grid-template-columns: 1fr; }
      .lock-title { font-size: 22px; }
      .status-strip { gap: 6px; }
    }
  </style>
</head>
<body>
  <div class="app">
    <!-- HEADER -->
    <header class="header">
      <div class="brand">
        <div class="brand-icon">&#x1f6e1;&#xfe0f;</div>
        <div class="brand-text">
          <h1>WinParentalControl</h1>
          <p>Session management &amp; enforcement</p>
        </div>
      </div>
      <div class="header-actions">
        <button type="button" class="btn-icon" id="settings-btn" title="Security Settings">&#x2699;&#xfe0f;</button>
      </div>
    </header>

    <!-- STATUS STRIP -->
    <div class="status-strip">
      <div class="status-chip">
        <span class="status-dot" id="mode-dot"></span>
        <span id="mode-label">Loading...</span>
      </div>
      <div class="status-chip">
        <span class="status-dot" id="agent-dot"></span>
        <span>Agent: <span id="agent-label">-</span></span>
      </div>
      <div class="status-chip">
        <span class="status-dot" id="session-dot"></span>
        <span>Session: <span id="session-label">-</span></span>
      </div>
    </div>

    <!-- HERO: STATE-DEPENDENT -->
    <div class="hero locked" id="hero">
      <div class="hero-inner">

        <!-- LOCKED VIEW -->
        <div class="lock-view" id="lock-view">
          <div class="lock-icon-wrap">&#x1f512;</div>
          <div class="lock-title">Locked</div>
          <div class="lock-form">
            <div class="pin-input-wrap">
              <span class="pin-input-icon">&#x1f511;</span>
              <input type="password" id="pin" placeholder="Enter parent PIN" autocomplete="off" />
            </div>
            <div class="unlock-options">
              <div class="opt-group">
                <div class="opt-label">Duration (min)</div>
                <input type="number" id="duration" min="-480" max="480" value="30" />
              </div>
              <div class="opt-group">
                <div class="opt-label">On Timeout</div>
                <select id="expiry-action">
                  <option value="app_lock">App Lock</option>
                  <option value="windows_lock">Windows Lock</option>
                  <option value="shutdown">Shutdown</option>
                </select>
              </div>
            </div>
            <button type="button" class="btn-unlock" id="unlock-btn">
              &#x1f513; Unlock
            </button>
          </div>
        </div>

        <!-- UNLOCKED VIEW -->
        <div class="unlock-view hidden" id="unlock-view">
          <div class="unlock-hero-row">
            <div class="unlock-status-icon">&#x1f513;</div>
            <div class="countdown-block">
              <div class="countdown-label">Time Remaining</div>
              <div class="countdown-timer" id="countdown">00:00</div>
              <div class="timeout-action-badge" id="timeout-badge">
                <span class="badge-icon">&#x26a0;&#xfe0f;</span>
                <span>On timeout: <strong id="timeout-action-text">App Lock</strong></span>
              </div>
            </div>
          </div>
          <div class="progress-bar-wrap">
            <div class="progress-bar" id="progress-bar" style="width: 100%"></div>
          </div>
          <div class="unlock-controls">
            <div class="ctrl-group pin">
              <div class="ctrl-label">PIN</div>
              <input type="password" id="pin-extend" placeholder="PIN" autocomplete="off" />
            </div>
            <div class="ctrl-group dur">
              <div class="ctrl-label">Minutes</div>
              <input type="number" id="duration-extend" min="-480" max="480" value="30" />
            </div>
            <div class="ctrl-group act">
              <div class="ctrl-label">On Timeout</div>
              <select id="expiry-action-extend">
                <option value="app_lock">App Lock</option>
                <option value="windows_lock">Windows Lock</option>
                <option value="shutdown">Shutdown</option>
              </select>
            </div>
            <div class="btn-row">
              <button type="button" class="btn btn-primary" id="extend-btn">Extend</button>
              <button type="button" class="btn btn-danger" id="lock-now-btn">Lock Now</button>
            </div>
          </div>
        </div>

        <div class="message" id="message" style="margin-top: 16px"></div>
      </div>
    </div>

    <!-- ENFORCEMENT PANEL -->
    <div class="panel">
      <div class="panel-header">
        <span class="icon">&#x26a1;</span>
        <h2>Immediate Actions</h2>
      </div>
      <div class="panel-body">
        <div class="actions-grid">
          <div class="action-card danger" id="lock-card">
            <div class="action-icon">&#x1f512;</div>
            <div class="action-label">Lock App</div>
          </div>
          <div class="action-card" id="windows-lock-card">
            <div class="action-icon">&#x1f5a5;&#xfe0f;</div>
            <div class="action-label">Lock Windows</div>
          </div>
          <div class="action-card danger" id="shutdown-card">
            <div class="action-icon">&#x23fb;</div>
            <div class="action-label">Shutdown</div>
          </div>
        </div>
      </div>
    </div>

    <!-- SNAPSHOT PANEL -->
    <div class="panel">
      <div class="panel-header">
        <span class="icon">&#x1f4f7;</span>
        <h2>Live Snapshot</h2>
        <button type="button" class="btn btn-secondary" id="snapshot-button" style="margin-left: auto; padding: 6px 14px; font-size: 12px;">Capture</button>
      </div>
      <div class="panel-body">
        <div class="snapshot-frame">
          <div class="snapshot-empty" id="snapshot-empty">
            <div>
              <div class="icon">&#x1f4f7;</div>
              <div>No snapshot captured</div>
            </div>
          </div>
          <img id="snapshot-image" class="snapshot-img hidden" alt="Session snapshot" />
        </div>
        <div id="snapshot-meta" style="font-size: 11px; color: var(--text-muted); text-align: center; margin-top: 6px;"></div>
      </div>
    </div>
  </div>

  <!-- SETTINGS MODAL -->
  <div class="modal-overlay" id="settings-modal">
    <div class="modal">
      <div class="modal-header">
        <h2>&#x1f511; Security Settings</h2>
        <button type="button" class="modal-close" id="modal-close">&times;</button>
      </div>
      <div class="modal-body">
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
          <button type="button" class="btn btn-primary" id="change-pin-btn" style="width: 100%;">Change PIN</button>
        </div>
        <div class="message" id="pin-message"></div>
      </div>
    </div>
  </div>

  <script>
    let token = null;
    let currentMode = "locked";
    let snapshotUrl = null;
    let countdownInterval = null;
    let serverExpiresAt = null;
    let totalDurationMs = null;

    function expiryActionLabel(action) {
      switch (action) {
        case "app_lock": case "agent_lock": return "App Lock";
        case "windows_lock": return "Windows Lock";
        case "shutdown": return "Shutdown";
        default: return "App Lock";
      }
    }

    function expiryActionIcon(action) {
      switch (action) {
        case "windows_lock": return "\u{1f5a5}\ufe0f";
        case "shutdown": return "\u23fb";
        default: return "\u{1f512}";
      }
    }

    function formatCountdown(seconds) {
      if (seconds <= 0) return "00:00";
      const hrs = Math.floor(seconds / 3600);
      const mins = Math.floor((seconds % 3600) / 60);
      const secs = seconds % 60;
      const mm = mins.toString().padStart(2, '0');
      const ss = secs.toString().padStart(2, '0');
      if (hrs > 0) return `${hrs.toString().padStart(2,'0')}:${mm}:${ss}`;
      return `${mm}:${ss}`;
    }

    function updateCountdownDisplay() {
      const el = document.getElementById("countdown");
      const bar = document.getElementById("progress-bar");
      if (!serverExpiresAt || currentMode === "locked") {
        el.textContent = "00:00";
        el.className = "countdown-timer";
        bar.style.width = "0%";
        return;
      }
      const now = Date.now();
      const remaining = Math.max(0, Math.floor((serverExpiresAt - now) / 1000));
      el.textContent = formatCountdown(remaining);

      // Progress bar
      if (totalDurationMs && totalDurationMs > 0) {
        const elapsed = totalDurationMs - (serverExpiresAt - now);
        const pct = Math.max(0, Math.min(100, ((totalDurationMs - elapsed) / totalDurationMs) * 100));
        bar.style.width = pct + "%";
      }

      // Urgency classes
      el.className = "countdown-timer";
      bar.className = "progress-bar";
      if (remaining <= 60) {
        el.classList.add("critical");
        bar.classList.add("critical");
      } else if (remaining <= 300) {
        el.classList.add("urgent");
        bar.classList.add("urgent");
      }

      if (remaining <= 0) {
        el.textContent = "00:00";
        el.className = "countdown-timer critical";
        bar.style.width = "0%";
      }
    }

    function startCountdown(expiresAtUtc) {
      const expiresMs = new Date(expiresAtUtc).getTime();
      if (!totalDurationMs || expiresMs !== serverExpiresAt) {
        totalDurationMs = expiresMs - Date.now();
      }
      serverExpiresAt = expiresMs;
      updateCountdownDisplay();
      if (countdownInterval) clearInterval(countdownInterval);
      countdownInterval = setInterval(updateCountdownDisplay, 1000);
    }

    function stopCountdown() {
      if (countdownInterval) { clearInterval(countdownInterval); countdownInterval = null; }
      serverExpiresAt = null;
      totalDurationMs = null;
      updateCountdownDisplay();
    }

    function switchView(mode) {
      currentMode = mode;
      const hero = document.getElementById("hero");
      const lockView = document.getElementById("lock-view");
      const unlockView = document.getElementById("unlock-view");

      if (mode === "unlocked") {
        hero.className = "hero unlocked";
        lockView.classList.add("hidden");
        unlockView.classList.remove("hidden");
      } else {
        hero.className = "hero locked";
        lockView.classList.remove("hidden");
        unlockView.classList.add("hidden");
      }
    }

    async function request(path, options = {}) {
      const headers = Object.assign({"Content-Type": "application/json"}, options.headers || {});
      if (token) headers["Authorization"] = "Bearer " + token;
      const response = await fetch(path, {...options, headers});
      const body = await response.json().catch(function() { return {}; });
      if (!response.ok) throw new Error(body.error || ("HTTP " + response.status));
      return body;
    }

    async function refresh() {
      try {
        const s = await request("/api/device/status", { method: "GET", headers: {} });

        // Status strip
        const modeDot = document.getElementById("mode-dot");
        const modeLabel = document.getElementById("mode-label");
        modeLabel.textContent = s.mode === "unlocked" ? "Unlocked" : "Locked";
        modeDot.className = "status-dot " + (s.mode === "unlocked" ? "online" : "offline");

        document.getElementById("agent-dot").className = "status-dot " + (s.agentHealthy ? "online" : "warning");
        document.getElementById("agent-label").textContent = s.agentHealthy ? "Healthy" : "Stale";

        document.getElementById("session-dot").className = "status-dot " + (s.protectedUserLoggedIn ? "online" : "offline");
        document.getElementById("session-label").textContent = s.protectedUserLoggedIn ? "Online" : "Offline";

        // Switch view
        switchView(s.mode);

        // Timeout badge (unlocked view)
        if (s.mode === "unlocked") {
          const actionText = expiryActionLabel(s.unlockExpiryAction || "app_lock");
          document.getElementById("timeout-action-text").textContent = actionText;

          if (s.unlockExpiryAction) {
            document.getElementById("expiry-action-extend").value = s.unlockExpiryAction;
          }

          if (s.unlockExpiresAtUtc) {
            startCountdown(s.unlockExpiresAtUtc);
          } else {
            stopCountdown();
          }
        } else {
          stopCountdown();
        }
      } catch (error) {
        setMessage(error.message, true);
      }
    }

    function setMessage(message, isError) {
      var node = document.getElementById("message");
      node.className = "message " + (isError ? "error" : "ok");
      node.textContent = message;
    }

    function setSnapshotMeta(message) {
      document.getElementById("snapshot-meta").textContent = message;
    }

    async function auth(pin) {
      var body = await request("/api/auth/pin", { method: "POST", body: JSON.stringify({ pin: pin }) });
      token = body.token;
    }

    async function ensureAuth() {
      if (!token) {
        var pinVal = document.getElementById("pin").value || document.getElementById("pin-extend").value;
        await auth(pinVal);
      }
    }

    // ── UNLOCK ──
    document.getElementById("unlock-btn").addEventListener("click", async function() {
      const pinEl = document.getElementById("pin");
      const pin = pinEl.value;
      const durHint = document.getElementById("duration").value || "30";
      const dur = Number.parseInt(durHint, 10);
      const act = document.getElementById("expiry-action").value;

      try {
        if (!Number.isFinite(dur) || dur < -480 || dur > 480) throw new Error("Duration: -480 to 480 min");

        await auth(pin);
        await request("/api/device/unlock", {
          method: "POST",
          body: JSON.stringify({ durationMinutes: dur, expiryAction: act })
        });
        setMessage("Unlocked for " + dur + " min \u2192 " + expiryActionLabel(act));
        await refresh();
      } catch (error) {
        setMessage(error.message, true);
      } finally {
        pinEl.value = "";
      }
    });

    // ── EXTEND ──
    document.getElementById("extend-btn").addEventListener("click", async function() {
      const pinEl = document.getElementById("pin-extend");
      const pin = pinEl.value;
      const durHint = document.getElementById("duration-extend").value || "30";
      const dur = Number.parseInt(durHint, 10);
      const act = document.getElementById("expiry-action-extend").value;

      try {
        if (!Number.isFinite(dur) || dur < -480 || dur > 480) throw new Error("Duration: -480 to 480 min");

        if (!token && pin) await auth(pin);
        await ensureAuth();
        await request("/api/device/extend", {
          method: "POST",
          body: JSON.stringify({ durationMinutes: dur, expiryAction: act })
        });
        setMessage("Extended by " + dur + " min \u2192 " + expiryActionLabel(act));
        totalDurationMs = null;
        await refresh();
      } catch (error) {
        setMessage(error.message, true);
      } finally {
        pinEl.value = "";
      }
    });

    // ── LOCK NOW (in unlocked view) ──
    document.getElementById("lock-now-btn").addEventListener("click", async function() {
      try {
        await ensureAuth();
        await request("/api/device/unlock", {
          method: "POST",
          body: JSON.stringify({ durationMinutes: 0, expiryAction: "app_lock" })
        });
        setMessage("Locked immediately");
        await refresh();
      } catch (error) { setMessage(error.message, true); }
    });

    // ── EXPIRY ACTION CHANGE (unlocked view) ──
    document.getElementById("expiry-action-extend").addEventListener("change", async function() {
      if (currentMode !== "unlocked") return;
      try {
        await ensureAuth();
        var act = document.getElementById("expiry-action-extend").value;
        await request("/api/device/expiry-action", { method: "POST", body: JSON.stringify({ expiryAction: act }) });
        setMessage("Timeout action: " + expiryActionLabel(act));
        await refresh();
      } catch (error) { setMessage(error.message, true); }
    });

    // ── IMMEDIATE ENFORCEMENT CARDS ──
    document.getElementById("lock-card").addEventListener("click", async function() {
      try {
        await ensureAuth();
        await request("/api/device/unlock", { method: "POST", body: JSON.stringify({ durationMinutes: 0, expiryAction: "app_lock" }) });
        setMessage("App lock applied");
        await refresh();
      } catch (error) { setMessage(error.message, true); }
    });

    document.getElementById("windows-lock-card").addEventListener("click", async function() {
      try {
        await ensureAuth();
        await request("/api/device/unlock", { method: "POST", body: JSON.stringify({ durationMinutes: 0, expiryAction: "windows_lock" }) });
        setMessage("Windows lock applied");
        await refresh();
      } catch (error) { setMessage(error.message, true); }
    });

    document.getElementById("shutdown-card").addEventListener("click", async function() {
      try {
        await ensureAuth();
        await request("/api/device/unlock", { method: "POST", body: JSON.stringify({ durationMinutes: 0, expiryAction: "shutdown" }) });
        setMessage("Shutdown requested");
        await refresh();
      } catch (error) { setMessage(error.message, true); }
    });

    // ── SNAPSHOT ──
    document.getElementById("snapshot-button").addEventListener("click", async function() {
      try {
        await ensureAuth();
        var response = await fetch("/api/device/snapshot", { method: "GET", headers: { Authorization: "Bearer " + token } });
        if (!response.ok) {
          var msg = "HTTP " + response.status;
          try { var b = await response.json(); msg = b.error || msg; } catch(e) {}
          throw new Error(msg);
        }
        var blob = await response.blob();
        if (snapshotUrl) URL.revokeObjectURL(snapshotUrl);
        snapshotUrl = URL.createObjectURL(blob);
        document.getElementById("snapshot-image").src = snapshotUrl;
        document.getElementById("snapshot-image").classList.remove("hidden");
        document.getElementById("snapshot-empty").classList.add("hidden");
        setSnapshotMeta("Captured at " + new Date().toLocaleTimeString());
      } catch (error) { setSnapshotMeta(error.message); }
    });

    // ── SETTINGS MODAL ──
    var settingsModal = document.getElementById("settings-modal");

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
    settingsModal.addEventListener("click", function(e) { if (e.target === settingsModal) closeModal(); });
    document.addEventListener("keydown", function(e) {
      if (e.key === "Escape" && settingsModal.classList.contains("active")) closeModal();
    });

    document.getElementById("change-pin-btn").addEventListener("click", async function() {
      var currentPin = document.getElementById("current-pin").value;
      var newPin = document.getElementById("new-pin").value;
      var confirmNewPin = document.getElementById("confirm-new-pin").value;
      var pinMsg = document.getElementById("pin-message");
      pinMsg.textContent = ""; pinMsg.className = "message";

      if (!currentPin || !newPin || !confirmNewPin) {
        pinMsg.textContent = "All fields are required"; pinMsg.classList.add("error"); return;
      }
      if (newPin !== confirmNewPin) {
        pinMsg.textContent = "New PINs do not match"; pinMsg.classList.add("error"); return;
      }
      if (newPin.length < 4) {
        pinMsg.textContent = "PIN must be at least 4 characters"; pinMsg.classList.add("error"); return;
      }
      try {
        await request("/api/auth/change-pin", { method: "POST", body: JSON.stringify({ currentPin: currentPin, newPin: newPin }) });
        pinMsg.textContent = "PIN changed successfully"; pinMsg.classList.add("ok");
        document.getElementById("current-pin").value = "";
        document.getElementById("new-pin").value = "";
        document.getElementById("confirm-new-pin").value = "";
        token = null;
        setTimeout(closeModal, 1500);
      } catch (error) {
        pinMsg.textContent = error.message; pinMsg.classList.add("error");
      }
    });

    // Enter key submits PIN
    document.getElementById("pin").addEventListener("keydown", function(e) {
      if (e.key === "Enter") document.getElementById("unlock-btn").click();
    });
    document.getElementById("pin-extend").addEventListener("keydown", function(e) {
      if (e.key === "Enter") document.getElementById("extend-btn").click();
    });

    refresh();
    setInterval(refresh, 5000);
  </script>
</body>
</html>
"#;
