# chal3: PHP command injection -> reverse shell

## Build

```sh
docker build -t chal3-srv ./srv
docker build -t chal3-exp ./exp
```

## Run service

```sh
docker run --rm -p 8000:8000 chal3-srv
```

The flag generator starts on first request and updates `/flag` every 5 seconds as `FLAG{TS_<unix_timestamp_rounded_to_5s>}`.

## Run exploit (reverse shell)

### Option A: env vars

```sh
docker run --rm --network host \
  -e TARGET_HOST=127.0.0.1 -e TARGET_PORT=8000 -e TARGET_TEAM_ID=1 \
  chal3-exp
```

### Option B: argv

```sh
docker run --rm --network host \
  chal3-exp /run 127.0.0.1 8000 1
```

## Notes

- UI: `GET /` · Debug: `GET /source` · Vuln: `POST /api/ping` with JSON `{"host":"..."}`
- The injection is blind; the exploit spawns a reverse shell back and runs `cat /flag`.
