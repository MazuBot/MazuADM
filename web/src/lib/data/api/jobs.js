import { fetchJson } from './index.js'

export const jobs = (roundId) => fetchJson(`/jobs?round_id=${roundId}`)
export const reorderJobs = (updates) => fetchJson('/jobs/reorder', { method: 'POST', body: JSON.stringify(updates) })
export const runSingleJob = (exploit_run_id, team_id) =>
  fetchJson('/jobs/run', { method: 'POST', body: JSON.stringify({ exploit_run_id, team_id }) })
export const runExistingJob = (job_id) => fetchJson(`/jobs/${job_id}/run`, { method: 'POST' })
