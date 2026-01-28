import { describe, expect, it } from 'vitest'
import {
  isValidId,
  pickDefaultRound,
  resolveBoardSelection,
  resolveRoundSelection,
  resolveFlagSelection
} from './app-logic.js'

describe('isValidId', () => {
  it('returns true only for matching ids', () => {
    const list = [{ id: 1 }, { id: 3 }]
    expect(isValidId(list, 1)).toBe(true)
    expect(isValidId(list, 2)).toBe(false)
    expect(isValidId(list, null)).toBe(false)
  })
})

describe('pickDefaultRound', () => {
  it('returns the first non-pending round when available', () => {
    const rounds = [
      { id: 1, status: 'pending' },
      { id: 2, status: 'running' },
      { id: 3, status: 'pending' }
    ]
    expect(pickDefaultRound(rounds)).toBe(2)
  })

  it('returns the first round when all are pending', () => {
    const rounds = [
      { id: 5, status: 'pending' },
      { id: 6, status: 'pending' }
    ]
    expect(pickDefaultRound(rounds)).toBe(5)
  })

  it('returns null when empty', () => {
    expect(pickDefaultRound([])).toBe(null)
  })
})

describe('resolveBoardSelection', () => {
  it('prefers a valid route id', () => {
    const challenges = [{ id: 1 }, { id: 2 }]
    const result = resolveBoardSelection({ challenges, selectedId: 1, routeId: 2 })
    expect(result.selectedId).toBe(2)
  })

  it('falls back to the first challenge on invalid route', () => {
    const challenges = [{ id: 7 }, { id: 8 }]
    const result = resolveBoardSelection({ challenges, selectedId: null, routeId: 99 })
    expect(result.selectedId).toBe(7)
  })
})

describe('resolveRoundSelection', () => {
  it('keeps a valid route round', () => {
    const rounds = [{ id: 1, status: 'pending' }, { id: 2, status: 'running' }]
    const result = resolveRoundSelection({ rounds, selectedId: null, routeId: 2 })
    expect(result.selectedId).toBe(2)
  })

  it('chooses a default round when route is invalid', () => {
    const rounds = [{ id: 1, status: 'pending' }, { id: 2, status: 'running' }]
    const result = resolveRoundSelection({ rounds, selectedId: null, routeId: 99 })
    expect(result.selectedId).toBe(2)
  })
})

describe('resolveFlagSelection', () => {
  it('returns valid round ids', () => {
    const rounds = [{ id: 1 }, { id: 2 }]
    const result = resolveFlagSelection({ rounds, routeId: 2 })
    expect(result.selectedId).toBe(2)
  })

  it('returns null for invalid ids', () => {
    const rounds = [{ id: 1 }]
    const result = resolveFlagSelection({ rounds, routeId: 5 })
    expect(result.selectedId).toBe(null)
  })
})
