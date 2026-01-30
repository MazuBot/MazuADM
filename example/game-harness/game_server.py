import requests

from config import GAME_STATUS_MAP, REQUEST_TIMEOUT_SECS


class GameServer:
    def __init__(self, endpoint: str, timeout_secs: int = REQUEST_TIMEOUT_SECS):
        self._endpoint = endpoint
        self._timeout = timeout_secs

    def _require_endpoint(self):
        if not self._endpoint:
            raise ValueError('GAME_ENDPOINT is not configured')

    def send_flag(self, flag_content):
        self._require_endpoint()
        url = f"{self._endpoint.rstrip('/')}/flags"
        response = requests.post(
            url,
            json={'flag': flag_content},
            timeout=self._timeout,
        )
        response.raise_for_status()
        try:
            return response.json()
        except ValueError:
            return response.text

    def fetch_round(self):
        self._require_endpoint()
        url = f"{self._endpoint.rstrip('/')}/round"
        response = requests.get(url, timeout=self._timeout)
        response.raise_for_status()
        data = response.json()
        if isinstance(data, dict) and 'round' in data:
            return int(data['round'])
        return int(data)

    def extract_status(self, result):
        status = None
        if isinstance(result, dict):
            status = result.get('status') or result.get('result')
        elif isinstance(result, str):
            status = result
        if not status:
            return None
        return GAME_STATUS_MAP.get(str(status).strip().lower())
