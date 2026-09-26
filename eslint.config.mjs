// Shared ESLint config for every TS package (flat config, type-aware).
import js from "@eslint/js";
import reactHooks from "eslint-plugin-react-hooks";
import globals from "globals";
import tseslint from "typescript-eslint";

export default tseslint.config(
  {
    ignores: [
      "**/node_modules/**",
      "**/dist/**",
      "**/.expo/**",
      "**/ios/**",
      "**/android/**",
      "**/test-results/**",
      "**/playwright-report/**",
      "packages/types/src/bindings/**",
      "apps/backend/**",
      "target/**",
    ],
  },
  js.configs.recommended,
  ...tseslint.configs.recommendedTypeChecked,
  {
    languageOptions: {
      parserOptions: { projectService: true, tsconfigRootDir: import.meta.dirname },
      globals: { ...globals.browser, ...globals.node },
    },
    plugins: { "react-hooks": reactHooks },
    rules: {
      ...reactHooks.configs.recommended.rules,
      "@typescript-eslint/no-explicit-any": "error",
      "@typescript-eslint/no-unused-vars": ["error", { argsIgnorePattern: "^_", varsIgnorePattern: "^_" }],
      "@typescript-eslint/consistent-type-imports": ["error", { fixStyle: "inline-type-imports" }],
      // `onPress={() => void save()}` is the idiom for fire-and-forget handlers.
      "@typescript-eslint/no-misused-promises": ["error", { checksVoidReturn: { attributes: false } }],
    },
  },
  {
    // Plain JS config files (babel, metro, tailwind): CommonJS, no type info.
    files: ["**/*.{js,cjs,mjs}"],
    ...tseslint.configs.disableTypeChecked,
    languageOptions: {
      ...tseslint.configs.disableTypeChecked.languageOptions,
      sourceType: "commonjs",
      globals: globals.node,
    },
    rules: { ...tseslint.configs.disableTypeChecked.rules, "@typescript-eslint/no-require-imports": "off" },
  },
  {
    files: ["**/*.mjs", "eslint.config.mjs"],
    languageOptions: { sourceType: "module" },
  },
);
