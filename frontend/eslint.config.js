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
  // ── 架构约束（M11）：业务 views 禁止裸 OpenVue 组件 / 非 openicons 图标 ──
  // 见 Doc/17 §15.3 硬性规则 3、§17.3 禁止、§21.2 工程化。
  // views 只允许从 components/ui（Fp*）消费 UI；图标唯一来源 @openvue/openicons。
  {
    files: ['src/views/**/*.{vue,ts}'],
    rules: {
      'no-restricted-imports': [
        'error',
        {
          // 屏蔽所有底层 OpenVue 组件入口（openvue/* 深路径）。
          patterns: [{ group: ['openvue', 'openvue/*'], message: '业务 views 禁止直接 import OpenVue 底层组件，统一经 @/components/ui 的 Fp* 封装（Doc/17 §15.3/§17.3）。' }],
        },
      ],
    },
  },
  eslintConfigPrettier,
)
