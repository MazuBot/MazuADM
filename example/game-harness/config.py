ADM_ENDPOINT = 'http://localhost:3000'
WS_ENDPOINT = 'ws://localhost:3000/ws'
GAME_ENDPOINT = ''  # Mocked game server

WS_USER = 'connector'
WS_CLIENT = 'game-connector'

REQUEST_TIMEOUT_SECS = 5
ROUND_POLL_INTERVAL_SECS = 5
WS_RECONNECT_DELAY_SECS = 2
WS_RECV_TIMEOUT_SECS = 1

GAME_STATUS_MAP = {
    'success': 'succeed',
    'succeed': 'succeed',
    'failed': 'failed',
    'duplicated': 'duplicated',
}
