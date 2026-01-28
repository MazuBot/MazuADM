const ROUTES = new Set([
  'board',
  'challenges',
  'teams',
  'rounds',
  'flags',
  'containers',
  'settings'
]);

export function parseHash(hash = location.hash) {
  const raw = (hash || '').replace(/^#/, '');
  const path = raw.startsWith('/') ? raw.slice(1) : raw;
  const parts = path.split('/').filter(Boolean);

  const page = ROUTES.has(parts[0]) ? parts[0] : 'board';
  const id = parts[1] && /^\d+$/.test(parts[1]) ? Number(parts[1]) : null;

  return { page, id };
}

export function buildHash(page, id = null) {
  const safePage = ROUTES.has(page) ? page : 'board';
  const safeId = Number.isInteger(id) && id > 0 ? id : null;
  return `#/${safePage}${safeId ? `/${safeId}` : ''}`;
}

