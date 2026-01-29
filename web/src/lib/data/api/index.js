const BASE = '/api'

export async function fetchJson(path, opts = {}) {
  const res = await fetch(BASE + path, {
    ...opts,
    headers: { 'Content-Type': 'application/json', ...opts.headers }
  })
  return res.json()
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
