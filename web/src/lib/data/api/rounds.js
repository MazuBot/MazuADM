import { fetchJson } from './index.js'

export const rounds = () => fetchJson('/rounds')
export const createRound = (target) => {
  const body = target != null ? { target } : {}
  return fetchJson('/rounds', { method: 'POST', body: JSON.stringify(body) })
}
export const runRound = (id) => fetchJson(`/rounds/${id}/run`, { method: 'POST' })
export const rerunRound = (id) => fetchJson(`/rounds/${id}/rerun`, { method: 'POST' })
export const rerunUnflaggedRound = (id) => fetchJson(`/rounds/${id}/rerun-unflagged`, { method: 'POST' })
