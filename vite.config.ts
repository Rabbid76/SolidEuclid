import { defineConfig } from 'vite';
import { resolve } from 'path';
import wasm from 'vite-plugin-wasm';

export default defineConfig({
  root: 'examples/web',
  plugins: [wasm()],
  resolve: {
    alias: {
      '@solid-euclid': resolve(__dirname, 'pkg'),
    },
  },
  build: {
    outDir: resolve(__dirname, 'dist'),
    emptyOutDir: true,
    target: 'esnext',
    rollupOptions: {
      input: {
        main: resolve(__dirname, 'examples/web/index.html'),
        platonic_solid: resolve(__dirname, 'examples/web/platonic_solid/index.html'),
        sphere: resolve(__dirname, 'examples/web/sphere/index.html'),
        sweeps: resolve(__dirname, 'examples/web/sweeps/index.html'),
        extrude: resolve(__dirname, 'examples/web/extrude/index.html'),
      },
    },
  },
  server: {
    open: true,
  },
  optimizeDeps: {
    exclude: ['solid_euclid'],
  },
});
