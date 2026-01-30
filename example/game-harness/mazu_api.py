from typing import List, Optional
from urllib.parse import urlencode

import requests
import websocket

from config import REQUEST_TIMEOUT_SECS


class MazuAPI:
    def __init__(
        self,
        endpoint: str,
        ws_endpoint: str,
        ws_user: str,
        ws_client: str,
        timeout_secs: int = REQUEST_TIMEOUT_SECS,
    ):
        self._endpoint = endpoint
        self._ws_endpoint = ws_endpoint
        self._ws_user = ws_user
        self._ws_client = ws_client
        self._timeout = timeout_secs

    # POST MazuADM/api/rounds
    def push_new_round(self) -> int:
        url = f"{self._endpoint.rstrip('/')}/api/rounds"
        response = requests.post(url, timeout=self._timeout)
        response.raise_for_status()
        return int(response.json())

    # POST MazuADM/api/rounds/{id}/run
    def push_round_start(self, round_id):
        url = f"{self._endpoint.rstrip('/')}/api/rounds/{round_id}/run"
        response = requests.post(url, timeout=self._timeout)
        response.raise_for_status()

    # GET MazuADM/api/flags
    def fetch_all_flags(self) -> List[dict]:
        url = f"{self._endpoint.rstrip('/')}/api/flags?status=captured"
        response = requests.get(url, timeout=self._timeout)
        response.raise_for_status()
        flags = response.json()
        return flags if isinstance(flags, list) else []

    def update_flag_status(self, flag_id: int, status: str, force: bool = False):
        url = f"{self._endpoint.rstrip('/')}/api/flags"
        params = {'force': 'true'} if force else None
        response = requests.patch(
            url,
            params=params,
            json={'id': flag_id, 'status': status},
            timeout=self._timeout,
        )
        response.raise_for_status()
        return response.json()

    def ws_connect(self, events: Optional[List[str]] = None):
        if events is None:
            events = ['flag']
        params = {'user': self._ws_user}
        if self._ws_client:
            params['client'] = self._ws_client
        if events is not None:
            params['events'] = ','.join(events)

        query = urlencode(params)
        ws_url = f"{self._ws_endpoint}?{query}" if query else self._ws_endpoint
        return websocket.create_connection(ws_url, timeout=self._timeout)
