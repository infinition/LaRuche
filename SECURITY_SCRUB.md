# Security: purge leaked secrets from git history

Two secrets were committed and still live in git history (removing the files from
the working tree does NOT remove them from history):

1. **Telegram bot token** in `laruche/_archive/football_scraper.py`
   (a `TOKEN = "<numeric_id>:<secret>"` literal).
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

Then, because the token also appears in commit content, scrub the literal string
anywhere it slipped in. Put the real token (followed by `==>REDACTED`) in a file
that is NOT committed, then run replace-text against it:

```bash
# scrub-rules.txt holds one line:  <the-real-token>==>REDACTED   (do not commit it)
git filter-repo --force --replace-text scrub-rules.txt
rm scrub-rules.txt
```

Verify it is gone (use the token's numeric id prefix to search):

```bash
git log --all -p -S '<token numeric id>' | head
```

This should return nothing.

## Step 3 - force-push (only if there is a remote)

There is currently no git remote, so this is local-only. If you ever add a remote
after scrubbing, the first push must be `--force` and any existing clones must
re-clone (their old history still holds the secret).
