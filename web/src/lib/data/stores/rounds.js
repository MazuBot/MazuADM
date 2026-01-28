import { get, writable } from 'svelte/store'
import * as api from '$lib/data/api'

export const rounds = writable([])
export const jobs = writable([])
export const selectedRoundId = writable(null)

export async function loadRounds() {
  rounds.set(await api.rounds())
}

export async function loadJobs(roundId) {
  if (!roundId) {
    jobs.set([])
    return
  }
  const result = await api.jobs(roundId)
  if (roundId === get(selectedRoundId)) jobs.set(result)
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

export async function scheduleUnflaggedRound(id) {
  if (!id) return
  await api.scheduleUnflaggedRound(id)
}
