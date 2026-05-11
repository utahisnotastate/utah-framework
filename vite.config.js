import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { viteSingleFile } from 'vite-plugin-singlefile';
import { existsSync, unlinkSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const portFile = () => join(process.cwd(), '.utah-vite-port');

/** Writes the real bound port so `wait-for-vite.mjs` + Rust can find the desktop shell (not the browser). */
function utahVitePortPlugin() {
  return {
    name: 'utah-vite-port',
    configureServer(server) {
      try {
        if (existsSync(portFile())) unlinkSync(portFile());
      } catch {
        /* ignore */
      }
      server.httpServer?.once('listening', () => {
        const addr = server.httpServer?.address();
        const port = typeof addr === 'object' && addr && 'port' in addr ? addr.port : null;
        if (port) {
          try {
            writeFileSync(portFile(), String(port), 'utf8');
          } catch (e) {
            console.warn('[utah-vite-port] could not write .utah-vite-port:', e);
          }
        }
      });
    },
  };
}

export default defineConfig({
  plugins: [utahVitePortPlugin(), react(), viteSingleFile()],
  server: {
    host: '127.0.0.1',
    port: 5173,
    strictPort: false,
    open: false,
  },
  build: {
    target: 'esnext',
    assetsInlineLimit: 100000000,
    chunkSizeWarningLimit: 100000000,
    cssCodeSplit: false,
    brotliSize: false,
    rollupOptions: {
      inlineDynamicImports: true,
    },
  },
});
