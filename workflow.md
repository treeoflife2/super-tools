# Local Fork Workflow

This document covers how to run, build, update, and maintain a personal local
build of Clauge. The fork has three surgical changes from upstream:

- Telemetry scheduler neutralized (no heartbeat ever sent).
- Auto-updater + "What's New" GitHub fetch disabled at boot.
- Agent `cd` fix so Claude/Codex sessions land in the project directory.

All other features (REST, SQL, NoSQL, SSH, Explorer, Agent, Workspace, MCP
server) work normally. Cloud features are dormant unless you sign in.

---

## Where your data lives

Tauri uses the `identifier` in `tauri.conf.json` (`com.clauge.desktop`) to
derive the per-user data directory. As long as you do not change that
identifier, every build — dev, release, official-from-website — reads and
writes the same files, so updates never wipe your data.

On Windows, all of this lives outside the repo:

| Path | Contents |
|---|---|
| `%APPDATA%\com.clauge.desktop\clauge.db` | Main SQLite DB — REST collections, SQL/NoSQL/SSH/Explorer profiles, workspace boards/notes/cards, agent sessions, settings |
| `%APPDATA%\com.clauge.desktop\logs\` | Rolling logs (30-day retention) |
| `%APPDATA%\com.clauge.desktop\settings.json` | Advanced/diagnostic toggles |
| Windows Credential Manager | Cloud auth tokens (service `Clauge Cloud Auth`), SSH/Explorer secrets (separate services) |
| `~/.claude/`, `~/.codex/`, `~/.gemini/`, `~/.config/opencode/` | Per-CLI session logs and settings (owned by those CLIs, shared with raw CLI usage) |

The `src-tauri/target/` build artifacts in the repo are throwaway — wipe and
rebuild, your data is fine.

**Belt-and-braces snapshot before any risky change:**

```powershell
Copy-Item "$env:APPDATA\com.clauge.desktop\clauge.db" `
          "$env:APPDATA\com.clauge.desktop\clauge.db.bak"
```

---

## Development workflow

```powershell
bun install         # once (after clone, or when package.json changes)
bun tauri dev       # hot-reload frontend, recompile Rust on save
```

`bun tauri dev` runs Vite on `localhost:5173` for the frontend and a debug
Rust binary for the shell. Frontend edits hot-reload instantly; Rust edits
trigger a recompile (~10–60 seconds depending on change). It uses the same
`%APPDATA%\com.clauge.desktop\` directory as a release build, so all your real
data is live — handy, but do not run experimental migrations against your
real DB without backing up first.

---

## "I'm satisfied" workflow

```powershell
bun tauri build
```

Produces in `src-tauri/target/release/`:

- `clauge.exe` — standalone binary
- `bundle/nsis/Clauge_<version>_x64-setup.exe` — NSIS installer
- `bundle/msi/Clauge_<version>_x64_en-US.msi` — MSI installer

Two ways to run for real:

1. **Just launch the binary.** `src-tauri\target\release\clauge.exe`. No
   install step, no Start Menu entry. Data still lands in
   `%APPDATA%\com.clauge.desktop\`.
2. **Install via NSIS/MSI.** Gives you a Start Menu shortcut and a proper
   uninstaller. Same data path.

Tip: keep a desktop shortcut to `clauge.exe` from your build dir. Rebuilding
overwrites it in place, so nothing to reinstall each iteration.

---

## Pulling upstream updates without losing fork changes

```powershell
# One-time setup
git remote add upstream https://github.com/ansxuman/Clauge.git

# Each time you want updates
git fetch upstream
git merge upstream/main      # or: git rebase upstream/main
bun install                  # in case dependencies changed
bun tauri build              # or `bun tauri dev` to verify
```

Because every fork change was kept small and well-scoped, merges should stay
clean:

- Telemetry: module-level `#![allow(dead_code)]` + a no-op `spawn()` →
  merges cleanly even if upstream adds new bump sites or telemetry keys.
- Updater: commented-out calls inside one `try { … }` block in
  `+layout.svelte` → conflicts only if upstream rewrites that exact block.
- Agent `cd` fix: lives next to `apply_windows_env` in
  `src-tauri/src/modes/agent/terminal.rs` → conflicts only if upstream
  rewrites the spawn path.

To inspect local-only deltas before merging:

```powershell
git diff upstream/main -- `
  src-tauri/src/telemetry/ `
  src-tauri/src/modes/agent/terminal.rs `
  src/routes/+layout.svelte
```

---

## Recommended day-to-day loop

1. `git fetch upstream && git merge upstream/main` weekly (or whenever you
   want new features).
2. `bun install` if dependencies changed.
3. `bun tauri build`.
4. Double-click `src-tauri\target\release\clauge.exe` — same DB, same
   workspaces, same agent sessions as last time. Done.

For active hacking on the fork itself: leave `bun tauri dev` running in a
terminal and edit code in the IDE — hot reload handles the rest.

---

## Side-panel terminal in agent mode

Built in. Press **Ctrl+L** (Cmd+L on macOS) while a session is active to
toggle a shell panel on the right side of the agent view. The panel:

- Spawns in the session's project (or worktree) directory.
- Persists per session — each tab has its own shell scrollback.
- Has a drag-resizable split, find/search, case sensitivity, regex, whole
  word match.

There is also a toggle button in the top bar.

---

## Raw Claude CLI equivalence

Clauge's Claude session is just `claude` with optional flags. To use the raw
CLI manually with the same data:

```powershell
cd C:\path\to\your\project
claude
```

Full Clauge invocation, all flags included:

```
claude [--resume <session-id>] `
       [--dangerously-skip-permissions] `
       [--append-system-prompt '<purpose-prompt>']
```

Session state lives in `~/.claude/projects/<encoded-path>/*.jsonl`, where
each non-alphanumeric character of the project path is replaced with `-`.
Clauge and raw `claude` read/write the same files, so you can interleave
freely.
