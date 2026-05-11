import React from 'react';
import { useUtah } from './useUtah';

export default function App() {
    const [pingData, sendPing] = useUtah('system:ping');
    const [updateData, checkUpdate] = useUtah('system:update');

    const minimize = () => window.Utah.window.minimize();
    const maximize = () => window.Utah.window.maximize();
    const closeApp = () => window.Utah.window.close();

    return (
        <div style={{
            height: '100vh',
            display: 'flex',
            flexDirection: 'column',
            backgroundColor: '#0f172a',
            borderRadius: '12px',
            overflow: 'hidden',
            border: '1px solid #334155'
        }}>
            {/* THE CUSTOM TITLE BAR */}
            <div
                data-utah-drag="true"
                style={{
                    height: '40px',
                    backgroundColor: '#1e293b',
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'space-between',
                    padding: '0 15px',
                    cursor: 'grab',
                    userSelect: 'none'
                }}
            >
                <span style={{ color: '#94a3b8', fontSize: '14px', fontWeight: 'bold' }}>
                    UTAH FRAMEWORK v1.0
                </span>

                {/* WINDOW CONTROLS — stopPropagation so drag handler never eats clicks */}
                <div
                    data-utah-no-drag="true"
                    style={{ display: 'flex', gap: '8px' }}
                    onMouseDown={(e) => e.stopPropagation()}
                >
                    <button type="button" title="Minimize" onClick={(e) => { e.stopPropagation(); minimize(); }} style={btnStyle('#eab308')} />
                    <button type="button" title="Maximize" onClick={(e) => { e.stopPropagation(); maximize(); }} style={btnStyle('#22c55e')} />
                    <button type="button" title="Quit" onClick={(e) => { e.stopPropagation(); closeApp(); }} style={btnStyle('#ef4444')} />
                </div>
            </div>

            {/* MAIN CONTENT AREA */}
            <div style={{ padding: '20px', color: 'white' }}>
                <h1>The Post-Electron Paradigm</h1>
                <p>Click and drag the top bar to move the window natively.</p>
                <div style={{ display: 'flex', flexWrap: 'wrap', gap: '10px', marginTop: '12px' }}>
                    <button
                        onClick={() => sendPing({ timestamp: Date.now() })}
                        style={btnPrimary}
                    >
                        Ping Core
                    </button>
                    <button
                        onClick={() => checkUpdate({})}
                        style={btnSecondary}
                    >
                        Delta-Wave: Check updates
                    </button>
                </div>
                {pingData && (
                    <pre style={{ marginTop: '12px', color: '#22c55e' }}>
                        {JSON.stringify(pingData, null, 2)}
                    </pre>
                )}
                {updateData && (
                    <pre style={{ marginTop: '12px', color: '#a78bfa' }}>
                        {JSON.stringify(updateData, null, 2)}
                    </pre>
                )}
                <p style={{ marginTop: '16px', fontSize: '12px', color: '#94a3b8', maxWidth: '520px' }}>
                    Self-update reads GitHub Releases for repo set by{' '}
                    <code style={{ color: '#e2e8f0' }}>UTAH_UPDATE_REPO_OWNER</code> /{' '}
                    <code style={{ color: '#e2e8f0' }}>UTAH_UPDATE_REPO_NAME</code> (defaults Utah-1 / utah-framework).
                    Release assets must match <code style={{ color: '#e2e8f0' }}>self_update</code> expectations (zip with binary{' '}
                    <code style={{ color: '#e2e8f0' }}>utah-core</code> or override with{' '}
                    <code style={{ color: '#e2e8f0' }}>UTAH_UPDATE_BIN_NAME</code>).
                </p>
            </div>
        </div>
    );
}

// Helper for Mac-style buttons
const btnStyle = (color) => ({
    width: '14px',
    height: '14px',
    borderRadius: '50%',
    backgroundColor: color,
    border: 'none',
    cursor: 'pointer'
});

const btnPrimary = {
    padding: '8px 16px',
    backgroundColor: '#0ea5e9',
    border: 'none',
    borderRadius: '6px',
    color: 'white',
    cursor: 'pointer',
    fontWeight: 'bold'
};

const btnSecondary = {
    padding: '8px 16px',
    backgroundColor: '#6366f1',
    border: 'none',
    borderRadius: '6px',
    color: 'white',
    cursor: 'pointer',
    fontWeight: 'bold'
};

