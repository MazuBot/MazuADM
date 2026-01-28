FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive
ENV PYTHONUNBUFFERED=1

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    python3 \
    python3-pip \
    && rm -rf /var/lib/apt/lists/*

RUN pip3 install --no-cache-dir --break-system-packages pwntools httpx

RUN rm -rf /run /var/run && mkdir -p /var/run
COPY exploit.py /run
COPY lib.py /lib.py
RUN chmod 755 /run

CMD ["/run"]
