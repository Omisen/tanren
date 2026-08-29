import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

// La configurazione tiene conto di Tauri: porta fissa, niente pulizia dello
// schermo che nasconderebbe gli errori Rust, e watcher cieco su src-tauri.
// https://vite.dev/config/
export default defineConfig({
  plugins: [react()],

  // Tauri si aspetta una porta stabile e fallisce se non la trova.
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      ignored: ['**/src-tauri/**'],
    },
  },

  // Rende leggibili al frontend solo le variabili di ambiente di Tauri.
  envPrefix: ['VITE_', 'TAURI_ENV_'],
})
