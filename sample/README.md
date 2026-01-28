# Sample challenges & exploits

This folder contains the sample challenge services and exploit images used for local testing.

## Quick exploit testing (all images)

Use the helper script to run all sample exploit images against the already-running sample services.
It uses `--rm --network host -it` so the pwntools images avoid terminfo warnings.

```sh
./scripts/test_exps.sh
```

## Start sample services

Build and run all sample challenge services (skips containers that already exist).

```sh
./scripts/start_chals.sh
```

### Options

```sh
FLAG_SECRET=sample-secret CHAL1_PORT=18000 CHAL2_PORT=18001 CHAL3_PORT=18002 \
  ./scripts/start_chals.sh
```

### Options

```sh
TEAM_ID=1 HOST=127.0.0.1 \
  CHAL1_PORT=18000 CHAL2_PORT=18001 CHAL3_PORT=18002 \
  ./scripts/test_exps.sh
```

## Manual testing examples

### chal1 (web)

```sh
docker run --rm -it --network host \
  -e TARGET_HOST=127.0.0.1 -e TARGET_PORT=18000 -e TARGET_TEAM_ID=1 \
  sample-chal1-exp

docker run --rm -it --network host \
  -e TARGET_HOST=127.0.0.1 -e TARGET_PORT=18000 -e TARGET_TEAM_ID=1 \
  sample-chal1-exp2
```

### chal2 (bin)

```sh
docker run --rm -it --network host sample-chal2-exp /run 127.0.0.1 18001 1
docker run --rm -it --network host sample-chal2-exp2 /run 127.0.0.1 18001 1
```

### chal3 (cmdi)

```sh
docker run --rm -it --network host sample-chal3-exp /run 127.0.0.1 18002 1
docker run --rm -it --network host sample-chal3-exp2 /run 127.0.0.1 18002 1
```
