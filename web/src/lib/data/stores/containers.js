import { writable } from 'svelte/store'
import * as api from '$lib/data/api'

export const containers = writable([])

export async function loadContainers() {
  containers.set(await api.containers())
}
