import { fetchJson } from './index.js'

export const settings = () => fetchJson('/settings')
export const updateSetting = (key, value) =>
  fetchJson('/settings', { method: 'POST', body: JSON.stringify({ key, value }) })
