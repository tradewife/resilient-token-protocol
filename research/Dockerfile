FROM python:3.12-slim
WORKDIR /app
COPY requirements-ci.txt .
RUN pip install --no-cache-dir -r requirements-ci.txt
COPY research/ research/
COPY data/ data/
COPY configs/ configs/
CMD ["python3", "-m", "research.orchestration.night_shift", "--skip-fetch"]
