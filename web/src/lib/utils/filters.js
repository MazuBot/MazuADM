export function buildStatusOptions(list, key = 'status') {
  return [...new Set((list ?? []).map((item) => item?.[key]).filter(Boolean))].sort()
}
