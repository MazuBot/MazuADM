import { writable } from 'svelte/store'

export const TOAST_TIMEOUT_MS = 10000

export const toasts = writable([])

let nextId = 1
const timers = new Map()

export function pushToast(message, variant = 'success') {
  const id = nextId++
  toasts.update((items) => [...items, { id, message, variant }])
  const timer = setTimeout(() => removeToast(id), TOAST_TIMEOUT_MS)
  timers.set(id, timer)
  return id
}

export function removeToast(id) {
  const timer = timers.get(id)
  if (timer) {
    clearTimeout(timer)
    timers.delete(id)
  }
  toasts.update((items) => items.filter((toast) => toast.id !== id))
}
