let ws = null;
let reconnectTimer = null;
let handlers = {};

const feVersion = import.meta.env.DEV ? 'dev' : __BUILD_GIT_HASH__;

export function connect(onMessage) {
  if (ws) return;
  
  const proto = location.protocol === 'https:' ? 'wss:' : 'ws:';
  ws = new WebSocket(`${proto}//${location.host}/ws?client=web-ui-${feVersion}`);
  
  ws.onmessage = (e) => {
    try {
      const msg = JSON.parse(e.data);
      onMessage(msg);
    } catch {}
  };
  
  ws.onclose = () => {
    ws = null;
    reconnectTimer = setTimeout(() => connect(onMessage), 2000);
  };
  
  ws.onerror = () => ws?.close();
}

export function disconnect() {
  clearTimeout(reconnectTimer);
  ws?.close();
  ws = null;
}
