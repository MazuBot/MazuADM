export function isValidId(list, id) {
  if (!id || !Array.isArray(list)) return false
  return list.some((item) => item.id === id)
}

export function pickDefaultRound(rounds) {
  if (!rounds?.length) return null
  const nonPending = rounds.filter((round) => round.status !== 'pending')
  return nonPending.length ? nonPending[0].id : rounds[0].id
}

export function resolveBoardSelection({ challenges, selectedId, routeId }) {
  let nextSelected = selectedId
  if (isValidId(challenges, routeId)) nextSelected = routeId
  if (!isValidId(challenges, nextSelected)) nextSelected = challenges?.[0]?.id ?? null
  return { selectedId: nextSelected }
}

export function resolveRoundSelection({ rounds, selectedId, routeId }) {
  let nextSelected = selectedId
  if (isValidId(rounds, routeId)) nextSelected = routeId
  if (!isValidId(rounds, nextSelected)) nextSelected = pickDefaultRound(rounds)
  return { selectedId: nextSelected }
}

export function resolveFlagSelection({ rounds, routeId }) {
  const nextSelected = isValidId(rounds, routeId) ? routeId : null
  return { selectedId: nextSelected }
}
