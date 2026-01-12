module.exports = {
  root: true,
  env: {
    browser: true,
    es2022: true
  },
  parserOptions: {
    ecmaVersion: 'latest',
    sourceType: 'module',
    ecmaFeatures: {
      jsx: true
    }
  },
  plugins: ['react', 'react-hooks', 'react-refresh'],
  extends: [
    'eslint:recommended',
    'plugin:react/recommended',
    'plugin:react-hooks/recommended'
  ],
  settings: {
    react: {
      version: 'detect'
    }
  },
  rules: {
    'react/prop-types': 'off',
    'react-refresh/only-export-components': 'warn',
    'no-unused-vars': 'warn',
    'no-dupe-keys': 'warn',
    'react/no-unknown-property': 'warn'
  },
  overrides: [
    {
      files: ['*.cjs'],
      env: {
        node: true
      }
    }
  ],
  ignorePatterns: ['node_modules/', 'dist/']
}
