FROM python:3.12-slim
WORKDIR /app

# Install git for committing night results back to repo
RUN apt-get update && apt-get install -y git && rm -rf /var/lib/apt/lists/*

COPY requirements-ci.txt .
RUN pip install --no-cache-dir -r requirements-ci.txt
COPY research/ research/
COPY data/ data/
COPY configs/ configs/

# Copy the commit script
COPY scripts/commit-night-results.sh .

# Symlink night_results into git working dir so the commit script can git-add them.
# Volume mounts /data/night_results at runtime; night-shift writes there via NIGHT_RESULTS_DIR.
# The symlink lets the commit script (running from /app) see the volume data.
RUN mkdir -p /data && ln -s /data/night_results /app/data/night_results 2>/dev/null || true

# Night Shift writes results to the shared volume (not baked into image).
# Promote and daemon services mount the same volume and read without rebuilding.
ENV NIGHT_RESULTS_DIR=/data/night_results

# Declare shared volume — Railway mounts this across all services.
# Night Shift writes, promote + daemon read. Persistent across cron runs.
VOLUME ["/data"]

# Run night shift, then commit results back to repo (symlink ensures git sees them)
CMD ["sh", "-c", "python3 -m research.orchestration.night_shift --skip-fetch && sh commit-night-results.sh"]
