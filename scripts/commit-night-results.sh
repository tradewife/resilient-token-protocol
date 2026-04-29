#!/bin/bash
# commit-night-results.sh
# Commits the latest night results back to the repo so other services
# (rtp-devnet-loop, rtp-promote-strategy) can read them at build time.
#
# Requires env vars:
#   GIT_DEPLOY_KEY — base64-encoded SSH private key for pushing
#   GIT_AUTHOR_NAME / GIT_AUTHOR_EMAIL — git identity (defaults provided)

set -e

# Skip if no deploy key is configured (local dev, CI without push access)
if [ -z "$GIT_DEPLOY_KEY" ]; then
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

# Check if there are changes to commit.
# Night results live in /data/night_results (shared volume) — check there.
if git diff --quiet data/night_results/ 2>/dev/null && [ -z "$(git ls-files --others --exclude-standard data/night_results/)" ]; then
  echo "[COMMIT] No changes in data/night_results/ — nothing to commit"
  exit 0
fi

# Commit and push
DATE=$(date -u +%Y-%m-%d)
# Add from both the symlink working dir (for staged changes) and the volume (for new files)
git add data/night_results/
git add /data/night_results/ 2>/dev/null || true
git commit -m "night-shift: results for ${DATE} [skip ci]" || true

# Retry push up to 3 times (network flakiness on Railway)
for i in 1 2 3; do
  if git push origin HEAD:main; then
    echo "[COMMIT] Pushed night results for ${DATE}"
    exit 0
  fi
  echo "[COMMIT] Push attempt ${i} failed, retrying in 5s..."
  sleep 5
done

echo "[COMMIT] WARNING: Failed to push after 3 attempts"
exit 1
