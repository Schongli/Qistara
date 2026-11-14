import { svelte } from '@sveltejs/vite-plugin-svelte'
import { defineConfig } from 'vite'
import tauri from './src-tauri/tauri.conf.json' with { type: 'json' }

// https://vite.dev/config/
export default defineConfig({
  plugins: [svelte()],
  // the app version for the User-Agent (src/lib/ua.ts), from the one place it is set
  define: { __APP_VERSION__: JSON.stringify(tauri.version) },
  // cargo writes thousands of files under src-tauri/target while building; they are not frontend code
  server: { watch: { ignored: ['**/src-tauri/**'] } },
})
