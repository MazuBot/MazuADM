import { get, writable } from 'svelte/store'
import * as api from '$lib/data/api'
import { connect, disconnect } from '$lib/websocket.js'
import { pushToast } from '$lib/ui/toastStore.js'
import { challenges, exploits, exploitRuns, loadAllEntities, teams } from './entities.js'
import { flags, loadFlags, selectedFlagRoundId, submitFlag } from './flags.js'
import { containers, containerRunners, loadContainers, loadRunners, resetContainers } from './containers.js'
import { jobs, loadJobs, rounds, selectedRoundId, createRound, loadRounds, rerunRound, runRound, rerunUnflaggedRound } from './rounds.js'
import { selectedChallengeId, ensureSelections } from './selections.js'

export const ready = writable(false)
export const settings = writable([])
export const wsConnections = writable([])
export const roundCreatedAt = writable(0)

async function loadSettings() {
  settings.set(await api.settings())
}

export async function loadAll() {
  ready.set(false)
  await Promise.all([loadAllEntities(), loadRounds(), loadSettings()])
  ensureSelections()
  ready.set(true)
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
      rounds.update((list) => [{ ...data, jobs_ready: false }, ...list])
      pushToast(`Round #${data.id} created.`, 'success')
      roundCreatedAt.set(Date.now())
      break
    case 'round_updated':
      {
        let statusToast = null
        rounds.update((list) =>
          list.map((r) => {
            if (r.id !== data.id) return r
            const next = { ...r, ...data }
            if (r.status && next.status && r.status !== next.status) {
              if (next.status === 'running') statusToast = `Round #${next.id} started.`
              else if (next.status === 'finished') statusToast = `Round #${next.id} finished.`
            }
            return next
          })
        )
        if (statusToast) pushToast(statusToast, 'success')
      }
      break
    case 'round_jobs_ready': {
      const currentRoundId = get(selectedRoundId)
      rounds.update((list) =>
        list.map((r) => (r.id === data.round_id ? { ...r, jobs_ready: true } : r))
      )
      if (data?.round_id) {
        if (data.success) {
          const count = Number.isFinite(data.created) ? ` (${data.created})` : ''
          pushToast(`Round #${data.round_id} jobs created${count}.`, 'success')
        } else {
          pushToast(`Round #${data.round_id} job creation failed.`, 'error')
        }
      }
      if (currentRoundId && data?.round_id === currentRoundId) {
        loadJobs(currentRoundId)
      }
      break
    }
    case 'job_created': {
      const currentRoundId = get(selectedRoundId)
      if (data.round_id === currentRoundId) {
        jobs.update((list) => [...list, data])
      }
      break
    }
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
    case 'jobs_changed': {
      const currentRoundId = get(selectedRoundId)
      if (currentRoundId && data?.round_id === currentRoundId) {
        loadJobs(currentRoundId)
      }
      break
    }
    case 'flag_created': {
      const currentFlagRoundId = get(selectedFlagRoundId)
      if (!currentFlagRoundId || data.round_id === currentFlagRoundId) {
        flags.update((list) => (list.some((f) => f.id === data.id) ? list : [data, ...list]))
      }
      break
    }
    case 'setting_updated':
      settings.update((list) =>
        list.map((s) => (s.key === data.key ? { ...s, value: data.value } : s))
      )
      break
    case 'container_created':
      containers.update((list) => {
        const idx = list.findIndex((c) => c.id === data.id)
        if (idx >= 0) return [...list.slice(0, idx), data, ...list.slice(idx + 1)]
        return [...list, data]
      })
      break
    case 'container_updated':
      containers.update((list) => {
        const idx = list.findIndex((c) => c.id === data.id)
        if (idx >= 0) return [...list.slice(0, idx), data, ...list.slice(idx + 1)]
        return [...list, data]
      })
      break
    case 'container_execs_updated':
      containers.update((list) =>
        list.map((c) =>
          c.id === data.id
            ? { ...c, running_execs: data.running_execs, max_execs: data.max_execs }
            : c
        )
      )
      break
    case 'container_deleted':
      containers.update((list) => list.filter((c) => c.id !== data))
      containerRunners.update((current) => {
        if (!(data in current)) return current
        const { [data]: _removed, ...rest } = current
        return rest
      })
      break
    case 'connection_info_updated':
      break
    case 'ws_connections':
      wsConnections.set(data)
      break
  }
}

let started = false

export function start() {
  if (started) return
  started = true
  connect(handleWsMessage)
}

export function restart() {
  disconnect()
  started = false
  start()
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
  wsConnections,
  roundCreatedAt,
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
  rerunUnflaggedRound,
  submitFlag,
  start,
  restart,
  stop
}
