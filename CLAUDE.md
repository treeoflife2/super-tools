# Repo Rules for AI-Assisted Development

This is a **public fork**. Anything written to a file here may be pushed to a
public git remote. Treat every edit as if it will be world-readable forever.

## STRONG RULE — No personal information in the codebase

Never write personal or environment-identifying information into source
files, documentation, configuration, comments, commit messages, or any other
file tracked by git. This rule is non-negotiable and overrides convenience.

### Forbidden (do not write to any tracked file)

- Real names, usernames, email addresses, phone numbers.
- Absolute filesystem paths that contain a username
  (`C:\Users\<name>\…`, `/Users/<name>/…`, `/home/<name>/…`).
- API keys, tokens, OAuth client IDs/secrets, session IDs, passwords,
  private keys, SSH key material, `.env` contents, AWS access keys, GCP
  service-account JSON.
- Hostnames or IPs of personal/private infrastructure (home server,
  bastion, internal endpoints).
- Workplace/employer identifiers, internal project codenames, ticket IDs
  from private trackers.
- Git config that reveals identity (`user.name`, `user.email`).
- Browser/OS device fingerprint output, MAC addresses.
- Personal database names, S3 bucket names, real SSH profiles, real REST
  collection URLs.

### Required substitutions

When an example, comment, or doc needs a value that *could* be personal, use
neutral placeholders:

| Don't write | Write |
|---|---|
| `C:\Users\jane\repo\…` | `<project-root>` or `C:\path\to\project` |
| `/Users/jane/repo/…` | `~/path/to/project` |
| `jane@company.com` | `user@example.com` |
| `sk-ant-abc123…` | `<your-anthropic-key>` |
| `192.168.1.42` | `<host-ip>` |
| `acme-prod-db` | `<database-name>` |
| `my-bucket-secrets` | `<bucket-name>` |

### Before every commit

1. Run `git diff --staged` and scan for: a username segment in a path, an
   `@` followed by a real domain, anything matching `sk-`, `ghp_`, `gho_`,
   `xoxb-`, `AKIA`, `-----BEGIN`, `eyJ` (JWT), or 32+ char hex/base64 blobs
   that aren't checksums.
2. If unsure whether a value is personal, ask the human before committing.
3. Never paste shell output from this machine (which may contain absolute
   paths) into a tracked file without sanitizing first.

### Logs, scratch notes, and AI conversation transcripts

- Never save a Claude/Codex/etc. transcript into the repo.
- Never commit `*.log`, `.history`, `.bash_history`, `.zsh_history`.
- Scratch `.md` files used during a session must either be added to
  `.gitignore` or sanitized before commit.

### What to do if personal info slips in

1. Do **not** push. If already pushed, rotate any leaked credential
   immediately (revoke API keys, rotate SSH keys, change passwords) — git
   history rewrites do not erase what other clients/forks already pulled.
2. Remove the value from the working tree.
3. If the value was committed locally but not pushed:
   `git reset --soft HEAD~1`, sanitize, recommit.
4. If pushed: rotate the credential first, then consider `git filter-repo`
   to scrub history, then force-push only after coordinating with anyone
   who may have pulled.

## Other fork-maintenance rules

- This fork has three intentional changes vs upstream (telemetry off,
  updater off, agent `cd` fix). Keep each change small and local so
  `git merge upstream/main` stays painless. See `workflow.md`.
- License: PolyForm Noncommercial 1.0.0. Personal use only. No
  redistribution, no commercial deployment of this fork.
- Do not re-enable telemetry, the auto-updater, or any new outbound
  network call unless explicitly authorized by the human.
- Do not add CLAs, telemetry, analytics SDKs, error-reporting SDKs
  (Sentry/Bugsnag/etc.), or third-party HTTP beacons.
- Default to **no comments**. Only add a comment when the *why* is
  non-obvious. Never include personal context (e.g. "added for Jane's
  request") in comments.
- Verify before claiming success: build with `bun tauri build` (or run
  `bun tauri dev`) and confirm the change actually works.

## Process expectations

- Match the existing code style. Look at neighboring files before introducing
  a new pattern.
- Prefer editing existing files over creating new ones. Never create
  documentation files (`*.md`) unless the human explicitly asks.
- Keep diffs minimal so upstream merges remain trivial.
- When unsure about a sensitive change, stop and ask.
