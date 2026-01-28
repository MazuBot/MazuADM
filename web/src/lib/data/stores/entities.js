import { writable } from 'svelte/store'
import * as api from '$lib/data/api'

export const challenges = writable([])
export const teams = writable([])
export const exploits = writable([])
export const exploitRuns = writable([])

export async function loadAllEntities() {
  const [c, t, e, r] = await Promise.all([
    api.challenges(),
    api.teams(),
    api.exploits(),
    api.exploitRuns()
  ])
  challenges.set(c)
  teams.set(t)
  exploits.set(e)
  exploitRuns.set(r)
}
