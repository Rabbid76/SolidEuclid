import js from '@eslint/js';
import tseslint from 'typescript-eslint';
import eslintConfigPrettier from 'eslint-config-prettier';

export default tseslint.config(
  js.configs.recommended,
  ...tseslint.configs.recommended,
  eslintConfigPrettier,
  {
    ignores: [
      '.internal/',
      'build*/',
      'dist/',
      'node_modules/',
      'pkg/',
      '**/pkg/',
      'solid_euclid/target/',
      'target/',
    ],
  },
  {
    files: ['.agents/scripts/**/*.js'],
    languageOptions: {
      globals: {
        console: 'readonly',
        process: 'readonly',
      },
    },
  }
);
