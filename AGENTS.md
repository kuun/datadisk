# Repository Guidelines

## Project Structure & Module Organization
- Backend entrypoint in `main.go`; configuration helpers in `config/`, logging in `logger/`, auth via Casbin in `auth/`, and HTTP wiring in `routers/` using Gin.
- Domain logic and persistence: `models/` (GORM models), `file/` (file operations and tasks), `webdav/` (WebDAV server and protocol helpers), `taskmgr/` (task scheduling), `message/` (websocket hub), and `audit/` (operation logging).
- Frontend Vue app lives in `webapp/` (`src/` for code, `dist/` for built assets, `node_modules/` for deps).
- Configuration and seeds in `etc/` (`datadisk_example.ini`, `datadisk.ini`, `casbin_model.conf`, SQL seed). Test fixtures sit under `test/` and `testdir/`.

## Build, Test, and Development Commands
- Backend dev: `go run . -config etc/datadisk.ini` to start the API with the provided config.
- Backend tests: `go test ./...` (runs WebDAV and file operation suites; ensure fixture paths like `testdir/` remain intact).
- Frontend setup: `cd webapp && npm install` (once), then `npm run dev` for local dev, `npm run build` to emit `webapp/dist`, and `npm run lint` for JS/Vue linting.
- Formatting/linting: `gofmt -w <files>` and `go vet ./...` before committing backend changes.

## Coding Style & Naming Conventions
- Go: follow `gofmt` output; package names are short/lowercase; exported types/functions use PascalCase; keep errors wrapped with context (see `pkg/errors` usage); prefer dependency injection over globals where possible.
- Vue: Composition API with single-file components; use kebab-case filenames in `webapp/src`, PascalCase components, and 2-space indentation per ESLint defaults.

## Testing Guidelines
- Go tests live in `_test.go` files (e.g., `webdav/*_test.go`, `file/*_test.go`); write table-driven tests where practical.
- Keep deterministic fixtures: avoid mutating `testdir/` contents in assertions; create temp dirs for destructive cases.
- No frontend tests currently; at minimum, run `npm run lint` on UI changes.

## Commit & Pull Request Guidelines
- Commits: use a short imperative subject plus a body that lists the main changes (modules touched, configs updated, user-visible effects); reference issues with `#id` when applicable.
- PRs: include a concise summary, setup steps (config flags, env vars), and test evidence (`go test`, `npm run lint`, screenshots for UI). Note security-impacting changes (auth, file access, WebDAV) explicitly.

## Configuration & Security Notes
- Copy `etc/datadisk_example.ini` to `etc/datadisk.ini` and adjust `addr`, `root_dir`, database DSN, and `casbin_conf` paths before running. `etc/casbin_model.conf` holds the RBAC model used by `auth/`.
- Keep secrets (DB passwords, doc service secrets) out of version control; prefer local `.ini` overrides. Ensure `config/Config.InitedPath` points to a writable location for initialization markers.
