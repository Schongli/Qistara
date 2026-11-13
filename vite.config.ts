import { svelte } from '@sveltejs/vite-plugin-svelte'
import { defineConfig } from 'vite'

// https://vite.dev/config/
export default defineConfig({
  plugins: [svelte()],
  // cargo writes thousands of files under src-tauri/target while building; they are not frontend code
  server: { watch: { ignored: ['**/src-tauri/**'] } },
})
