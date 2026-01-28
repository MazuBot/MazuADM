import { writable } from 'svelte/store'
import * as api from '$lib/data/api'

export const containers = writable([])
export const containerRunners = writable({})

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
