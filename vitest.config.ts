import { defineConfig } from 'vitest/config';
import { resolve } from 'path';
import wasm from 'vite-plugin-wasm';

export default defineConfig({
  plugins: [wasm()],
  resolve: {
    alias: {
      '@solid-euclid': resolve(import.meta.dirname, 'pkg'),
    },
  },
  test: {
    include: ['examples/web/__tests__/**/*.test.ts'],
    globals: false,
    testTimeout: 10000,
    pool: 'forks',
  },
});
