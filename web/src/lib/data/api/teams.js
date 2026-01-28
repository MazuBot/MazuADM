import { fetchJson } from './index.js'

export const teams = () => fetchJson('/teams')
export const createTeam = (data) => fetchJson('/teams', { method: 'POST', body: JSON.stringify(data) })
export const updateTeam = (id, data) => fetchJson(`/teams/${id}`, { method: 'PUT', body: JSON.stringify(data) })
export const deleteTeam = (id) => fetchJson(`/teams/${id}`, { method: 'DELETE' })
