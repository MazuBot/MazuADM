import { fetchJson } from './index.js'

export const flags = (roundId) => fetchJson(roundId ? `/flags?round_id=${roundId}` : '/flags')
export const submitFlag = (payload) =>
  fetchJson('/flags', { method: 'POST', body: JSON.stringify(payload) })
