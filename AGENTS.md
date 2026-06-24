# AGENTS.md

## Repo Shape

- Rust workspace with four crates: `tmkpr-lib` core library, `tmkpr-cli` binary `tmkpr`, `tmkpr-ui` binary `tmkpr-ui`, and `tmkpr-pomodoro` binary `tmkpr-pomodoro`.
- Put shared domain, config, SQLite, migrations, models, NLP time parsing, and services in `tmkpr-lib`; interface crates should call library services/storage instead of duplicating persistence rules.
- SQLite migrations are inline Rust strings in `tmkpr-lib/src/storage/sqlite/migrations.rs`; update models, `Storage`, SQLite implementation, and affected interfaces together when changing schema.
- `Cargo.lock` is intentionally ignored; do not add it unless the project policy changes.

## Commands

- Pre-commit gate: `just check` runs `cargo fmt --all`, `cargo clippy --all-targets --all-features -- -D warnings`, then `cargo test --all`.
- Focused checks: `cargo test -p tmkpr-lib`, `cargo test -p tmkpr-cli`, `cargo test -p tmkpr-ui`, or `cargo test -p tmkpr-pomodoro`.
- Single Rust test: `cargo test -p <crate> <test_name>`.
- Build all crates: `just build` or `cargo build --all`; release build: `just build-release`.
- Run against disposable DB: `just cli <args>`, `just ui <args>`, and `just pomo <args>` use `/tmp/test-tmkpr.db`.
- Install all binaries locally: `just install`.

## Runtime Data

- Default config is `~/.config/tmkpr/config.toml`; loading config creates the file if missing.
- Default database is `~/.local/share/tmkpr/tmkpr.db`; SQLite opens with WAL and foreign keys enabled.
- Use `--db` or `TMKPR_DB` for `tmkpr` and `tmkpr-ui` when testing manually so real user data is not touched.
- `tmkpr-pomodoro` currently reads the DB path only from config despite README examples mentioning `TMKPR_DB`; `just pomo` passes `--db` but the current binary does not define that flag.
- UI state is stored separately at `~/.config/tmkpr/ui-state.toml` and may reference IDs from a different DB.

## Testing Notes

- Unit tests are inline in crate source files; there is no top-level `tests/` directory currently.
- Many storage-facing tests use `SqliteStorage::open_in_memory()`; prefer that for new unit tests unless a file path is the behavior under test.
- Pomodoro audio depends on system audio libraries at build time on Linux (`alsa-lib-devel` on Fedora/RHEL, `libasound2-dev` on Debian/Ubuntu).

## Release

- Release flow is scripted by `just release [VERSION]` / `just release-publish [VERSION]`, which runs `release.sh` and expects a clean worktree plus `gh`, `cargo`, `git`, and for publishing `curl`/`jq`/crates credentials.
- `release.sh` edits the workspace version, commits, creates/pushes tag `v<version>`, waits for GitHub Actions, and optionally publishes crates; do not run it as a verification shortcut.
