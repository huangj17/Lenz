# Repository Guidelines

## Project Structure & Module Organization

Lenz is a Tauri 2 desktop application. The React/TypeScript frontend lives in `src/`: window UI is under `src/windows/`, with shared components and hooks in `src/components/` and `src/hooks/`. Rust code lives in `src-tauri/src/`; keep Tauri handlers in `commands/`, application logic in `core/`, and persisted settings in `config/`. Artwork belongs in `src/assets/` or `src-tauri/icons/`.

Treat `docs/PRD.md` as the product source of truth, with implementation detail in `docs/spec/` and milestones in `docs/TODO.md`. Keep all three synchronized when requirements change. `docs/design/project/` is a read-only visual reference, not production source.

## Build, Test, and Development Commands

- `pnpm install` installs JavaScript dependencies.
- `pnpm tauri dev` runs the desktop app with the Rust backend and Vite frontend.
- `pnpm build` type-checks and builds the frontend for production.
- `pnpm typecheck` performs a TypeScript-only check.
- `cargo check --manifest-path src-tauri/Cargo.toml` checks Rust compilation.
- `cargo test --manifest-path src-tauri/Cargo.toml` runs all Rust tests.
- `cargo test export_bindings --manifest-path src-tauri/Cargo.toml` regenerates `src/lib/bindings.ts` from Rust commands and types.

## Coding Style & Naming Conventions

Use two-space indentation, double quotes, and semicolons in TypeScript. Name React components and files in `PascalCase`, hooks with `useCamelCase`, and stores/utilities in `camelCase`. Prefer the `@/` alias for imports from `src/`. Keep TypeScript strict and resolve unused-symbol errors.

Format Rust with `cargo fmt`; use standard `snake_case` modules/functions and `PascalCase` types. Keep serialized IPC fields in `camelCase`. Do not hand-edit generated `src/lib/bindings.ts`; update the Rust contract and regenerate it. Avoid `serde_json::Value` in Specta-exposed command signatures because it can break type export.

## Testing Guidelines

Rust unit tests live beside implementation code in `#[cfg(test)]` modules and use descriptive `snake_case` names. Add regression tests for config migrations, translation adapters, capture geometry, and other pure logic. There is no frontend test runner or coverage threshold; validate UI work with `pnpm typecheck`, `pnpm build`, and the affected Tauri window. Document manual platform checks, especially accessibility, OCR, screen-recording, and multi-monitor behavior.

## Commit & Pull Request Guidelines

Follow the existing Conventional Commit pattern, such as `feat(popup): add language swap` or `fix(capture): hide selection overlay`. Keep commits focused. Pull requests should describe user-visible behavior, link the relevant issue or spec, list commands run, and include screenshots or recordings for UI changes. Note tested platforms and any new permissions or configuration. Never commit API keys, `.env` files, or local translator credentials.
