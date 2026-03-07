# Styling Contract

_Styling automation updates were applied by Codex tooling; see `.codex/**` for scripts, mappings, and reports._

This project is **CSS Modules first**.

## Rules

1. Use `*.module.css` for component styles and layout.
2. Use `src/styles/globals.css` only for:
   - reset/base layer
   - app-wide element behavior
   - third-party style overrides
3. Use `src/styles/variables.css` as the single source of theme tokens.
4. Avoid adding ad-hoc utility-class strings in JSX where module classes are clearer.

## Theme Tokens

- Use CSS custom properties (for example `var(--c-raw-012)`) in modules.
- Keep token names synchronized with `src/styles/variables.css`.

## Imports

- Global styles are imported once in `src/main.tsx`.
- Component files should not import global style files directly.
