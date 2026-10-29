// ESLint for the stylesheet, `npm run lint`: syntax, unknown properties
// (`colr: red`, which Tailwind's build passes silently) and features too
// new for Baseline. The editor checks it as you type (.vscode/settings.json).
import css from "@eslint/css";
import { defineConfig } from "eslint/config";
import { tailwind4 } from "tailwind-csstree";

export default defineConfig([
  {
    // style/tailwind.css uses Tailwind v4 at-rules (@source, @theme, ...),
    // which plain CSS parsing flags as unknown; tailwind-csstree knows them.
    files: ["**/*.css"],
    plugins: { css },
    language: "css/css",
    languageOptions: {
      customSyntax: tailwind4,
    },
    extends: ["css/recommended"],
    rules: {
      // Custom properties declared inside `@theme` (Tailwind's token block)
      // are invisible to the rule's variable tracking, so `var(--color-x)`
      // would be reported as unknown. Tailwind itself fails the build on a
      // token that does not exist.
      "css/no-invalid-properties": ["error", { allowUnknownVariables: true }],
    },
  },
]);
