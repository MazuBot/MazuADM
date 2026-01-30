import logging
import queue
import threading
import time
from typing import Optional

from config import (
    ADM_ENDPOINT,
    GAME_ENDPOINT,
    REQUEST_TIMEOUT_SECS,
    ROUND_POLL_INTERVAL_SECS,
    WS_CLIENT,
    WS_ENDPOINT,
    WS_RECONNECT_DELAY_SECS,
    WS_RECV_TIMEOUT_SECS,
    WS_USER,
)
from game_server import GameServer
from mazu_api import MazuAPI
from models import FlagItem


logger = logging.getLogger(__name__)


def flag_item_from_payload(flag: dict) -> Optional[FlagItem]:
    if not isinstance(flag, dict):
        return None
    flag_value = flag.get('flag_value')
    if not flag_value:
        return None
    flag_id = flag.get('id')
    return FlagItem(flag_id=flag_id, flag_value=flag_value, raw=flag)


def seed_flag_queue(flag_queue: queue.Queue, api: MazuAPI) -> int:
    flags = api.fetch_all_flags()
    queued = 0
    for flag in flags:
        item = flag_item_from_payload(flag)
        if item:
            flag_queue.put(item)
            queued += 1
    logger.info('seeded %s flags into queue', queued)
    return queued


def enqueue_flag_from_ws(flag_queue: queue.Queue, payload: dict):
    item = flag_item_from_payload(payload)
    if item:
        flag_queue.put(item)
        logger.info('flag enqueued from websocket: %s', item.flag_id)


def flag_dispatch_loop(
    stop_event: threading.Event,
    flag_queue: queue.Queue,
    game_server: GameServer,
    api: MazuAPI,
):
    while not stop_event.is_set():
        try:
            item = flag_queue.get(timeout=1)
        except queue.Empty:
            continue
        try:
            result = game_server.send_flag(item.flag_value)
            status = game_server.extract_status(result)
            if status and item.flag_id is not None:
                api.update_flag_status(item.flag_id, status)
                logger.info('flag %s updated to %s', item.flag_id, status)
            else:
                logger.warning('flag %s sent but no status returned', item.flag_id)
        except Exception:
            logger.exception('failed to dispatch flag %s', item.flag_id)
        finally:
            flag_queue.task_done()


def round_monitor_loop(stop_event: threading.Event, game_server: GameServer, api: MazuAPI):
    game_round = None
    last_run_round = None
    while not stop_event.is_set():
        try:
            latest_game_round = game_server.fetch_round()
        except Exception:
            logger.exception('failed to fetch game round')
            time.sleep(ROUND_POLL_INTERVAL_SECS)
            continue
        if game_round != latest_game_round:
            game_round = latest_game_round
            logger.info('game round changed to %s', game_round)
        try:
            adm_round = api.get_current_round_id()
        except Exception:
            logger.exception('failed to fetch MazuADM current round')
            time.sleep(ROUND_POLL_INTERVAL_SECS)
            continue

        if adm_round is None:
            logger.info('no MazuADM round; creating up to %s', game_round)
            adm_round = 0

        if adm_round < game_round:
            logger.info('advancing MazuADM rounds from %s to %s', adm_round, game_round)
            while adm_round < game_round and not stop_event.is_set():
                adm_round = api.push_new_round()
                logger.info('created round %s', adm_round)

        target_round = game_round
        if last_run_round != target_round:
            logger.info('starting MazuADM round %s', target_round)
            api.push_round_start(target_round)
            last_run_round = target_round
        time.sleep(ROUND_POLL_INTERVAL_SECS)


def run_connector():
    stop_event = threading.Event()
    flag_queue: queue.Queue = queue.Queue()
    game_server = GameServer(GAME_ENDPOINT, timeout_secs=REQUEST_TIMEOUT_SECS)
    api = MazuAPI(
        ADM_ENDPOINT,
        WS_ENDPOINT,
        WS_USER,
        WS_CLIENT,
        timeout_secs=REQUEST_TIMEOUT_SECS,
    )

    seed_flag_queue(flag_queue, api)

    threads = [
        threading.Thread(
            target=api.listen_flag_events,
            args=(
                stop_event,
                lambda payload: enqueue_flag_from_ws(flag_queue, payload),
                WS_RECONNECT_DELAY_SECS,
                WS_RECV_TIMEOUT_SECS,
            ),
            name='ws-flag-listener',
            daemon=True,
        ),
        threading.Thread(
            target=flag_dispatch_loop,
            args=(stop_event, flag_queue, game_server, api),
            name='flag-dispatcher',
            daemon=True,
        ),
        threading.Thread(
            target=round_monitor_loop,
            args=(stop_event, game_server, api),
            name='round-monitor',
            daemon=True,
        ),
    ]

    for thread in threads:
        thread.start()

    try:
        while True:
            time.sleep(1)
    except KeyboardInterrupt:
        logger.info('shutdown requested')
        stop_event.set()
        for thread in threads:
            thread.join(timeout=2)
