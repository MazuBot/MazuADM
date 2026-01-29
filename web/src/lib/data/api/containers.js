import { fetchJson } from './index.js'

export const containers = () => fetchJson('/containers')
export const containerRunners = (id) => fetchJson(`/containers/${id}/runners`)
export const deleteContainer = (id) => fetchJson(`/containers/${id}`, { method: 'DELETE' })
export const restartContainer = (id) => fetchJson(`/containers/${id}/restart`, { method: 'POST' })
export const restartAllContainers = () => fetchJson('/containers/restart-all', { method: 'POST' })
export const removeAllContainers = () => fetchJson('/containers/remove-all', { method: 'POST' })
