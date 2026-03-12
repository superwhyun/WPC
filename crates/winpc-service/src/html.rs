pub const INDEX_HTML: &str = r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>WinParentalControl</title>
  <style>
    :root {
      --bg: #0f172a;
      --panel: rgba(15, 23, 42, 0.9);
      --border: rgba(148, 163, 184, 0.25);
      --text: #e2e8f0;
      --muted: #94a3b8;
      --accent: #f59e0b;
      --danger: #ef4444;
      --ok: #22c55e;
    }
    body {
      margin: 0;
      font-family: ui-sans-serif, system-ui, sans-serif;
      color: var(--text);
      background:
        radial-gradient(circle at top, rgba(245, 158, 11, 0.28), transparent 30%),
        linear-gradient(180deg, #020617, #0f172a);
      min-height: 100vh;
      display: grid;
      place-items: center;
    }
    .panel {
      width: min(720px, calc(100vw - 32px));
      padding: 24px;
      background: var(--panel);
      border: 1px solid var(--border);
      border-radius: 20px;
      box-shadow: 0 24px 60px rgba(2, 6, 23, 0.45);
      backdrop-filter: blur(18px);
    }
    h1 {
      margin: 0 0 8px;
      font-size: 30px;
    }
    p {
      margin: 0 0 16px;
      color: var(--muted);
    }
    .status {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));
      gap: 12px;
      margin: 16px 0 24px;
    }
    .card, form {
      border: 1px solid var(--border);
      border-radius: 16px;
      padding: 14px;
      background: rgba(15, 23, 42, 0.65);
    }
    .label {
      font-size: 12px;
      text-transform: uppercase;
      color: var(--muted);
      letter-spacing: 0.08em;
      margin-bottom: 8px;
    }
    .value {
      font-size: 22px;
      font-weight: 700;
    }
    .row {
      display: grid;
      grid-template-columns: 1fr 1fr auto;
      gap: 12px;
      margin-bottom: 12px;
    }
    input, button {
      font: inherit;
      border-radius: 12px;
      border: 1px solid var(--border);
      padding: 12px 14px;
    }
    input {
      background: rgba(2, 6, 23, 0.7);
      color: var(--text);
    }
    button {
      cursor: pointer;
      color: #0f172a;
      background: var(--accent);
      font-weight: 700;
    }
    button.secondary {
      color: var(--text);
      background: transparent;
    }
    button.danger {
      background: var(--danger);
      color: white;
    }
    .actions {
      display: flex;
      gap: 12px;
      flex-wrap: wrap;
    }
    .snapshot {
      margin-top: 18px;
      border: 1px solid var(--border);
      border-radius: 16px;
      padding: 14px;
      background: rgba(15, 23, 42, 0.45);
    }
    .snapshot-header {
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 12px;
      margin-bottom: 12px;
    }
    .snapshot img {
      width: 100%;
      min-height: 220px;
      max-height: 420px;
      object-fit: contain;
      border-radius: 12px;
      border: 1px solid var(--border);
      background:
        radial-gradient(circle at top, rgba(245, 158, 11, 0.12), transparent 28%),
        rgba(2, 6, 23, 0.88);
    }
    .snapshot-empty {
      display: grid;
      place-items: center;
      min-height: 220px;
      border-radius: 12px;
      border: 1px dashed var(--border);
      color: var(--muted);
      background: rgba(2, 6, 23, 0.5);
      text-align: center;
      padding: 16px;
    }
    .hidden {
      display: none;
    }
    .message {
      margin-top: 12px;
      min-height: 24px;
      color: var(--muted);
    }
    .ok { color: var(--ok); }
    .error { color: var(--danger); }
  </style>
</head>
<body>
  <main class="panel">
    <h1>WinParentalControl</h1>
    <p>Local control page for lock, unlock and time-limited access.</p>
    <section class="status">
      <article class="card"><div class="label">Mode</div><div class="value" id="mode">-</div></article>
      <article class="card"><div class="label">Remaining</div><div class="value" id="remaining">-</div></article>
      <article class="card"><div class="label">Agent</div><div class="value" id="agent">-</div></article>
      <article class="card"><div class="label">User Session</div><div class="value" id="session">-</div></article>
    </section>
    <section id="auth-form">
      <div class="label">Parent PIN Session</div>
      <div class="row">
        <input type="password" id="pin" placeholder="PIN" />
        <input type="number" id="duration" min="1" max="480" value="30" />
        <button type="button" id="unlock">Unlock</button>
      </div>
      <div class="actions">
        <button type="button" class="secondary" id="extend">Extend</button>
        <button type="button" class="danger" id="lock">Lock now</button>
        <button type="button" class="secondary" id="windows-lock">Windows lock</button>
        <button type="button" class="danger" id="shutdown">Shut down</button>
      </div>
      <div class="message" id="message"></div>
    </section>
    <section class="snapshot">
      <div class="snapshot-header">
        <div>
          <div class="label">Live Snapshot</div>
          <p id="snapshot-meta">Capture the current child session on demand.</p>
        </div>
        <button type="button" class="secondary" id="snapshot-button">Capture snapshot</button>
      </div>
      <div id="snapshot-empty" class="snapshot-empty">No snapshot captured yet.</div>
      <img id="snapshot-image" class="hidden" alt="Current child session snapshot" />
    </section>
  </main>
  <script>
    let token = null;
    let currentModeState = { mode: "locked" };
    let snapshotUrl = null;

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
        document.getElementById("mode").textContent = status.mode;
        document.getElementById("remaining").textContent = `${status.remainingMinutes} min`;
        document.getElementById("agent").textContent = status.agentHealthy ? "healthy" : "stale";
        document.getElementById("session").textContent = status.protectedUserLoggedIn ? "online" : "offline";
        syncActions(status);
      } catch (error) {
        setMessage(error.message, true);
      }
    }

    function syncActions(status) {
      const unlockButton = document.getElementById("unlock");
      const extendButton = document.getElementById("extend");
      if (status.mode === "unlocked") {
        unlockButton.classList.add("hidden");
        extendButton.classList.remove("hidden");
      } else {
        unlockButton.classList.remove("hidden");
        extendButton.classList.add("hidden");
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
      const body = await request("/api/auth/pin", {
        method: "POST",
        body: JSON.stringify({ pin }),
      });
      token = body.token;
    }

    function readDurationMinutes() {
      const raw = document.getElementById("duration").value.trim();
      const parsed = Number.parseInt(raw || "30", 10);
      if (!Number.isFinite(parsed) || parsed < 1 || parsed > 480) {
        throw new Error("Duration must be between 1 and 480 minutes.");
      }
      return parsed;
    }

    async function action(path) {
      const options = { method: "POST" };
      if (!path.endsWith("lock")) {
        options.body = JSON.stringify({ durationMinutes: readDurationMinutes() });
      }

      const result = await request(path, options);
      if (path.endsWith("lock")) {
        setMessage("Locked now.");
      } else {
        setMessage(`Mode: ${result.status.mode}, remaining ${result.status.remainingMinutes} min`);
      }
      return result;
    }

    async function ensureAuth() {
      if (!token) {
        await auth(document.getElementById("pin").value);
      }
    }

    async function captureSnapshot() {
      await ensureAuth();
      const response = await fetch("/api/device/snapshot", {
        method: "GET",
        headers: { Authorization: `Bearer ${token}` },
      });
      if (!response.ok) {
        let message = `HTTP ${response.status}`;
        try {
          const body = await response.json();
          message = body.error || message;
        } catch {}
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
        setMessage("Already unlocked. Use Extend or Lock now.");
        return;
      }
      try {
        await auth(document.getElementById("pin").value);
        await action("/api/device/unlock");
        await refresh();
      } catch (error) {
        setMessage(error.message, true);
      }
    });

    document.getElementById("extend").addEventListener("click", async () => {
      try {
        await ensureAuth();
        await action("/api/device/extend");
        await refresh();
      } catch (error) {
        setMessage(error.message, true);
      }
    });

    document.getElementById("lock").addEventListener("click", async () => {
      try {
        await ensureAuth();
        await action("/api/device/lock");
        await refresh();
      } catch (error) {
        setMessage(error.message, true);
      }
    });

    document.getElementById("windows-lock").addEventListener("click", async () => {
      try {
        await ensureAuth();
        await request("/api/device/windows-lock", { method: "POST" });
        setMessage("Windows lock requested.");
      } catch (error) {
        setMessage(error.message, true);
      }
    });

    document.getElementById("shutdown").addEventListener("click", async () => {
      try {
        await ensureAuth();
        await request("/api/device/shutdown", { method: "POST" });
        setMessage("Shutdown requested.");
      } catch (error) {
        setMessage(error.message, true);
      }
    });

    document.getElementById("snapshot-button").addEventListener("click", async () => {
      try {
        await captureSnapshot();
      } catch (error) {
        setSnapshotMeta(error.message);
      }
    });

    refresh();
    setInterval(refresh, 5000);
  </script>
</body>
</html>
"#;
