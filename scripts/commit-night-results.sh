#!/bin/bash
# commit-night-results.sh
# Commits the latest night results back to the repo so other services
# (rtp-devnet-loop, rtp-promote-strategy) can read them at build time.
#
# Requires env vars:
#   GIT_DEPLOY_KEY — base64-encoded SSH private key for pushing
#   GIT_AUTHOR_NAME / GIT_AUTHOR_EMAIL — git identity (defaults provided)

set -euo pipefail

echo "[COMMIT] script starting, GIT_DEPLOY_KEY=${GIT_DEPLOY_KEY:+SET}/${GIT_DEPLOY_KEY:-UNSET}"

# Skip if no deploy key is configured (local dev, CI without push access)
if [ -z "${GIT_DEPLOY_KEY:-}" ]; then
  echo "[COMMIT] GIT_DEPLOY_KEY not set — skipping git commit (expected in local dev)"
  exit 0
fi

# Configure git identity
export GIT_AUTHOR_NAME="${GIT_AUTHOR_NAME:-rtp-night-shift}"
export GIT_AUTHOR_EMAIL="${GIT_AUTHOR_EMAIL:-night-shift@resilientprotocol.xyz}"
export GIT_COMMITTER_NAME="$GIT_AUTHOR_NAME"
export GIT_COMMITTER_EMAIL="$GIT_AUTHOR_EMAIL"

# Set up SSH deploy key
mkdir -p ~/.ssh
echo "$GIT_DEPLOY_KEY" | base64 -d > ~/.ssh/deploy_key
chmod 600 ~/.ssh/deploy_key
cat > ~/.ssh/config <<EOF
Host github.com
  HostName github.com
  User git
  IdentityFile ~/.ssh/deploy_key
  StrictHostKeyChecking no
EOF

# Set remote to SSH (in case it was https)
cd /app
git remote set-url origin "git@github.com:tradewife/resilient-token-protocol.git" 2>/dev/null || true

# Stage night results from the NIGHT_RESULTS_DIR (written by the Python pipeline).
# The Dockerfile creates /data/night_results; this is where night-shift writes.
NIGHT_DIR="${NIGHT_RESULTS_DIR:-/data/night_results}"

if [ ! -d "$NIGHT_DIR" ]; then
  echo "[COMMIT] Night results directory not found: $NIGHT_DIR — nothing to commit"
  exit 0
fi

# Copy results into the git working tree so git can track them.
# The repo's data/night_results/ is the canonical location.
mkdir -p /app/data/night_results
cp -rf "$NIGHT_DIR"/* /app/data/night_results/ 2>/dev/null || true

# Check if there are changes to commit
if git diff --quiet data/night_results/ 2>/dev/null && [ -z "$(git ls-files --others --exclude-standard data/night_results/)" ]; then
  echo "[COMMIT] No changes in data/night_results/ — nothing to commit"
  exit 0
fi

# Commit and push
DATE=$(date -u +%Y-%m-%d)
git add data/night_results/ || true
git commit -m "night-shift: results for ${DATE} [skip ci]" || {
  echo "[COMMIT] Nothing to commit (possibly empty diff after add)"
  exit 0
}

# Retry push up to 3 times with exponential backoff
for i in 1 2 3; do
  if git push origin HEAD:main; then
    echo "[COMMIT] Pushed night results for ${DATE}"
    exit 0
  fi
  delay=$((5 * (2 ** (i - 1))))
  echo "[COMMIT] Push attempt ${i} failed, retrying in ${delay}s..."
  sleep "$delay"
done

echo "[COMMIT] WARNING: Failed to push after 3 attempts"
exit 1
