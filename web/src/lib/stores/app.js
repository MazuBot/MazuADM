import { get, writable } from 'svelte/store'
import * as api from '$lib/data/api'
import { connect, disconnect } from '$lib/websocket.js'
import { isValidId, pickDefaultRound } from './app-logic.js'

export const ready = writable(false)
export const challenges = writable([])
export const teams = writable([])
export const exploits = writable([])
export const exploitRuns = writable([])
export const rounds = writable([])
export const jobs = writable([])
export const flags = writable([])
export const settings = writable([])
export const containers = writable([])
export const containerRunners = writable({})

export const selectedChallengeId = writable(null)
export const selectedRoundId = writable(null)
export const selectedFlagRoundId = writable(null)

let started = false

function ensureSelections() {
  const currentChallenges = get(challenges)
  const currentRounds = get(rounds)
  const currentChallengeId = get(selectedChallengeId)
  const currentRoundId = get(selectedRoundId)
  const currentFlagRoundId = get(selectedFlagRoundId)

  if (!isValidId(currentChallenges, currentChallengeId)) {
    selectedChallengeId.set(currentChallenges[0]?.id ?? null)
  }

  if (!isValidId(currentRounds, currentRoundId)) {
    selectedRoundId.set(pickDefaultRound(currentRounds))
  }

  if (currentFlagRoundId && !isValidId(currentRounds, currentFlagRoundId)) {
    selectedFlagRoundId.set(null)
  }
}

export async function loadAll() {
  ready.set(false)
  const [c, t, e, r, ro, s] = await Promise.all([
    api.challenges(),
    api.teams(),
    api.exploits(),
    api.exploitRuns(),
    api.rounds(),
    api.settings()
  ])

  challenges.set(c)
  teams.set(t)
  exploits.set(e)
  exploitRuns.set(r)
  rounds.set(ro)
  settings.set(s)

  ensureSelections()
  ready.set(true)
}

export async function loadJobs(roundId) {
  if (!roundId) {
    jobs.set([])
    return
  }
  const result = await api.jobs(roundId)
  if (roundId === get(selectedRoundId)) jobs.set(result)
}

export async function loadFlags(roundId) {
  flags.set(await api.flags(roundId))
}

export async function loadContainers() {
  containers.set(await api.containers())
}

export async function loadRunners(containerId) {
  const runners = await api.containerRunners(containerId)
  containerRunners.update((current) => ({ ...current, [containerId]: runners }))
}

export function resetContainers() {
  containers.set([])
  containerRunners.set({})
}

export async function createRound() {
  const id = await api.createRound()
  selectedRoundId.set(id)
  jobs.set([])
  return id
}

export async function runRound(id) {
  if (!id) return
  await api.runRound(id)
}

export async function rerunRound(id) {
  if (!id) return
  await api.rerunRound(id)
}

function handleWsMessage(msg) {
  const { type, data } = msg
  switch (type) {
    case 'challenge_created':
      challenges.update((list) => [...list, data])
      break
    case 'challenge_updated':
      challenges.update((list) => list.map((c) => (c.id === data.id ? data : c)))
      break
    case 'challenge_deleted':
      challenges.update((list) => list.filter((c) => c.id !== data))
      break
    case 'team_created':
      teams.update((list) => [...list, data])
      break
    case 'team_updated':
      teams.update((list) => list.map((t) => (t.id === data.id ? data : t)))
      break
    case 'team_deleted':
      teams.update((list) => list.filter((t) => t.id !== data))
      break
    case 'exploit_created':
      exploits.update((list) => [...list, data])
      break
    case 'exploit_updated':
      exploits.update((list) => list.map((e) => (e.id === data.id ? data : e)))
      break
    case 'exploit_deleted':
      exploits.update((list) => list.filter((e) => e.id !== data))
      break
    case 'exploit_run_created':
      exploitRuns.update((list) => [...list, data])
      break
    case 'exploit_run_updated':
      exploitRuns.update((list) => list.map((r) => (r.id === data.id ? data : r)))
      break
    case 'exploit_run_deleted':
      exploitRuns.update((list) => list.filter((r) => r.id !== data))
      break
    case 'exploit_runs_reordered':
      loadAll()
      break
    case 'round_created':
      rounds.update((list) => [data, ...list])
      break
    case 'round_updated':
      rounds.update((list) => list.map((r) => (r.id === data.id ? data : r)))
      break
    case 'job_updated': {
      const currentRoundId = get(selectedRoundId)
      if (data.round_id === currentRoundId) {
        jobs.update((list) => {
          const idx = list.findIndex((j) => j.id === data.id)
          if (idx >= 0) return [...list.slice(0, idx), data, ...list.slice(idx + 1)]
          return [...list, data]
        })
      }
      break
    }
    case 'flag_created': {
      const currentFlagRoundId = get(selectedFlagRoundId)
      if (!currentFlagRoundId || data.round_id === currentFlagRoundId) {
        flags.update((list) => [...list, data])
      }
      break
    }
    case 'setting_updated':
      settings.update((list) =>
        list.map((s) => (s.key === data.key ? { ...s, value: data.value } : s))
      )
      break
    case 'relation_updated':
      break
  }
}

export function start() {
  if (started) return
  started = true
  connect(handleWsMessage)
}

export function stop() {
  if (!started) return
  started = false
  disconnect()
}

export const app = {
  ready,
  challenges,
  teams,
  exploits,
  exploitRuns,
  rounds,
  jobs,
  flags,
  settings,
  containers,
  containerRunners,
  selectedChallengeId,
  selectedRoundId,
  selectedFlagRoundId,
  loadAll,
  loadJobs,
  loadFlags,
  loadContainers,
  loadRunners,
  resetContainers,
  createRound,
  runRound,
  rerunRound,
  start,
  stop
}
