import { fetchJson } from './index.js'

export const flags = (roundId) => fetchJson(roundId ? `/flags?round_id=${roundId}` : '/flags')
