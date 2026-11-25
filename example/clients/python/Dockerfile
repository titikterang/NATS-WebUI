FROM python:3.14-slim
ENV PYTHONDONTWRITEBYTECODE=1 PYTHONUNBUFFERED=1
WORKDIR /app
COPY requirements.txt ./
RUN pip install --no-cache-dir -r requirements.txt
COPY client.py ./
USER 65532:65532
# One-shot client: it publishes or receives COUNT messages and exits, so there's nothing to probe.
HEALTHCHECK NONE
ENTRYPOINT ["python", "client.py"]
