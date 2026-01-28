FROM python:3.11-slim

WORKDIR /app

RUN pip install --no-cache-dir httpx

COPY exploit.py /app/exploit.py
RUN chmod 755 /app/exploit.py

ENTRYPOINT ["/app/exploit.py"]
