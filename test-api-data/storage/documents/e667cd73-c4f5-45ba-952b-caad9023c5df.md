---
tags:
- file
- kota-db
- ext_js
---
module.exports = {
  parser: '@typescript-eslint/parser',
  parserOptions: {
    ecmaVersion: 2022,
    sourceType: 'module',
  },
  plugins: ['@typescript-eslint'],
  rules: {
    // Basic TypeScript rules
    '@typescript-eslint/no-explicit-any': 'error',
    '@typescript-eslint/no-unused-vars': ['error', { argsIgnorePattern: '^_' }],
    '@typescript-eslint/prefer-const': 'error',
    '@typescript-eslint/no-var-requires': 'error',
    
    // Basic ESLint rules
    'no-console': 'warn',
    'prefer-const': 'error',
    'no-var': 'error',
    'no-duplicate-imports': 'error',
  },
  env: {
    node: true,
    es2022: true,
  },
};