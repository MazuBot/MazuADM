import { fetchJson } from './index.js'

export const jobs = (roundId) => fetchJson(`/jobs?round_id=${roundId}`)
export const job = (id) => fetchJson(`/jobs/${id}`)
export const reorderJobs = (updates) => fetchJson('/jobs/reorder', { method: 'POST', body: JSON.stringify(updates) })
export const enqueueSingleJob = (exploit_run_id, team_id, debug) => 
  fetchJson(`/jobs/enqueue${debug ? '?debug=1' : ''}`, { method: 'POST', body: JSON.stringify({ exploit_run_id, team_id }) })
export const enqueueExistingJob = (job_id, debug) => fetchJson(`/jobs/${job_id}/enqueue${debug ? '?debug=1' : ''}`, { method: 'POST' })
export const stopJob = (job_id) => fetchJson(`/jobs/${job_id}/stop`, { method: 'POST' })
