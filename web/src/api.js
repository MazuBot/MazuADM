const BASE = '/api';

export async function fetchJson(path, opts = {}) {
  const res = await fetch(BASE + path, { ...opts, headers: { 'Content-Type': 'application/json', ...opts.headers } });
  return res.json();
}

export const api = {
  challenges: () => fetchJson('/challenges'),
  createChallenge: (data) => fetchJson('/challenges', { method: 'POST', body: JSON.stringify(data) }),
  updateChallenge: (id, data) => fetchJson(`/challenges/${id}`, { method: 'PUT', body: JSON.stringify(data) }),
  deleteChallenge: (id) => fetchJson(`/challenges/${id}`, { method: 'DELETE' }),
  teams: () => fetchJson('/teams'),
  createTeam: (data) => fetchJson('/teams', { method: 'POST', body: JSON.stringify(data) }),
  updateTeam: (id, data) => fetchJson(`/teams/${id}`, { method: 'PUT', body: JSON.stringify(data) }),
  deleteTeam: (id) => fetchJson(`/teams/${id}`, { method: 'DELETE' }),
  exploits: (challengeId) => fetchJson(challengeId ? `/exploits?challenge_id=${challengeId}` : '/exploits'),
  createExploit: (data) => fetchJson('/exploits', { method: 'POST', body: JSON.stringify(data) }),
  updateExploit: (id, data) => fetchJson(`/exploits/${id}`, { method: 'PUT', body: JSON.stringify(data) }),
  deleteExploit: (id) => fetchJson(`/exploits/${id}`, { method: 'DELETE' }),
  exploitRuns: (challengeId, teamId) => {
    const params = new URLSearchParams();
    if (challengeId) params.set('challenge_id', challengeId);
    if (teamId) params.set('team_id', teamId);
    const qs = params.toString();
    return fetchJson('/exploit-runs' + (qs ? '?' + qs : ''));
  },
  createExploitRun: (data) => fetchJson('/exploit-runs', { method: 'POST', body: JSON.stringify(data) }),
  updateExploitRun: (id, data) => fetchJson(`/exploit-runs/${id}`, { method: 'PUT', body: JSON.stringify(data) }),
  deleteExploitRun: (id) => fetchJson(`/exploit-runs/${id}`, { method: 'DELETE' }),
  reorderExploitRuns: (updates) => fetchJson('/exploit-runs/reorder', { method: 'POST', body: JSON.stringify(updates) }),
  rounds: () => fetchJson('/rounds'),
  createRound: () => fetchJson('/rounds', { method: 'POST' }),
  runRound: (id) => fetchJson(`/rounds/${id}/run`, { method: 'POST' }),
  jobs: (roundId) => fetchJson(`/jobs?round_id=${roundId}`),
  flags: (roundId) => fetchJson(roundId ? `/flags?round_id=${roundId}` : '/flags'),
  settings: () => fetchJson('/settings'),
  updateSetting: (key, value) => fetchJson('/settings', { method: 'POST', body: JSON.stringify({ key, value }) }),
  containers: () => fetchJson('/containers'),
  containerRunners: (id) => fetchJson(`/containers/${id}/runners`),
  deleteContainer: (id) => fetchJson(`/containers/${id}`, { method: 'DELETE' }),
  restartContainer: (id) => fetchJson(`/containers/${id}/restart`, { method: 'POST' }),
  healthCheckContainers: () => fetchJson('/containers/health', { method: 'POST' }),
  ensureContainers: () => fetchJson('/containers/ensure', { method: 'POST' }),
};
