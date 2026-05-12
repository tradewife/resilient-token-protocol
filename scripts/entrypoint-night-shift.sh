#!/bin/bash
# entrypoint-night-shift.sh
# Runs the night shift pipeline, then commits results back to the repo.
#
# The Docker container does not have a .git directory (COPY doesn't bring it).
# This script shallow-clones the repo at runtime so git operations work,
# runs night shift, then copies results into the clone and pushes.
#
# Requires env vars (set in Railway):
#   GIT_DEPLOY_KEY — base64-encoded SSH private key
#   GIT_AUTHOR_NAME / GIT_AUTHOR_EMAIL — git identity (optional)

set -euo pipefail

REPO="git@github.com:tradewife/resilient-token-protocol.git"
CLONE_DIR="/tmp/rtp-repo"
NIGHT_DIR="${NIGHT_RESULTS_DIR:-/app/data/night_results}"
DATE=$(date -u +%Y-%m-%d)

# ── Phase 1: Set up git clone if deploy key is available ──

if [ -n "${GIT_DEPLOY_KEY:-}" ]; then
  echo "[ENTRYPOINT] Setting up SSH deploy key and cloning repo..."
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

  rm -rf "$CLONE_DIR"
  git clone --depth 1 "$REPO" "$CLONE_DIR"
  echo "[ENTRYPOINT] Repo cloned to $CLONE_DIR"
else
  echo "[ENTRYPOINT] GIT_DEPLOY_KEY not set — results will not be pushed to git"
fi

# ── Phase 2: Run Night Shift ──

echo "[ENTRYPOINT] Starting night shift..."
cd /app
python3 -m research.orchestration.night_shift --skip-fetch
NIGHT_EXIT=$?

if [ $NIGHT_EXIT -ne 0 ]; then
  echo "[ENTRYPOINT] Night shift exited with code $NIGHT_EXIT"
fi

# ── Phase 3: Commit results back to repo ──

if [ -z "${GIT_DEPLOY_KEY:-}" ]; then
  echo "[ENTRYPOINT] No deploy key — skipping git commit"
  exit $NIGHT_EXIT
fi

if [ ! -d "$NIGHT_DIR" ]; then
  echo "[ENTRYPOINT] Night results directory not found: $NIGHT_DIR — nothing to commit"
  exit $NIGHT_EXIT
fi

# Copy results into the cloned repo
TARGET_DIR="$CLONE_DIR/data/night_results"
mkdir -p "$TARGET_DIR"
cp -rf "$NIGHT_DIR"/* "$TARGET_DIR"/ 2>/dev/null || true

cd "$CLONE_DIR"

# Check for changes
if git diff --quiet data/night_results/ 2>/dev/null && \
   [ -z "$(git ls-files --others --exclude-standard data/night_results/)" ]; then
  echo "[ENTRYPOINT] No changes in data/night_results/ — nothing to commit"
  exit $NIGHT_EXIT
fi

# Commit
git add data/night_results/ || true
export GIT_AUTHOR_NAME="${GIT_AUTHOR_NAME:-rtp-night-shift}"
export GIT_AUTHOR_EMAIL="${GIT_AUTHOR_EMAIL:-night-shift@resilientprotocol.xyz}"
export GIT_COMMITTER_NAME="$GIT_AUTHOR_NAME"
export GIT_COMMITTER_EMAIL="$GIT_AUTHOR_EMAIL"

git commit -m "night-shift: results for ${DATE} [skip ci]" || {
  echo "[ENTRYPOINT] Nothing to commit (possibly empty diff after add)"
  exit $NIGHT_EXIT
}

# Push with retry
for i in 1 2 3; do
  if git push origin HEAD:main; then
    echo "[ENTRYPOINT] Pushed night results for ${DATE}"
    exit $NIGHT_EXIT
  fi
  delay=$((5 * (2 ** (i - 1))))
  echo "[ENTRYPOINT] Push attempt ${i} failed, retrying in ${delay}s..."
  sleep "$delay"
done

echo "[ENTRYPOINT] WARNING: Failed to push after 3 attempts"
exit $NIGHT_EXIT
