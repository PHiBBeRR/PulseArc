import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import path from 'path';

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [react()],

  // Frontend root directory
  root: './frontend',

  // Vite options tailored for Tauri development
  clearScreen: false,

  // Tauri expects a fixed port, fail if that port is not available
  server: {
    port: 5173,
    strictPort: true,
    watch: {
      // Tell vite to ignore watching certain directories and database files
      ignored: [
        '**/crates/**',
        '**/legacy/**',
        '**/target/**',
        '**/node_modules/**',
        '**/.cargo/**',
        '**/.claude/**',
        '**/.git/**',
        '**/.husky/**',
        '**/.vscode/**',
        '**/*.db',
        '**/*.db-shm',
        '**/*.db-wal',
      ],
    },
  },

  // Path resolution
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './frontend'),
      '@components': path.resolve(__dirname, './frontend/components'),
      '@features': path.resolve(__dirname, './frontend/features'),
      '@shared': path.resolve(__dirname, './frontend/shared'),
    },
  },

  // Build options
  build: {
    // Output to dist directory inside frontend/
    outDir: './dist',
    emptyOutDir: true,
    // Tauri uses Chromium on Windows and WebKit on macOS and Linux
    target: process.env.TAURI_PLATFORM === 'windows' ? 'chrome105' : 'safari13',
    // Don't minify for debug builds
    minify: !process.env.TAURI_DEBUG ? 'esbuild' : false,
    // Produce sourcemaps for debug builds
    sourcemap: !!process.env.TAURI_DEBUG,
  },

  // Environment variable prefix
  envPrefix: ['VITE_', 'TAURI_'],
});
