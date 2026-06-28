---
type: skill
name: github-repo-management
description: "Clone, create, fork, release repos via gh/curl."
version: 1.2.0
license: MIT
platforms: [linux, macos, windows]
tools: [shell_exec]
metadata:
  laruche:
    tags: [GitHub, Repositories, Git, Releases, Secrets, Actions]
---

# GitHub Repository Management

Create, clone, fork, configure, and manage GitHub repositories. Each section shows `gh` first, then `git` + `curl` as fallback.

## Prerequisites

`GITHUB_TOKEN` is injected by LaRuche's secret vault at execution. Detect available auth:

```bash
if command -v gh &>/dev/null && gh auth status &>/dev/null; then
  AUTH="gh"
  GH_USER=$(gh api user --jq '.login')
else
  AUTH="git"
  GH_USER=$(curl -s -H "Authorization: token ${GITHUB_TOKEN}" \
    https://api.github.com/user | python3 -c "import sys,json; print(json.load(sys.stdin)['login'])")
fi

# If inside an existing repo, extract owner/repo:
REMOTE_URL=$(git remote get-url origin)
OWNER_REPO=$(echo "$REMOTE_URL" | sed -E 's|.*github\.com[:/]||; s|\.git$||')
OWNER=$(echo "$OWNER_REPO" | cut -d/ -f1)
REPO=$(echo "$OWNER_REPO" | cut -d/ -f2)
```

---

## 1. Clone

```bash
# gh
gh repo clone owner/repo-name
gh repo clone owner/repo-name -- --depth 1

# git (works everywhere)
git clone https://github.com/owner/repo-name.git
git clone --depth 1 https://github.com/owner/repo-name.git
git clone --branch develop https://github.com/owner/repo-name.git ./my-dir
git clone git@github.com:owner/repo-name.git   # SSH
```

---

## 2. Create

```bash
# gh
gh repo create my-project --public --clone
gh repo create my-project --private --description "Desc" --license MIT --clone
gh repo create my-org/my-project --public --clone
gh repo create my-project --source . --public --push   # from existing local dir

# curl
curl -s -X POST -H "Authorization: token ${GITHUB_TOKEN}" \
  https://api.github.com/user/repos \
  -d '{"name":"my-project","description":"Desc","private":false,"auto_init":true,"license_template":"mit"}'

# For an org:
curl -s -X POST -H "Authorization: token ${GITHUB_TOKEN}" \
  https://api.github.com/orgs/my-org/repos \
  -d '{"name":"my-project","private":false}'

# Push existing local dir to new repo (curl path):
git init && git add . && git commit -m "Initial commit"
git remote add origin https://github.com/${GH_USER}/my-project.git
git push -u origin main
```

### From a Template

```bash
# gh
gh repo create my-app --template owner/template-repo --public --clone

# curl
curl -s -X POST -H "Authorization: token ${GITHUB_TOKEN}" \
  https://api.github.com/repos/owner/template-repo/generate \
  -d '{"owner":"'"${GH_USER}"'","name":"my-app","private":false}'
```

---

## 3. Fork

```bash
# gh (preferred - handles the wait automatically)
gh repo fork owner/repo-name --clone

# curl + git
curl -s -X POST -H "Authorization: token ${GITHUB_TOKEN}" \
  https://api.github.com/repos/owner/repo-name/forks
sleep 5   # GitHub needs a moment
git clone https://github.com/${GH_USER}/repo-name.git
cd repo-name
git remote add upstream https://github.com/owner/repo-name.git
```

### Keep Fork in Sync

```bash
git fetch upstream && git checkout main && git merge upstream/main && git push origin main

# gh shortcut
gh repo sync ${GH_USER}/repo-name
```

---

## 4. Repository Info

```bash
# gh
gh repo view owner/repo-name
gh repo list --limit 20
gh search repos "machine learning" --language python --sort stars

# curl
curl -s -H "Authorization: token ${GITHUB_TOKEN}" \
  https://api.github.com/repos/${OWNER}/${REPO} \
  | python3 -c "
import sys,json; r=json.load(sys.stdin)
print(r['full_name'], r['description'], 'Stars:', r['stargazers_count'], 'Branch:', r['default_branch'])"

curl -s -H "Authorization: token ${GITHUB_TOKEN}" \
  "https://api.github.com/user/repos?per_page=20&sort=updated" \
  | python3 -c "
import sys,json
for r in json.load(sys.stdin):
    print(f\"{r['full_name']:40} {'private' if r['private'] else 'public':8} ★{r['stargazers_count']}\")"
```

---

## 5. Repository Settings

```bash
# gh
gh repo edit --description "Updated" --visibility public
gh repo edit --enable-wiki=false --enable-issues=true --default-branch main
gh repo edit --add-topic "machine-learning,python" --enable-auto-merge

# curl (PATCH)
curl -s -X PATCH -H "Authorization: token ${GITHUB_TOKEN}" \
  https://api.github.com/repos/${OWNER}/${REPO} \
  -d '{"description":"Updated","has_wiki":false,"has_issues":true,"allow_auto_merge":true}'

# Update topics
curl -s -X PUT \
  -H "Authorization: token ${GITHUB_TOKEN}" \
  -H "Accept: application/vnd.github.mercy-preview+json" \
  https://api.github.com/repos/${OWNER}/${REPO}/topics \
  -d '{"names":["machine-learning","python","automation"]}'
```

---

## 6. Branch Protection

```bash
# View current protection
curl -s -H "Authorization: token ${GITHUB_TOKEN}" \
  https://api.github.com/repos/${OWNER}/${REPO}/branches/main/protection

# Set protection
curl -s -X PUT -H "Authorization: token ${GITHUB_TOKEN}" \
  https://api.github.com/repos/${OWNER}/${REPO}/branches/main/protection \
  -d '{
    "required_status_checks":{"strict":true,"contexts":["ci/test","ci/lint"]},
    "enforce_admins":false,
    "required_pull_request_reviews":{"required_approving_review_count":1},
    "restrictions":null
  }'
```

---

## 7. Secrets (GitHub Actions)

**Use `gh secret set` whenever possible - curl path requires NaCl encryption.**

```bash
# gh
gh secret set API_KEY --body "your-secret-value"
gh secret set SSH_KEY < ~/.ssh/id_rsa
gh secret list
gh secret delete API_KEY

# curl: fetch public key, encrypt with PyNaCl, then PUT
PUB=$(curl -s -H "Authorization: token ${GITHUB_TOKEN}" \
  https://api.github.com/repos/${OWNER}/${REPO}/actions/secrets/public-key)
KEY_ID=$(echo "$PUB" | python3 -c "import sys,json; print(json.load(sys.stdin)['key_id'])")
ENC=$(echo "$PUB" | python3 -c "
import sys,json
from base64 import b64encode
from nacl import encoding, public
d=json.load(sys.stdin)
box=public.SealedBox(public.PublicKey(d['key'].encode(), encoding.Base64Encoder))
print(b64encode(box.encrypt(b'your-secret-value')).decode())")
curl -s -X PUT -H "Authorization: token ${GITHUB_TOKEN}" \
  https://api.github.com/repos/${OWNER}/${REPO}/actions/secrets/API_KEY \
  -d "{\"encrypted_value\":\"$ENC\",\"key_id\":\"$KEY_ID\"}"

# List secrets (names only)
curl -s -H "Authorization: token ${GITHUB_TOKEN}" \
  https://api.github.com/repos/${OWNER}/${REPO}/actions/secrets \
  | python3 -c "import sys,json; [print(s['name']) for s in json.load(sys.stdin)['secrets']]"
```

---

## 8. Releases

```bash
# gh
gh release create v1.0.0 --title "v1.0.0" --generate-notes
gh release create v2.0.0-rc1 --draft --prerelease --generate-notes
gh release create v1.0.0 ./dist/binary --notes "Release notes"
gh release list
gh release download v1.0.0 --dir ./downloads

# curl: create release
curl -s -X POST -H "Authorization: token ${GITHUB_TOKEN}" \
  https://api.github.com/repos/${OWNER}/${REPO}/releases \
  -d '{"tag_name":"v1.0.0","name":"v1.0.0","body":"## Changelog\n- Feature A","draft":false,"prerelease":false,"generate_release_notes":true}'

# Upload asset (get RELEASE_ID from create response)
RELEASE_ID=$(curl -s -H "Authorization: token ${GITHUB_TOKEN}" \
  https://api.github.com/repos/${OWNER}/${REPO}/releases/latest \
  | python3 -c "import sys,json; print(json.load(sys.stdin)['id'])")

curl -s -X POST \
  -H "Authorization: token ${GITHUB_TOKEN}" \
  -H "Content-Type: application/octet-stream" \
  "https://uploads.github.com/repos/${OWNER}/${REPO}/releases/${RELEASE_ID}/assets?name=binary-amd64" \
  --data-binary @./dist/binary-amd64
```

---

## 9. GitHub Actions Workflows

```bash
# gh
gh workflow list
gh run list --limit 10
gh run view <RUN_ID> --log-failed
gh run rerun <RUN_ID> --failed
gh workflow run ci.yml --ref main
gh workflow run deploy.yml -f environment=staging

# curl: list workflows
curl -s -H "Authorization: token ${GITHUB_TOKEN}" \
  https://api.github.com/repos/${OWNER}/${REPO}/actions/workflows \
  | python3 -c "import sys,json; [print(w['id'],w['name'],w['state']) for w in json.load(sys.stdin)['workflows']]"

# Trigger workflow_dispatch
curl -s -X POST -H "Authorization: token ${GITHUB_TOKEN}" \
  https://api.github.com/repos/${OWNER}/${REPO}/actions/workflows/${WORKFLOW_ID}/dispatches \
  -d '{"ref":"main","inputs":{"environment":"staging"}}'

# Rerun failed jobs only
curl -s -X POST -H "Authorization: token ${GITHUB_TOKEN}" \
  https://api.github.com/repos/${OWNER}/${REPO}/actions/runs/${RUN_ID}/rerun-failed-jobs
```

---

## 10. Gists

```bash
# gh
gh gist create script.py --public --desc "Useful script"
gh gist list

# curl
curl -s -X POST -H "Authorization: token ${GITHUB_TOKEN}" \
  https://api.github.com/gists \
  -d '{"description":"Useful script","public":true,"files":{"script.py":{"content":"print(\"hello\")"}}}'
```

---

## Error Handling

| Symptom | Fix |
|---|---|
| `gh: command not found` | Fall back to curl path; install with `brew install gh` or `winget install gh` |
| 401 Unauthorized | `${GITHUB_TOKEN}` not set or expired - re-inject secret |
| 422 on fork | Repo already forked - clone existing fork |
| 403 on branch protection | Token needs `repo` scope (not fine-grained) |
| Asset upload 404 | Release must exist before uploading assets |
