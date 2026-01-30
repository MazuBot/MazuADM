import { fetchJson } from './index.js'

const withQuery = (path, params = {}) => {
  const entries = Object.entries(params).filter(([, value]) => value !== null && value !== undefined)
  if (!entries.length) return path
  const search = new URLSearchParams()
  for (const [key, value] of entries) {
    search.set(key, `${value}`)
  }
  const suffix = search.toString()
  return suffix ? `${path}?${suffix}` : path
}

export const containers = () => fetchJson('/containers')
export const deleteContainer = (id) => fetchJson(`/containers/${id}`, { method: 'DELETE' })
export const restartContainer = (id, opts = {}) =>
  fetchJson(`/containers/${id}/restart`, { method: 'POST', body: JSON.stringify(opts) })
export const restartAllContainers = (opts = {}) => {
  const { challenge_id, ...body } = opts
  return fetchJson(withQuery('/containers/restart-all', { challenge_id }), {
    method: 'POST',
    body: JSON.stringify(body)
  })
}
export const removeAllContainers = (opts = {}) =>
  fetchJson(withQuery('/containers/remove-all', { challenge_id: opts.challenge_id }), { method: 'POST' })
