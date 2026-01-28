FROM python:3.11-slim

ENV PYTHONUNBUFFERED=1

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN pip install --no-cache-dir httpx

RUN rm -rf /run /var/run && mkdir -p /var/run
COPY exploit.py /run
COPY lib.py /lib.py
RUN chmod 755 /run

CMD ["/run"]
