import { fetchJson } from './index.js'

export const containers = () => fetchJson('/containers')
export const containerRunners = (id) => fetchJson(`/containers/${id}/runners`)
export const deleteContainer = (id) => fetchJson(`/containers/${id}`, { method: 'DELETE' })
export const restartContainer = (id, opts = {}) =>
  fetchJson(`/containers/${id}/restart`, { method: 'POST', body: JSON.stringify(opts) })
export const restartAllContainers = (opts = {}) =>
  fetchJson('/containers/restart-all', { method: 'POST', body: JSON.stringify(opts) })
export const removeAllContainers = () => fetchJson('/containers/remove-all', { method: 'POST' })
