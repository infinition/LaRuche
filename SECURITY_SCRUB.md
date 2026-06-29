# Security: purge leaked secrets from git history

Two secrets were committed and still live in git history (removing the files from
the working tree does NOT remove them from history):

1. **Telegram bot token** in `laruche/_archive/football_scraper.py`
   (`TOKEN = "***REDACTED***:AAH...GdM"`).
2. **cookie_secret** inside the stale `laruche/laruche-node/laruche-state.json`
   (already untracked now, but present in past commits).

## Step 1 - revoke / rotate first (do this regardless of history)

History rewriting does not undo exposure. Invalidate the secrets:

- **Telegram token**: open BotFather, `/revoke` the bot, generate a new token, and
  store it ONLY in the secrets vault (`@@secret/...`), never in a file.
- **cookie_secret**: if the value committed in `laruche-state.json` was ever the
  live one, rotate it (delete the `cookie_secret` field in the active
  `laruche/laruche-state.json`; the node regenerates one on next start). Existing
  sessions will be invalidated, which is the point.

## Step 2 - purge from history (optional, rewrites history)

This rewrites every commit, so coordinate if anyone else has a clone. Use
`git-filter-repo` (install: `pip install git-filter-repo`).

From the repo root:

```bash
# Remove the two files from ALL history.
git filter-repo --force \
  --path laruche/_archive/football_scraper.py \
  --path laruche/laruche-node/laruche-state.json \
  --invert-paths
```

Then, because the token also appears in commit content, also scrub the literal
string anywhere it slipped in:

```bash
# Replace the token string everywhere in history (belt and suspenders).
printf '***REDACTED***==>REDACTED\n' > /tmp/replacements.txt
git filter-repo --force --replace-text /tmp/replacements.txt
```

Verify it is gone:

```bash
git log --all -p -S '***REDACTED***' | head
git log --all --oneline -- laruche/_archive/football_scraper.py
```

Both should return nothing.

## Step 3 - force-push (only if there is a remote)

There is currently no git remote, so this is local-only. If you ever add a remote
after scrubbing, the first push must be `--force` and any existing clones must
re-clone (their old history still holds the secret).
