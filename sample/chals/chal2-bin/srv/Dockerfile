FROM debian:bookworm AS build

RUN apt-get update && apt-get install -y --no-install-recommends \
    gcc \
    libc6-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build
COPY src/chall.c /build/chall.c
RUN gcc -O0 -fno-stack-protector -no-pie -fno-omit-frame-pointer -o chall chall.c

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    socat \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -m -u 1000 ctf
WORKDIR /home/ctf

COPY --from=build /build/chall /home/ctf/chall
COPY flag /flag

RUN chown root:root /flag && chmod 444 /flag && \
    chown root:root /home/ctf/chall && chmod 555 /home/ctf/chall

EXPOSE 5000
USER ctf
CMD ["socat","TCP-LISTEN:5000,reuseaddr,fork","EXEC:/home/ctf/chall,stderr"]

