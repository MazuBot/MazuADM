import { fetchJson } from './index.js'

export const version = () => fetchJson('/version')
