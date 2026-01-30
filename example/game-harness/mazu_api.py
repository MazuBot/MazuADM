import json
import logging
import time
from typing import Callable, List, Optional
from urllib.parse import urlencode

import requests
import websocket
from websocket import WebSocketTimeoutException

from config import REQUEST_TIMEOUT_SECS


logger = logging.getLogger(__name__)


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

    # GET MazuADM/api/rounds/current
    def get_current_round_id(self) -> Optional[int]:
        url = f"{self._endpoint.rstrip('/')}/api/rounds/current"
        response = requests.get(url, timeout=self._timeout)
        response.raise_for_status()
        round_obj = response.json()
        if not round_obj:
            return None
        if isinstance(round_obj, dict) and 'id' in round_obj:
            return int(round_obj['id'])
        return None

    # GET MazuADM/api/flags
    def fetch_all_flags(self) -> List[dict]:
        url = f"{self._endpoint.rstrip('/')}/api/flags?status=captured,recheck"
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

    def listen_flag_events(
        self,
        stop_event,
        on_flag: Callable[[dict], None],
        reconnect_delay_secs: float,
        recv_timeout_secs: float,
    ):
        while not stop_event.is_set():
            ws = None
            try:
                logger.info('connecting to MazuADM websocket')
                ws = self.ws_connect(events=['flag'])
                ws.settimeout(recv_timeout_secs)
                logger.info('websocket connected')
                while not stop_event.is_set():
                    try:
                        raw = ws.recv()
                    except WebSocketTimeoutException:
                        continue
                    if not raw:
                        logger.warning('websocket closed by server')
                        break
                    try:
                        message = json.loads(raw)
                    except json.JSONDecodeError:
                        logger.debug('invalid websocket payload: %s', raw)
                        continue
                    if message.get('type') != 'flag_created':
                        continue
                    payload = message.get('data', {})
                    on_flag(payload)
            except Exception:
                logger.exception(
                    'websocket error; reconnecting in %ss', reconnect_delay_secs
                )
                time.sleep(reconnect_delay_secs)
            finally:
                if ws is not None:
                    try:
                        ws.close()
                    except Exception:
                        logger.debug('failed to close websocket cleanly')
