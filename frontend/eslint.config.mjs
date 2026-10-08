import vue from 'eslint-plugin-vue'
import tseslint from 'typescript-eslint'
export default [
  ...tseslint.configs.recommended,
  ...vue.configs['flat/essential'],
  { files: ['src/**/*.vue'], languageOptions: { parserOptions: { parser: tseslint.parser, extraFileExtensions: ['.vue'] } } },
  { files: ['src/**/*.{ts,vue}'], rules: { 'vue/multi-word-component-names': 'off' } },
]
