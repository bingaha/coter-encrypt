import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import { fileURLToPath, URL } from 'node:url'

// https://vite.dev/config/
export default defineConfig({
 plugins: [vue()],
 resolve: {
 alias: {
 '@': fileURLToPath(new URL('./src', import.meta.url))
 }
 },
 build: {
 rollupOptions: {
 output: {
 manualChunks(id) {
 const normalized = id.replace(/\\/g, '/')

 if (!normalized.includes('/node_modules/')) {
 return undefined
 }

 if (
 normalized.includes('/node_modules/vue/') ||
 normalized.includes('/node_modules/@vue/') ||
 normalized.includes('/node_modules/vue-router/') ||
 normalized.includes('/node_modules/pinia/')
 ) {
 return 'vendor-vue'
 }

 if (
 normalized.includes('/node_modules/naive-ui/') ||
 normalized.includes('/node_modules/vueuc/') ||
 normalized.includes('/node_modules/vooks/') ||
 normalized.includes('/node_modules/vdirs/') ||
 normalized.includes('/node_modules/treemate/') ||
 normalized.includes('/node_modules/css-render/') ||
 normalized.includes('/node_modules/@css-render/')
 ) {
 return 'vendor-naive'
 }

 if (normalized.includes('/node_modules/@vicons/ionicons5/')) {
 return 'vendor-icons'
 }

 if (
 normalized.includes('/node_modules/splitpanes/') ||
 normalized.includes('/node_modules/vue3-draggable-next/') ||
 normalized.includes('/node_modules/vuedraggable/')
 ) {
 return 'vendor-drag'
 }

 if (normalized.includes('/node_modules/@tauri-apps/')) {
 return 'vendor-tauri'
 }

 return 'vendor'
 }
 }
 }
 },
 server: {
 host: '0.0.0.0'
 }
})
