import { get, writable } from 'svelte/store'
import * as api from '$lib/data/api'

export const flags = writable([])
export const selectedFlagRoundId = writable(null)

export async function loadFlags(roundId) {
  flags.set(await api.flags(roundId))
}

export async function submitFlag(payload) {
  const flag = await api.submitFlag(payload)
  const currentRoundId = get(selectedFlagRoundId)
  if (!currentRoundId || flag.round_id === currentRoundId) {
    flags.update((list) => (list.some((f) => f.id === flag.id) ? list : [...list, flag]))
  }
  return flag
}
