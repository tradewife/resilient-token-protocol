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

# Night Shift writes results to /data/night_results in the container's writable layer.
# The commit script copies them into the git working tree before pushing.
ENV NIGHT_RESULTS_DIR=/data/night_results

# Run night shift, then commit results back to repo
CMD ["sh", "-c", "python3 -m research.orchestration.night_shift --skip-fetch && sh commit-night-results.sh"]
