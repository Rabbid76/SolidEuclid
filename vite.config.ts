import { defineConfig } from 'vite';
import { resolve } from 'path';

export default defineConfig({
  root: 'web/examples',
  build: {
    outDir: resolve(__dirname, 'dist'),
    emptyOutDir: true,
  },
  server: {
    open: true,
  },
});
