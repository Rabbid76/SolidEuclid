import { defineConfig } from 'vite';
import { resolve } from 'path';
import wasm from 'vite-plugin-wasm';

export default defineConfig({
  root: 'examples/web',
  plugins: [wasm()],
  resolve: {
    alias: {
      '@solid-euclid': resolve(import.meta.dirname, 'pkg'),
    },
  },
  build: {
    outDir: resolve(import.meta.dirname, 'dist'),
    emptyOutDir: true,
    target: 'esnext',
    rollupOptions: {
      input: {
        main: resolve(import.meta.dirname, 'examples/web/index.html'),
        platonic_solid: resolve(import.meta.dirname, 'examples/web/platonic_solid/index.html'),
        sphere: resolve(import.meta.dirname, 'examples/web/sphere/index.html'),
        sweeps: resolve(import.meta.dirname, 'examples/web/sweeps/index.html'),
        extrude: resolve(import.meta.dirname, 'examples/web/extrude/index.html'),
        transform: resolve(import.meta.dirname, 'examples/web/transform/index.html'),
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
