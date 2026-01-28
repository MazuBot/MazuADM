import { get, writable } from 'svelte/store'
import { isValidId, pickDefaultRound } from '$lib/stores/app-logic.js'
import { challenges } from './entities.js'
import { rounds, selectedRoundId } from './rounds.js'
import { selectedFlagRoundId } from './flags.js'

export const selectedChallengeId = writable(null)

export function ensureSelections() {
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
