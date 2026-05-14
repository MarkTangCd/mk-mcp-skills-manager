import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

const HOST = process.env.TAURI_DEV_HOST;

// Vite configuration tuned for Tauri 2 desktop development.
export default defineConfig(async () => ({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: HOST || false,
    hmr: HOST
      ? {
          protocol: 'ws',
          host: HOST,
          port: 1421,
        }
      : undefined,
    watch: {
      ignored: ['**/src-tauri/**'],
    },
  },
  test: {
    environment: 'jsdom',
    globals: true,
  },
}));
