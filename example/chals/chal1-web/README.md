# chal1: sample web service + exploit (no Docker network)

## Build

```sh
docker build -t chal1-srv ./srv
docker build -t chal1-exp ./exp
```

## Run service (bind port directly)

```sh
docker run --rm -p 8000:8000 -e FLAG_SECRET=testsecret chal1-srv
```

## Run exploit (no custom Docker network)

### Option A (Linux): host networking

```sh
docker run --rm --network host \
  -e TARGET_HOST=127.0.0.1 -e TARGET_PORT=8000 -e TARGET_TEAM_ID=1 \
  chal1-exp
```

### Option B (portable): reach host via `host.docker.internal`

```sh
docker run --rm --add-host=host.docker.internal:host-gateway \
  -e TARGET_HOST=host.docker.internal -e TARGET_PORT=8000 -e TARGET_TEAM_ID=1 \
  chal1-exp
```
