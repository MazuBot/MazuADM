import { fetchJson } from './index.js'

export const rounds = () => fetchJson('/rounds')
export const createRound = () => fetchJson('/rounds', { method: 'POST' })
export const runRound = (id) => fetchJson(`/rounds/${id}/run`, { method: 'POST' })
export const rerunRound = (id) => fetchJson(`/rounds/${id}/rerun`, { method: 'POST' })
