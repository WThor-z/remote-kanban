import { defineConfig } from 'vitest/config';
import path from 'path';

export default defineConfig({
  test: {
    exclude: ['dist/**', 'node_modules/**'],
  },
  resolve: {
    alias: {
      '@opencode-vibe/protocol': path.resolve(__dirname, '../protocol/src/index.ts'),
    },
  },
});
