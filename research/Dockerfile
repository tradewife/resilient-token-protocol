FROM python:3.12-slim
WORKDIR /app

# Install git + openssh for cloning and pushing results back to repo
RUN apt-get update && apt-get install -y git openssh-client && rm -rf /var/lib/apt/lists/*

COPY requirements-ci.txt .
RUN pip install --no-cache-dir -r requirements-ci.txt
COPY research/ research/
COPY data/ data/
COPY scripts/entrypoint-night-shift.sh .
RUN chmod +x entrypoint-night-shift.sh

# Night Shift writes results to /app/data/night_results (relative to WORKDIR).
ENV NIGHT_RESULTS_DIR=/app/data/night_results

CMD ["./entrypoint-night-shift.sh"]
