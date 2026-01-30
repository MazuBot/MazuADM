let ws = null;
let reconnectTimer = null;
let onAuthError = null;
let onConnect = null;

const feVersion = import.meta.env.DEV ? 'dev' : __BUILD_GIT_HASH__;

export function getUser() {
  return localStorage.getItem('mazuadm_user') || '';
}

export function setUser(user) {
  localStorage.setItem('mazuadm_user', user);
}

export function setOnAuthError(callback) {
  onAuthError = callback;
}

export function setOnConnect(callback) {
  onConnect = callback;
}

export function connect(onMessage) {
  if (ws) return;
  
  const user = getUser();
  if (!user) return;
  
  const proto = location.protocol === 'https:' ? 'wss:' : 'ws:';
  ws = new WebSocket(`${proto}//${location.host}/ws?client=web-ui-${feVersion}&user=${encodeURIComponent(user)}`);
  
  ws.onmessage = (e) => {
    try {
      const msg = JSON.parse(e.data);
      if (msg.type === 'error') {
        localStorage.removeItem('mazuadm_user');
        onAuthError?.();
        return;
      }
      onMessage(msg);
    } catch {}
  };

  ws.onopen = () => {
    onConnect?.();
  };
  
  ws.onclose = () => {
    ws = null;
    if (getUser()) {
      reconnectTimer = setTimeout(() => connect(onMessage), 2000);
    }
  };
  
  ws.onerror = () => ws?.close();
}

export function disconnect() {
  clearTimeout(reconnectTimer);
  ws?.close();
  ws = null;
}
