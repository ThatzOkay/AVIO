import vue from 'eslint-plugin-vue'
import tseslint from 'typescript-eslint'
import vueParser from 'vue-eslint-parser'
import globals from 'globals'

// Biome lints plain .ts files; ESLint here only covers .vue SFCs (Biome has no Vue support).
export default tseslint.config(
  {
    files: ['**/*.vue'],
    extends: [...vue.configs['flat/recommended']],
    languageOptions: {
      parser: vueParser,
      parserOptions: {
        parser: tseslint.parser,
        ecmaVersion: 'latest',
        sourceType: 'module',
      },
      globals: globals.browser,
    },
  },
  {
    // File-based routing (unplugin-vue-router, see typed-router.d.ts) names page
    // components after their route path, which is routinely a single word.
    files: ['src/pages/**/*.vue'],
    rules: {
      'vue/multi-word-component-names': 'off',
    },
  }
)
