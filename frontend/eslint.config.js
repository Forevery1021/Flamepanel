import eslintPluginVue from 'eslint-plugin-vue'
import tseslint from 'typescript-eslint'
import eslintConfigPrettier from 'eslint-config-prettier'
import globals from 'globals'

export default tseslint.config(
  {
    ignores: ['dist', 'node_modules', '**/*.js.map', '**/*.js'],
  },
  {
    files: ['**/*.{ts,vue}'],
    languageOptions: {
      ecmaVersion: 2022,
      sourceType: 'module',
      globals: { ...globals.browser, ...globals.es2021 },
    },
  },
  ...eslintPluginVue.configs['flat/recommended'],
  {
    files: ['**/*.{ts,mts,cts}'],
    extends: tseslint.configs.recommended,
  },
  {
    files: ['**/*.vue'],
    languageOptions: {
      parserOptions: {
        parser: tseslint.parser,
        extraFileExtensions: ['.vue'],
      },
    },
  },
  {
    files: ['**/*.{ts,vue}'],
    plugins: { '@typescript-eslint': tseslint.plugin },
    rules: {
      '@typescript-eslint/no-explicit-any': 'warn',
      '@typescript-eslint/no-unused-vars': ['error', { argsIgnorePattern: '^_' }],
      'no-console': ['warn', { allow: ['warn', 'error'] }],
      'vue/multi-word-component-names': ['error', { ignores: ['App', 'Layout', 'Sidebar'] }],
      'vue/max-attributes-per-line': 'off',
      'vue/html-self-closing': 'off',
      'vue/no-v-html': 'warn',
    },
  },
  eslintConfigPrettier,
)
