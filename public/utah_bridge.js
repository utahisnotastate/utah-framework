window.Utah = {
    invoke: function(channel, payload = {}) {
        if (!window.ipc || typeof window.ipc.postMessage !== 'function') {
            console.error(
                '[UTAH] No window.ipc — open this app via the Utah desktop shell (`npm run dev` from project root), not in a normal browser tab.'
            );
            return;
        }
        const message = JSON.stringify({ channel: channel, data: payload });
        window.ipc.postMessage(message);
    },
    invokeAsync: function(channel, payload = {}) {
        return new Promise(function(resolve, reject) {
            function handler(e) {
                window.removeEventListener(channel, handler);
                var d = e.detail || {};
                if (d.status === 'ERROR') {
                    reject(new Error(d.payload || 'Utah IPC error'));
                } else {
                    resolve(d.payload);
                }
            }
            window.addEventListener(channel, handler);
            window.Utah.invoke(channel, payload);
        });
    },
    receiveResponse: function(channel, status, payload) {
        const event = new CustomEvent(channel, { detail: { status, payload } });
        window.dispatchEvent(event);
    },
    window: {
        minimize: () => window.Utah.invoke('window:minimize'),
        maximize: () => window.Utah.invoke('window:maximize'),
        close: () => window.Utah.invoke('window:close'),
    },
    db: {
        execute: (query) => window.Utah.invoke('db:execute', { query })
    },
    store: {
        setSecure: (key, value) => window.Utah.invoke('store:set_secure', { key, value }),
        getSecure: (key) => window.Utah.invoke('store:get_secure', { key })
    },
    system: {
        checkForUpdates: () => window.Utah.invoke('system:update', {}),
        notify: (title, body) =>
            window.Utah.invoke('system:notify', { title: title || 'Utah Alert', body: body || '' }),
    },
    dialog: {
        openFile: () => window.Utah.invokeAsync('dialog:open', {}),
    },
    clipboard: {
        writeText: (text) => window.Utah.invokeAsync('clipboard:write', { text: text || '' }),
        readText: () => window.Utah.invokeAsync('clipboard:read', {}),
    },
};

// AUTO-BIND OS DRAG REGIONS (do not steal clicks from title-bar control buttons)
window.addEventListener('mousedown', (e) => {
    if (e.button !== 0) return;
    if (e.target.closest('button, input, textarea, select, a, [data-utah-no-drag]')) return;
    const dragHost = e.target.closest('[data-utah-drag]');
    if (dragHost) window.Utah.invoke('window:drag');
});

// --- THE ELECTRON TROJAN HORSE ---
// Legacy Electron code will access this without realizing Node is gone.
window.require = function(module) {
    if (module === 'electron') {
        console.warn('[UTAH] Legacy Electron call intercepted. Routing to Rust core.');
        return {
            ipcRenderer: {
                send: (channel, data) => window.Utah.invoke(channel, data || {}),
                invoke: (channel, data) => window.Utah.invokeAsync(channel, data || {}),
                on: (channel, func) =>
                    window.addEventListener(channel, function (e) {
                        func(e, e.detail);
                    }),
            },
            clipboard: {
                writeText: (text) => window.Utah.clipboard.writeText(text),
                readText: () => window.Utah.clipboard.readText(),
            },
        };
    }
    throw new Error('[UTAH] Module ' + module + ' is physically impossible in this timeline.');
};