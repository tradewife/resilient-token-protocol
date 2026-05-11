FROM python:3.12-slim
WORKDIR /app

# Install git for committing night results back to repo
RUN apt-get update && apt-get install -y git && rm -rf /var/lib/apt/lists/*

COPY requirements-ci.txt .
RUN pip install --no-cache-dir -r requirements-ci.txt
COPY research/ research/
COPY data/ data/
COPY scripts/commit-night-results.sh .

# Night Shift writes results to /app/data/night_results (relative to WORKDIR).
# The commit script reads from the same path and copies into the git working tree.
ENV NIGHT_RESULTS_DIR=/app/data/night_results

# Run night shift, then commit results back to repo
CMD ["sh", "-c", "python3 -m research.orchestration.night_shift --skip-fetch && sh commit-night-results.sh"]
