import { fetchJson } from './index.js'

export const challenges = () => fetchJson('/challenges')
export const createChallenge = (data) => fetchJson('/challenges', { method: 'POST', body: JSON.stringify(data) })
export const updateChallenge = (id, data) => fetchJson(`/challenges/${id}`, { method: 'PUT', body: JSON.stringify(data) })
export const deleteChallenge = (id) => fetchJson(`/challenges/${id}`, { method: 'DELETE' })
