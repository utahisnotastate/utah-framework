/**
 * Waits for Vite to write `.utah-vite-port` (authoritative), then verifies HTTP.
 * Falls back to scanning common ports if the file is missing (older setups).
 */
import http from 'node:http';
import fs from 'node:fs';
import path from 'node:path';

const ROOT = process.cwd();
const PORT_FILE = path.join(ROOT, '.utah-vite-port');
const FALLBACK_PORTS = [5173, 5174, 5175, 5176, 5177, 5178, 5179, 5180];
const TIMEOUT_MS = 120_000;
const POLL_MS = 120;

function looksLikeVite(body) {
  const b = body.toLowerCase();
  return (
    b.includes('vite') ||
    b.includes('@vite') ||
    b.includes('main.jsx') ||
    b.includes('/@fs/') ||
    b.includes('/@id/')
  );
}

function probePort(port) {
  return new Promise((resolve) => {
    const req = http.get(
      `http://127.0.0.1:${port}/`,
      { timeout: 1200 },
      (res) => {
        const chunks = [];
        res.on('data', (c) => chunks.push(c));
        res.on('end', () => {
          const body = Buffer.concat(chunks).toString('utf8');
          const code = res.statusCode ?? 0;
          const ok =
            code >= 200 &&
            code < 400 &&
            (looksLikeVite(body) || (body.length > 80 && body.includes('<!DOCTYPE')));
          resolve(ok ? port : null);
        });
      }
    );
    req.on('error', () => resolve(null));
    req.on('timeout', () => {
      req.destroy();
      resolve(null);
    });
  });
}

async function readPortFromFile() {
  try {
    if (!fs.existsSync(PORT_FILE)) return null;
    const raw = fs.readFileSync(PORT_FILE, 'utf8').trim();
    const p = parseInt(raw, 10);
    if (Number.isFinite(p) && p > 0 && p < 65536) return p;
  } catch {
    /* ignore */
  }
  return null;
}

async function waitForVite() {
  const deadline = Date.now() + TIMEOUT_MS;
  let logged = false;

  while (Date.now() < deadline) {
    const fromFile = await readPortFromFile();
    if (fromFile != null) {
      const hit = await probePort(fromFile);
      if (hit != null) {
        console.log(`[wait-for-vite] Vite ready (port file) http://127.0.0.1:${hit}`);
        return;
      }
    }

    for (const port of FALLBACK_PORTS) {
      const hit = await probePort(port);
      if (hit != null) {
        console.log(`[wait-for-vite] Vite ready (scan) http://127.0.0.1:${hit}`);
        return;
      }
    }

    if (!logged) {
      console.log(
        '[wait-for-vite] Waiting for Vite (.utah-vite-port or ports',
        FALLBACK_PORTS.join(', '),
        ')...'
      );
      logged = true;
    }
    await new Promise((r) => setTimeout(r, POLL_MS));
  }

  console.error('[wait-for-vite] Timed out: Vite never became ready.');
  process.exit(1);
}

await waitForVite();
