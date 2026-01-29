const BASE = '/api'

export class ApiError extends Error {
  constructor(status, message, payload) {
    super(message || `Request failed (${status})`)
    this.name = 'ApiError'
    this.status = status
    this.payload = payload
  }
}

export async function fetchJson(path, opts = {}) {
  const res = await fetch(BASE + path, {
    ...opts,
    headers: { 'Content-Type': 'application/json', ...opts.headers }
  })
  const contentType = res.headers.get('content-type') || ''
  let payload = null
  if (contentType.includes('application/json')) {
    try {
      payload = await res.json()
    } catch {
      payload = null
    }
  } else {
    const text = await res.text()
    payload = text || null
  }
  if (!res.ok) {
    const message =
      (payload && typeof payload === 'object' && (payload.error || payload.message)) ||
      (typeof payload === 'string' && payload) ||
      res.statusText ||
      `Request failed (${res.status})`
    throw new ApiError(res.status, message, payload)
  }
  return payload
}

export * from './challenges.js'
export * from './teams.js'
export * from './exploits.js'
export * from './exploitRuns.js'
export * from './rounds.js'
export * from './jobs.js'
export * from './flags.js'
export * from './settings.js'
export * from './containers.js'
export * from './relations.js'
export * from './version.js'
export * from './wsConnections.js'
