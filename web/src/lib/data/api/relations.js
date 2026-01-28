import { fetchJson } from './index.js'

export const relations = (challengeId) => fetchJson(`/relations/${challengeId}`)
export const getRelation = (challengeId, teamId) => fetchJson(`/relations/${challengeId}/${teamId}`)
export const updateRelation = (challengeId, teamId, data) =>
  fetchJson(`/relations/${challengeId}/${teamId}`, { method: 'PUT', body: JSON.stringify(data) })
