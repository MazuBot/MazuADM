FROM php:8.3-cli-bookworm

RUN apt-get update && apt-get install -y --no-install-recommends \
    bash \
    ca-certificates \
    iproute2 \
    iputils-ping \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /var/www/html

COPY public/ /var/www/html/

RUN useradd -m -u 1000 ctf && \
    chown -R ctf:ctf /var/www/html

RUN ln -sf /tmp/flag /flag

EXPOSE 8000
USER ctf

CMD ["php", "-S", "0.0.0.0:8000", "-t", "/var/www/html", "/var/www/html/index.php"]
