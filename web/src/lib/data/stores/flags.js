import { writable } from 'svelte/store'
import * as api from '$lib/data/api'

export const flags = writable([])
export const selectedFlagRoundId = writable(null)

export async function loadFlags(roundId) {
  flags.set(await api.flags(roundId))
}
