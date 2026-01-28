import hashlib
import os

import httpx
from fastapi import FastAPI, HTTPException, Request
from fastapi.responses import PlainTextResponse

app = FastAPI(title="chal1 sample service")


def _make_flag(team_id: str) -> str:
    secret = os.environ.get("FLAG_SECRET", "sample-secret")
    digest = hashlib.sha256(f"{secret}:{team_id}".encode()).hexdigest()[:16]
    return f"FLAG{{TEAM_{team_id}_{digest}}}"


@app.get("/", response_class=PlainTextResponse)
def index() -> str:
    return (
        "Sample service is up.\n"
        "Try: GET /health\n"
        "Vuln: GET /api/fetch?url=http://...\n"
    )


@app.get("/health")
def health() -> dict:
    return {"status": "ok"}


@app.get("/internal/flag", response_class=PlainTextResponse)
def internal_flag(request: Request, team_id: str) -> str:
    client_host = request.client.host if request.client else ""
    if client_host not in {"127.0.0.1", "::1"}:
        raise HTTPException(status_code=403, detail="localhost only")
    return _make_flag(team_id)


@app.get("/api/fetch", response_class=PlainTextResponse)
async def fetch(url: str) -> str:
    if not (url.startswith("http://") or url.startswith("https://")):
        raise HTTPException(status_code=400, detail="http(s) urls only")

    timeout = httpx.Timeout(2.0, connect=1.0)
    async with httpx.AsyncClient(timeout=timeout, follow_redirects=True) as client:
        resp = await client.get(url)
        resp.raise_for_status()

    # Intentionally unsafe: reflects fetched content (SSRF-style).
    return resp.text[:4096]
