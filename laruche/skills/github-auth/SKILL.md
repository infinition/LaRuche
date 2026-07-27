---
type: skill
name: github-auth
description: Authenticate to GitHub with a token, an SSH key, or the gh CLI.
---

# GitHub authentication

Everything else in the github family depends on this one. A push that asks for a password,
a `gh` that reports 404 on a repository that exists, a workflow that cannot be triggered:
all of it is authentication, and all of it looks like something else.

Establish the state first, then pick the path. Do not start by installing anything.

## Where the token comes from

`${GITHUB_TOKEN}` is served by the LaRuche secrets vault. Read it from the environment and
nowhere else: not from a `.env` you found, not from a file the user pasted, not from shell
history.

**Never print it.** Not to confirm it is set, not in an error message, not in a command you
echo before running. To check it exists, test its length or call an endpoint with it. The
moment a token reaches the transcript it is in the provider request and in the logs, and
it has to be revoked.

```bash
[ -n "${GITHUB_TOKEN}" ] && echo "token present" || echo "token absent"
```

## Step 1: find out what you already have

```bash
git --version
gh --version 2>/dev/null || echo "gh absent"
gh auth status 2>/dev/null || echo "gh not authenticated"
git config --global credential.helper 2>/dev/null || echo "no credential helper"
```

| What you see | Go to |
|---|---|
| `gh auth status` reports a logged-in account | nothing to do, use `gh` for everything |
| `gh` present, not authenticated | Path C |
| `gh` absent | Path A or B, neither needs an install or admin rights |

Inside a script, source the bundled helper instead. It sets `GH_AUTH_METHOD` to `gh`,
`curl` or `none`, plus `GITHUB_TOKEN`, `GH_USER`, `GH_OWNER`, `GH_REPO` and
`GH_OWNER_REPO` when it is in a repository with a GitHub remote:

```bash
source skills/github-auth/scripts/gh-env.sh
```

It is sourced, so it cannot signal by exit code. Read `GH_AUTH_METHOD`: `none` means every
call after it will come back 401.

## Path A: HTTPS with a personal access token

The path that works everywhere, including headless machines, with no install.

1. **Get a token.** Send the user to <https://github.com/settings/tokens>. Scopes:
   `repo` for anything to do with code, pull requests and issues; `workflow` only to touch
   GitHub Actions; `read:org` only for organisation repositories. Grant the least of these
   that covers the task.

   Fine-grained tokens work too and are better scoped, but they must list each repository
   explicitly and default to no permissions at all. A fine-grained token that returns 404
   on a repository the user can see in the browser is usually missing that repository, not
   expired.

   The token is displayed once. Have the user store it in the LaRuche vault, not in a file.

2. **Let git remember it.**

   ```bash
   git config --global credential.helper store
   git ls-remote https://github.com/<owner>/<repo>.git
   ```

   The second command triggers the prompt. The username is the GitHub login; **the
   password is the token**, not the account password. Success prints a list of refs.

   `store` writes the token in clear text to `~/.git-credentials`. On a shared machine use
   `git config --global credential.helper 'cache --timeout=28800'` instead, which keeps it
   in memory for eight hours and forgets it on reboot. On Windows, `manager` uses the
   Credential Manager and is better than either.

3. **Set the identity that will appear in commits.**

   ```bash
   git config --global user.name "<name>"
   git config --global user.email "<email>"
   ```

4. **Verify.** `git ls-remote` on a private repository succeeds, and
   `git config --global user.email` prints what you set. A public repository proves
   nothing: it answers without any credentials at all.

## Path B: SSH key

Better when the user pushes often, or juggles several accounts.

1. **Look before generating.** A second key for an account that already has one causes
   silent authentication against the wrong identity.

   ```bash
   ls -la ~/.ssh/id_*.pub 2>/dev/null || echo "no key"
   ```

2. **Generate, if there is none.**

   ```bash
   ssh-keygen -t ed25519 -C "<email>" -f ~/.ssh/id_ed25519 -N ""
   cat ~/.ssh/id_ed25519.pub
   ```

   `-N ""` means no passphrase, which is what makes it usable unattended and is a
   trade-off the user should be told about, not one you make quietly for them.

3. **The user adds the PUBLIC key** at <https://github.com/settings/keys>. Only the file
   ending in `.pub`. If you are ever about to display `id_ed25519` without the suffix,
   stop: that is the private key.

4. **Test, and read the answer.**

   ```bash
   ssh -T git@github.com
   ```

   Success is `Hi <username>! You've successfully authenticated, but GitHub does not
   provide shell access.` That sentence is the success case, in full. It exits non-zero,
   which is normal and not a failure.

5. **Route HTTPS remotes over SSH**, so existing clones keep working:

   ```bash
   git config --global url."git@github.com:".insteadOf "https://github.com/"
   ```

## Path C: the gh CLI

```bash
# a desktop with a browser
gh auth login

# headless, using the vault token
echo "${GITHUB_TOKEN}" | gh auth login --with-token
gh auth setup-git
```

`gh auth setup-git` is not optional on the token path: without it `gh` is authenticated
but plain `git push` is not, which produces the confusing state where every `gh` command
works and every push asks for a password.

Verify with `gh auth status`, which names the account and the scopes it holds.

## Without gh, without git

Any GitHub API call works with curl and the vault token:

```bash
curl -s -H "Authorization: Bearer ${GITHUB_TOKEN}" https://api.github.com/user
```

Success returns JSON with a `login` field. `Bearer` and the older `token` scheme both
work. This is the fallback the other github skills use when `GH_AUTH_METHOD` is `curl`.

## Traps

- **A public repository authenticates nothing.** Verify against something private, or with
  `/user`, which requires a real token.
- **The password prompt is asking for the token.** GitHub removed password authentication
  years ago. Typing the account password fails with a message about authentication, not
  about passwords.
- **A token in a remote URL leaks.** `git remote set-url origin https://user:TOKEN@...`
  works, and then the token sits in `.git/config`, in every `git remote -v`, and in the
  output of half the commands in the other github skills. Prefer a credential helper.
- **Scope errors read like permission errors.** `Permission to X denied` on a repository
  the user owns means the token lacks `repo`, not that the account lacks rights.
- **An expired token fails identically to a wrong one.** Check the expiry date before
  regenerating anything.
- **Do not run `gh auth login` without arguments in an automated context.** It is
  interactive, it will wait forever, and nothing will explain why.

## Failure modes

**`git push` asks for a username and password.** No credential helper, or `gh auth
setup-git` was never run. Set the helper, then push again; the prompt appears once more
and then never again.

**`fatal: Authentication failed` with a helper configured.** Stale credentials cached from
a token that has since expired. Clear them and re-authenticate:

```bash
printf "protocol=https\nhost=github.com\n\n" | git credential reject
```

**`remote: Permission to <repo> denied`.** The token is valid and under-scoped. Regenerate
with `repo`. For a fine-grained token, add this repository to its list.

**`ssh: connect to host github.com port 22: Connection refused`.** A network that blocks
port 22, which is most corporate and some mobile ones. Use SSH over 443 by adding to
`~/.ssh/config`:

```
Host github.com
  Hostname ssh.github.com
  Port 443
```

**Two accounts on one machine, pushing as the wrong one.** Give each a host alias in
`~/.ssh/config` with its own `IdentityFile`, and use `git@work-github:owner/repo.git` in
the remote. Global credential helpers cannot tell the accounts apart.

**`gh: command not found` and no administrator rights.** Do not try to install it. Path A
needs nothing beyond git, and every github skill here has a curl fallback.
