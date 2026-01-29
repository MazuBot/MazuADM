import { fetchJson } from './index.js'

export async function listWsConnections() {
  return fetchJson('/ws-connections')
}
