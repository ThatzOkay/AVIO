import eslintConfigPrettier from 'eslint-config-prettier'
import vue from 'eslint-plugin-vue'
import tseslint from 'typescript-eslint'
import vueParser from 'vue-eslint-parser'
import globals from 'globals'

// Biome lints plain .ts files; ESLint here only covers .vue SFCs (Biome has no Vue support).
// Prettier formats .vue files (Biome can't); eslint-config-prettier turns off the
// stylistic vue/* rules that vue.configs['flat/recommended'] enables so ESLint never
// fights Prettier's output.
export default tseslint.config(
  {
    ignores: ['target/**', 'dist/**', 'src-tauri/**'],
  },
  {
    files: ['src/**/*.vue'],
    extends: [...vue.configs['flat/recommended'], eslintConfigPrettier],
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
