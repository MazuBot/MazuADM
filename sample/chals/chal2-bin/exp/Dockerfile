FROM debian:bookworm AS build_chall

RUN apt-get update && apt-get install -y --no-install-recommends \
    gcc \
    libc6-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build
COPY chall.c /build/chall.c
RUN gcc -O0 -fno-stack-protector -no-pie -fno-omit-frame-pointer -o chall chall.c

FROM python:3.11-slim

ENV PYTHONUNBUFFERED=1

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN pip install --no-cache-dir pwntools

COPY --from=build_chall /build/chall /opt/chall
RUN rm -rf /run /var/run && mkdir -p /var/run
COPY exploit.py /run
COPY lib.py /lib.py
RUN chmod 755 /run

CMD ["/run"]
