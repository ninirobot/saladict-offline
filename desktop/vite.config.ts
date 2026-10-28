import { defineConfig } from 'vite'
import preact from '@preact/preset-vite'

// Tauri 会以 file 方式加载 dist，因此全部内联为单页资源，不做代码分割
export default defineConfig({
  plugins: [
    preact(),
    {
      // Tauri 自定义协议不返回 CORS 头，Vite 注入的 crossorigin 会让资源加载失败
      name: 'strip-crossorigin',
      enforce: 'post',
      transformIndexHtml: (html: string) => html.replace(/\s+crossorigin/g, '')
    }
  ],
  base: './',
  clearScreen: false,
  server: {
    host: '0.0.0.0',
    port: 5183,
    strictPort: true,
    allowedHosts: true,
    watch: { ignored: ['**/src-tauri/**'] }
  },
  envPrefix: ['VITE_', 'TAURI_ENV_'],
  // 仓库根目录有扩展用的 postcss.config.js，这里显式禁用外部配置，避免串味
  css: { postcss: { plugins: [] } },
  build: {
    target: 'chrome110',
    outDir: 'dist',
    emptyOutDir: true,
    minify: 'esbuild',
    cssCodeSplit: false,
    sourcemap: false,
    reportCompressedSize: false,
    rollupOptions: {
      output: {
        inlineDynamicImports: true,
        entryFileNames: 'app.js',
        chunkFileNames: 'app.js',
        assetFileNames: 'app.[ext]'
      }
    }
  }
})
