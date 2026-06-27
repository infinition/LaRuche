---
type: skill
name: github-auth
description: "GitHub auth: HTTPS PAT, SSH keys, gh CLI login."
version: 1.2.0
license: MIT
platforms: [linux, macos, windows]
tools: [shell_exec]
scripts: [scripts/gh-env.sh]
metadata:
  laruche:
    tags: [GitHub, Authentication, Git, gh-cli, SSH, Setup]
    related_skills: [github-pr-workflow, github-code-review, github-issues, github-repo-management]
---

# GitHub Authentication Setup

Sets up authentication for GitHub repositories, PRs, issues, and CI. Two paths:

- **`git` (always available)** — HTTPS personal access tokens or SSH keys
- **`gh` CLI (if installed)** — richer API access, simpler auth flow

`GITHUB_TOKEN` is provided by the LaRuche secrets vault as `${GITHUB_TOKEN}` — never read from a `.env` file.

## Detection Flow

Run this first:

```bash
git --version
gh --version 2>/dev/null || echo "gh not installed"
gh auth status 2>/dev/null || echo "gh not authenticated"
git config --global credential.helper 2>/dev/null || echo "no git credential helper"
```

**Decision tree:**
1. `gh auth status` shows authenticated → use `gh` for everything
2. `gh` installed but not authenticated → Method 2 below
3. `gh` not installed → Method 1 below (no sudo needed)

To detect auth state in scripts, source the bundled helper (sets `GH_AUTH_METHOD`, `GITHUB_TOKEN`, `GH_USER`, `GH_OWNER`, `GH_REPO`):

```bash
source skills/github-auth/scripts/gh-env.sh
```

---

## Method 1: Git-Only Authentication (No gh, No sudo)

### Option A: HTTPS with Personal Access Token (Recommended)

**Step 1: Create a token**

Direct user to: **https://github.com/settings/tokens** → "Generate new token (classic)"

Required scopes:
- `repo` — read/write/push/PRs
- `workflow` — trigger GitHub Actions
- `read:org` — if working with org repos

Set expiration (90 days recommended). Copy the token — shown only once.

**Step 2: Configure git credential storage**

```bash
# Save credentials to disk (persistent)
git config --global credential.helper store

# Trigger auth — enter GitHub username + token (NOT password)
git ls-remote https://github.com/<username>/<any-repo>.git
```

Credentials are saved and reused for all future operations.

Alternatives:
```bash
# Cache in memory for 8 hours instead of saving to disk
git config --global credential.helper 'cache --timeout=28800'

# Embed token directly in remote URL (per-repo, no prompts)
git remote set-url origin https://<username>:${GITHUB_TOKEN}@github.com/<owner>/<repo>.git
```

**Step 3: Configure git identity**

```bash
git config --global user.name "Their Name"
git config --global user.email "their-email@example.com"
```

**Step 4: Verify**

```bash
git ls-remote https://github.com/<username>/<any-repo>.git
git config --global user.name && git config --global user.email
```

---

### Option B: SSH Key Authentication

**Step 1: Check for existing keys**

```bash
ls -la ~/.ssh/id_*.pub 2>/dev/null || echo "No SSH keys found"
```

**Step 2: Generate if needed**

```bash
ssh-keygen -t ed25519 -C "their-email@example.com" -f ~/.ssh/id_ed25519 -N ""
cat ~/.ssh/id_ed25519.pub
```

Direct user to add the public key at: **https://github.com/settings/keys** → "New SSH key"

**Step 3: Test the connection**

```bash
ssh -T git@github.com
# Expected: "Hi <username>! You've successfully authenticated..."
```

**Step 4: Rewrite HTTPS URLs to SSH**

```bash
git config --global url."git@github.com:".insteadOf "https://github.com/"
```

**Step 5: Configure git identity**

```bash
git config --global user.name "Their Name"
git config --global user.email "their-email@example.com"
```

---

## Method 2: gh CLI Authentication

**Interactive (desktop):**

```bash
gh auth login
# Select: GitHub.com → HTTPS → authenticate via browser
```

**Token-based (headless/SSH servers) — use vault token:**

```bash
echo "${GITHUB_TOKEN}" | gh auth login --with-token
gh auth setup-git
```

**Verify:**

```bash
gh auth status
```

---

## GitHub API Without gh

Use `curl` with the vault-provided token:

```bash
curl -s -H "Authorization: token ${GITHUB_TOKEN}" https://api.github.com/user
```

Extract token from git credentials if already configured locally:

```bash
grep "github.com" ~/.git-credentials 2>/dev/null | head -1 \
  | sed 's|https://[^:]*:\([^@]*\)@.*|\1|'
```

---

## Troubleshooting

| Problem | Solution |
|---------|----------|
| `git push` asks for password | GitHub disabled password auth — use a PAT as the password, or switch to SSH |
| `remote: Permission to X denied` | Token lacks `repo` scope — regenerate with correct scopes |
| `fatal: Authentication failed` | Stale cached credentials — run `git credential reject` then re-authenticate |
| `ssh: connect to host github.com port 22: Connection refused` | SSH over HTTPS port: add `Host github.com` / `Hostname ssh.github.com` / `Port 443` to `~/.ssh/config` |
| Credentials not persisting | Check `git config --global credential.helper` — must be `store` or `cache` |
| Multiple GitHub accounts | SSH with different keys per host alias in `~/.ssh/config`, or per-repo credential URLs |
| `gh: command not found` + no sudo | Use Method 1 — no installation needed |
