FROM debian:bookworm AS build_chall

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y --no-install-recommends \
    gcc \
    libc6-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build
COPY chall.c /build/chall.c
RUN gcc -O0 -fno-stack-protector -no-pie -fno-omit-frame-pointer -o chall chall.c

FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive
ENV PYTHONUNBUFFERED=1

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    python3 \
    python3-pip \
    && rm -rf /var/lib/apt/lists/*

RUN pip3 install --no-cache-dir --break-system-packages pwntools httpx

COPY --from=build_chall /build/chall /opt/chall
RUN rm -rf /run /var/run && mkdir -p /var/run
COPY exploit.py /run
COPY lib.py /lib.py
RUN chmod 755 /run

CMD ["/run"]
