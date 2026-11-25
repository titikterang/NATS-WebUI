import pluginVue from 'eslint-plugin-vue'
import tseslint from 'typescript-eslint'
import vueParser from 'vue-eslint-parser'

export default tseslint.config(
  { ignores: ['dist/**', 'node_modules/**'] },
  ...tseslint.configs.recommended,
  ...pluginVue.configs['flat/recommended'],
  {
    files: ['**/*.vue'],
    languageOptions: {
      parser: vueParser,
      parserOptions: { parser: tseslint.parser, extraFileExtensions: ['.vue'], sourceType: 'module' },
    },
  },
  {
    rules: {
      'vue/multi-word-component-names': 'off',
      'vue/max-attributes-per-line': 'off',
      'vue/singleline-html-element-content-newline': 'off',
      'vue/html-self-closing': 'off',
      // Architecture rule: components and stores talk to the backend only through
      // src/infrastructure. Importing axios elsewhere is an error.
      'no-restricted-imports': [
        'error',
        { paths: [{ name: 'axios', message: 'Use src/infrastructure/api.ts instead of axios directly.' }] },
      ],
    },
  },
  {
    files: ['src/infrastructure/**'],
    rules: { 'no-restricted-imports': 'off' },
  },
  {
    // The domain layer is framework-free: no Vue, Pinia, Element Plus or infrastructure.
    files: ['src/domain/**'],
    rules: {
      'no-restricted-imports': [
        'error',
        { patterns: ['vue', 'pinia', 'element-plus', 'axios', '@/infrastructure/*', '@/stores/*', '@/composables/*'] },
      ],
    },
  },
)
