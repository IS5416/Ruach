# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

Tauri 2 + React 19 + TypeScript desktop app (Vite 7 frontend, Rust backend). Project name: Ruach.

## Commands

- `npm run tauri dev` — full app in dev mode (Vite + Rust, hot reload on both sides)
- `npm run dev` — Vite only (frontend work; frontend served at `http://localhost:1420`, fixed port via `strictPort`)
- `npm run build` — typecheck (`tsc`) + frontend bundle
- `npm run tauri build` — release bundle (add `--debug` for debug build)
- `npm run tauri` — pass-through to Tauri CLI (`tauri add`, `tauri icon`, etc.)

Tests: Rust service layer has unit tests (`cargo test` in `src-tauri/`); no frontend test framework yet.

## Architecture

- Frontend: React SPA in `src/` (Vite + TypeScript). Talks to Rust only via Tauri IPC (`@tauri-apps/api/core` `invoke()`).
- Backend: Rust in `src-tauri/`. The crate lib is named `ruach_lib` (must stay distinct from the bin name on Windows); `src-tauri/src/main.rs` is a thin wrapper calling `ruach_lib::run()` from `src-tauri/src/lib.rs`.
- Commands: Rust functions annotated `#[tauri::command]`, registered in `lib.rs` via `.invoke_handler(tauri::generate_handler![...])`. New commands must be added to the handler to be callable from the frontend.
- Permissions: `src-tauri/capabilities/default.json` gates which plugins/IPC the webview can use. When adding a plugin via `tauri add <plugin>`, check the capability file and grant the permissions you need (default templates only grant what the scaffold needs).
- Tauri config: `src-tauri/tauri.conf.json` (bundle identifier, window, build config). `vite.config.ts` is Tauri-aware: fixed port 1420, ignores `src-tauri/` watching, HMR over port 1421.
- `src-tauri/target/` holds build artifacts — never touch; it's git-ignored.

## Conventions

- Frontend calls Rust with `invoke("command_name", args)` where `command_name` is the Rust fn name (snake_case); args must match the command's parameter names.
- Serde structs (`serde::Serialize`/`Deserialize`) are the standard way to pass structured data across IPC.

## Git commits

- Format: `type: description` (conventional commits, e.g. `feat:`, `fix:`, `docs:`, `refactor:`, `chore:`).
- Message must be in English (body lines too).
- No co-author trailer, no `@` symbols in commit messages or commands.
