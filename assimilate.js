#!/usr/bin/env node
'use strict';

/**
 * Utah assimilation: run from an existing (Electron) project root.
 * Invoked via: npx utah-framework --assimilate [--sync-core]
 */
const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

const syncCore =
    process.argv.includes('--sync-core') ||
    process.env.UTAH_ASSIMILATE_SYNC_CORE === '1';

const packageRoot = path.join(__dirname);
const cwd = process.cwd();

const log = (m) => console.log(m);

log('[ZEO-ARCHITECT L6] Initiating Electron Assimilation Protocol...');
log('--> Target directory: ' + cwd);

const pkgPath = path.join(cwd, 'package.json');
if (!fs.existsSync(pkgPath)) {
    console.error('[UTAH] No package.json in current directory. Run from your project root.');
    process.exit(1);
}

let pkg = JSON.parse(fs.readFileSync(pkgPath, 'utf8'));
pkg.dependencies = pkg.dependencies || {};
pkg.devDependencies = pkg.devDependencies || {};
pkg.scripts = pkg.scripts || {};

log('--> Purging Chromium & Node.js artifacts (Electron-era deps)...');
['electron', 'electron-store', 'electron-updater', 'sqlite3'].forEach((k) => {
    delete pkg.dependencies[k];
});
['electron', 'electron-builder'].forEach((k) => {
    delete pkg.devDependencies[k];
});

if (!pkg.scripts['dev:ui']) {
    pkg.scripts['dev:ui'] = 'vite';
}
if (!pkg.scripts['build:ui']) {
    pkg.scripts['build:ui'] = 'vite build';
}

pkg.scripts['dev:utah'] =
    'npx concurrently -n ui,core -c cyan,magenta "npm run dev:ui" "node scripts/wait-for-vite.mjs && cargo run"';
pkg.scripts['build:utah'] = 'npm run build:ui && cargo build --release';

fs.writeFileSync(pkgPath, JSON.stringify(pkg, null, 2) + '\n');

const publicDir = path.join(cwd, 'public');
if (!fs.existsSync(publicDir)) {
    fs.mkdirSync(publicDir, { recursive: true });
}

const bridgeSrc = path.join(packageRoot, 'public', 'utah_bridge.js');
if (!fs.existsSync(bridgeSrc)) {
    console.error('[UTAH] Missing bundled bridge at:', bridgeSrc);
    process.exit(1);
}
const bridgeCode = fs.readFileSync(bridgeSrc, 'utf8');
const polyfillPath = path.join(publicDir, 'utah_electron_polyfill.js');
fs.writeFileSync(polyfillPath, bridgeCode);
log('--> Wrote assimilation bridge: public/utah_electron_polyfill.js');

const waitSrc = path.join(packageRoot, 'scripts', 'wait-for-vite.mjs');
const scriptsOut = path.join(cwd, 'scripts');
if (fs.existsSync(waitSrc)) {
    if (!fs.existsSync(scriptsOut)) {
        fs.mkdirSync(scriptsOut, { recursive: true });
    }
    fs.copyFileSync(waitSrc, path.join(scriptsOut, 'wait-for-vite.mjs'));
    log('--> Installed scripts/wait-for-vite.mjs');
}

function patchIndexHtml(htmlPath) {
    if (!fs.existsSync(htmlPath)) return false;
    let html = fs.readFileSync(htmlPath, 'utf8');
    if (html.includes('utah_electron_polyfill.js')) return false;
    const tag = '    <script src="/utah_electron_polyfill.js"></script>\n';
    let next = html;
    if (/<script[^>]*\stype=["']module["']/i.test(html)) {
        next = html.replace(/<script[^>]*\stype=["']module["']/i, (m) => tag + m);
    } else if (html.includes('</head>')) {
        next = html.replace('</head>', tag + '  </head>');
    } else {
        return false;
    }
    fs.writeFileSync(htmlPath, next);
    return true;
}

const indexCandidates = [path.join(cwd, 'index.html'), path.join(cwd, 'public', 'index.html')];
let patched = false;
for (const p of indexCandidates) {
    if (patchIndexHtml(p)) {
        log('--> Patched ' + path.relative(cwd, p) + ' (Utah bridge script tag).');
        patched = true;
        break;
    }
}
if (!patched) {
    log('[UTAH] No index.html patched. Add before your module entry:');
    log('    <script src="/utah_electron_polyfill.js"></script>');
}

log('--> Materializing Rust core (cargo init if needed)...');
if (!fs.existsSync(path.join(cwd, 'Cargo.toml'))) {
    try {
        execSync('cargo init --name utah-core', { cwd, stdio: 'inherit' });
    } catch (e) {
        console.error('[UTAH] cargo init failed. Install Rust from https://rustup.rs/');
        process.exit(1);
    }
} else {
    log('[UTAH] Cargo.toml already present; skipping cargo init.');
}

if (syncCore) {
    log('--> --sync-core: overwriting src/main.rs and Cargo.toml from Utah template...');
    const mainRs = path.join(packageRoot, 'src', 'main.rs');
    const cargoToml = path.join(packageRoot, 'Cargo.toml');
    const dstMain = path.join(cwd, 'src', 'main.rs');
    if (!fs.existsSync(path.join(cwd, 'src'))) {
        fs.mkdirSync(path.join(cwd, 'src'), { recursive: true });
    }
    if (fs.existsSync(mainRs)) fs.copyFileSync(mainRs, dstMain);
    if (fs.existsSync(cargoToml)) fs.copyFileSync(cargoToml, path.join(cwd, 'Cargo.toml'));
    log('[UTAH] Core synced. Review Cargo.toml package name / version if needed.');
}

log('[ZEO-ARCHITECT] Assimilation complete. Electron-era deps removed from package.json.');
log("Run npm install (add concurrently if prompted) then npm run build:utah to build the native binary.");
log('Tip: npx utah-framework --assimilate --sync-core copies the full Rust Aegis worker from this package.');
