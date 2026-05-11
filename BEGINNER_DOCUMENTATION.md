# Utah Framework: a guide for beginners (and non-technical users)

Welcome to Utah. If you can build a small website with HTML, CSS, and JavaScript—or use React—you can ship a **fast native desktop app** without learning Rust first. The Rust side is the “engine”; your JavaScript/React side is the dashboard.

This guide is for **beginners** and **non-technical users** who want a clear path from “clone the repo” to “I see my app in a window.”

---

## Step 1: install two standard tools

1. **Node.js** — manages web packages (React, Vite). Download from [nodejs.org](https://nodejs.org).
2. **Rust** — compiles the Utah native shell. Install from [rustup.rs](https://rustup.rs/) (use the default options unless you know you need something else).

Restart your terminal after installing so `node`, `npm`, and `cargo` are on your `PATH`.

---

## Step 2: run the app from the project root

Always run commands in the **folder that contains `package.json`** (not inside `src/`).

```bash
npm install
npm run dev
```

- **`npm install`** downloads the web toolchain (Vite, React, etc.).
- **`npm run dev`** starts Vite, waits until it is ready, then launches the **desktop window** (not “just a browser tab” by default—you want the window titled like Utah Protocol).

When you edit files under `src/` (for example `src/App.jsx`), Vite hot-reloads and the desktop UI updates.

---

## Step 3: “superpowers” from the `window.Utah` bridge

Websites in a normal browser cannot safely read arbitrary files, talk to SQLite, or use the OS password locker. In Utah’s **webview**, your UI talks to Rust through **`window.Utah`**.

### Secure vault (tokens / API keys)

Avoid putting secrets in `localStorage` for real products. Use the OS-backed vault:

```javascript
window.Utah.store.setSecure('user_api_key', 'sk-your-secret');

window.Utah.store.getSecure('user_api_key');
```

Results come back as **custom DOM events** on the same channel name (your React hook `useUtah` listens for them).

### SQLite (built-in database)

```javascript
window.Utah.db.execute("CREATE TABLE IF NOT EXISTS notes (id INTEGER PRIMARY KEY, body TEXT)");
window.Utah.db.execute("INSERT INTO notes (body) VALUES ('hello from Utah')");
```

You still design your own schema and queries. Errors return as event payloads with an `ERROR` status.

### Window controls (title bar buttons)

```javascript
window.Utah.window.minimize();
window.Utah.window.maximize();
window.Utah.window.close();   // quits the entire application
```

**Important:** in the current Utah template, **`close()` exits the process** so users can leave the app predictably. Use the **system tray menu → “Terminate Process”** for the same outcome, or click the red control in the custom title bar.

### Dragging a frameless window

Add **`data-utah-drag="true"`** to a bar or header region. Put **window control buttons outside** that region (or mark them with `data-utah-no-drag`) so clicks on **Minimize / Maximize / Quit** are not swallowed as drag gestures.

Example pattern:

```html
<div data-utah-drag="true" style="height: 40px; display: flex; align-items: center; padding: 0 12px;">
  <span>My App</span>
</div>
<div data-utah-no-drag="true">
  <button type="button" onclick="window.Utah.window.minimize()">Minimize</button>
  <button type="button" onclick="window.Utah.window.close()">Quit</button>
</div>
```

### Electron-style code (optional)

```javascript
const { ipcRenderer } = window.require('electron');
ipcRenderer.send('system:ping', { timestamp: Date.now() });
ipcRenderer.on('system:ping', (_event, detail) => console.log(detail));
```

Only the **`electron`** module is stubbed; other `require(...)` paths are blocked by design.

---

## Step 4: ship a release build

When you are ready to share a build:

```bash
npm run build
```

This produces a **stripped release binary** under `target/release/` (`utah-core.exe` on Windows). Your UI is baked into the binary as embedded HTML/JS from the Vite single-file output.

End users **do not need Node** installed to run that binary—they only need the OS webview runtime (WebView2 is standard on current Windows).

---

## Troubleshooting (beginner-friendly)

| Symptom | What to try |
| :--- | :--- |
| **`vite` not recognized** | Run `npm install` again from the repo root. |
| **Blank window / nothing loads** | Ensure `npm run dev` was started from the **root**; wait until the terminal shows Vite “ready” and the Rust shell locked onto the correct port. |
| **Buttons in the UI do nothing** | You must be inside the **Utah desktop webview** (`window.ipc` exists). A normal browser tab will not have IPC. |
| **Ping / update UI does not refresh** | Use the latest template: the event loop is woken when the worker replies so React state can update immediately. |

---

## Where to read next

- **Repository overview & comparison table:** [README.md](./README.md)
- **Rust IPC channels and env vars:** `src/main.rs` and `public/utah_bridge.js`

You do **not** need a separate “clearance level” or insider vocabulary to contribute—start from this guide, open issues when something is unclear, and we can improve this document for the next **beginner** who arrives.
