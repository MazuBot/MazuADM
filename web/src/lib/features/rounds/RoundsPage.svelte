<script>
  import { AnsiUp } from 'ansi_up';
  import * as api from '$lib/data/api';
  import Modal from '$lib/ui/Modal.svelte';
  import FilterBar from '$lib/ui/FilterBar.svelte';
  import Icon from '$lib/ui/Icon.svelte';
  import { buildStatusOptions } from '$lib/utils/filters.js';
  import { getChallengeName, getExploitName, getTeamDisplay } from '$lib/utils/lookup.js';

  let { rounds, jobs, teams, challenges, exploits, exploitRuns, selectedRoundId, onSelectRound, onNewRound, onRunRound, onRerunUnflagged, onRefresh } = $props();

  const ansi_up = new AnsiUp();
  function renderAnsi(text) {
    return ansi_up.ansi_to_html(text || '');
  }

  let selectedJob = $state(null);
  let selectedJobDetail = $state(null);
  let jobDetailLoading = $state(false);
  let jobDetailError = $state('');
  let jobDetailToken = 0;
  let draggingJob = $state(null);
  let challengeFilterId = $state('');
  let teamFilterId = $state('');
  let statusFilter = $state('');

  function getSelectedRound() {
    return rounds.find(r => r.id === selectedRoundId);
  }

  async function handleRunClick() {
    if (!selectedRoundId) return;
    const round = getSelectedRound();
    if (round && round.status !== 'pending') {
      if (confirm(`Round ${selectedRoundId} is ${round.status}. Re-running will kill all running jobs and reset all later rounds. Continue?`)) {
        await api.rerunRound(selectedRoundId);
      }
    } else {
      onRunRound(selectedRoundId);
    }
  }

  async function handleRerunUnflaggedClick() {
    if (!selectedRoundId) return;
    const round = getSelectedRound();
    if (!round || round.status !== 'running') return;
    if (!confirm(`Rerun all non-flag jobs for running round ${selectedRoundId}?`)) return;
    await onRerunUnflagged?.(selectedRoundId);
    onRefresh?.();
  }

  function getExploitRunInfo(runId) {
    return exploitRuns.find((r) => r.id === runId);
  }

  function formatStatus(status) {
    return status === 'flag' ? '🚩 FLAG' : status;
  }

  function formatTimestamp(value) {
    if (!value) return '-';
    const parsed = new Date(value);
    if (Number.isNaN(parsed.getTime())) return '-';
    return parsed.toLocaleString();
  }

  function closeModal() {
    jobDetailToken += 1;
    selectedJob = null;
    selectedJobDetail = null;
    jobDetailLoading = false;
    jobDetailError = '';
  }

  async function openJob(job) {
    selectedJob = job;
    selectedJobDetail = null;
    jobDetailError = '';
    const token = ++jobDetailToken;
    jobDetailLoading = true;
    try {
      const detail = await api.job(job.id);
      if (token === jobDetailToken) {
        selectedJobDetail = detail;
      }
    } catch (err) {
      if (token === jobDetailToken) {
        jobDetailError = 'Failed to load logs.';
      }
    } finally {
      if (token === jobDetailToken) {
        jobDetailLoading = false;
      }
    }
  }

  function jobScheduleMs(job) {
    if (!job.schedule_at) return null;
    const ms = Date.parse(job.schedule_at);
    return Number.isNaN(ms) ? null : ms;
  }

  function sortedJobs(list = jobs) {
    return [...list].sort((a, b) => {
      const aSchedule = jobScheduleMs(a);
      const bSchedule = jobScheduleMs(b);

      if (aSchedule !== null && bSchedule !== null) {
        return aSchedule - bSchedule || a.id - b.id;
      }
      if (aSchedule !== null) return -1;
      if (bSchedule !== null) return 1;
      return b.priority - a.priority || a.id - b.id;
    });
  }

  function filterJobs() {
    const teamId = teamFilterId ? Number(teamFilterId) : null;
    const challengeId = challengeFilterId ? Number(challengeFilterId) : null;
    return jobs.filter((job) => {
      if (statusFilter && job.status !== statusFilter) return false;
      if (teamId && Number(job.team_id) !== teamId) return false;
      if (challengeId) {
        const run = getExploitRunInfo(job.exploit_run_id);
        if (!run || Number(run.challenge_id) !== challengeId) return false;
      }
      return true;
    });
  }

  let filteredJobs = $derived(filterJobs());
  let availableStatuses = $derived(buildStatusOptions(jobs));

  function onDragStart(e, job) {
    if (job.status !== 'pending') { e.preventDefault(); return; }
    draggingJob = job;
    e.dataTransfer.effectAllowed = 'move';
  }

  async function onDrop(e, targetJob) {
    e.preventDefault();
    if (!draggingJob || draggingJob.id === targetJob.id) return;
    if (draggingJob.status !== 'pending' || targetJob.status !== 'pending') return;

    const sorted = sortedJobs().filter(j => j.status === 'pending');
    const fromIdx = sorted.findIndex(j => j.id === draggingJob.id);
    const toIdx = sorted.findIndex(j => j.id === targetJob.id);
    const reordered = [...sorted];
    reordered.splice(fromIdx, 1);
    reordered.splice(toIdx, 0, draggingJob);

    const maxPrio = Math.max(...reordered.map(j => j.priority), 0);
    const updates = reordered.map((j, i) => ({ id: j.id, priority: maxPrio - i }));
    await api.reorderJobs(updates);
    draggingJob = null;
    onRefresh?.();
  }

  async function runJob(job, e) {
    e.stopPropagation();
    if (job.status === 'running') {
      await api.stopJob(job.id);
    } else {
      await api.enqueueExistingJob(job.id);
    }
  }
</script>

<div class="rounds-panel">
  <div class="controls">
    <button onclick={onNewRound}>New Round</button>
    <select
      value={selectedRoundId ?? ''}
      onchange={(e) => onSelectRound(e.target.value ? Number(e.target.value) : null)}
    >
      <option value="">Select round</option>
      {#each rounds as r}
        <option value={r.id}>Round {r.id} ({r.status})</option>
      {/each}
    </select>
    <button onclick={handleRunClick} disabled={!selectedRoundId}>Run</button>
    <button
      onclick={handleRerunUnflaggedClick}
      disabled={!selectedRoundId || getSelectedRound()?.status !== 'running'}
    >
      Rerun Unflagged
    </button>
  </div>
  <FilterBar
    bind:challengeId={challengeFilterId}
    bind:teamId={teamFilterId}
    bind:status={statusFilter}
    {challenges}
    {teams}
    statuses={availableStatuses}
    onReset={() => {
      challengeFilterId = '';
      teamFilterId = '';
      statusFilter = '';
    }}
  />

  {#if filteredJobs.length}
    <table>
      <thead>
        <tr>
          <th>ID</th>
          <th>Challenge</th>
          <th>Exploit</th>
          <th>Team</th>
          <th>Create Reason</th>
          <th>Container</th>
          <th>Priority</th>
          <th>Status</th>
          <th>Duration</th>
          <th></th>
        </tr>
      </thead>
      <tbody>
        {#each sortedJobs(filteredJobs) as j}
          <tr 
            class={j.status} 
            class:draggable={j.status === 'pending'}
            class:dragging={draggingJob?.id === j.id}
            draggable={j.status === 'pending'}
            ondragstart={(e) => onDragStart(e, j)}
            ondragover={(e) => e.preventDefault()}
            ondrop={(e) => onDrop(e, j)}
            ondragend={() => draggingJob = null}
            onclick={() => openJob(j)} 
            style="cursor:pointer"
          >
            <td>{j.id}</td>
            <td>{getChallengeName(challenges, getExploitRunInfo(j.exploit_run_id)?.challenge_id)}</td>
            <td>{getExploitName(exploits, getExploitRunInfo(j.exploit_run_id)?.exploit_id)}</td>
            <td><span class="truncate">{getTeamDisplay(teams, j.team_id)}</span></td>
            <td><span class="truncate">{j.create_reason || '-'}</span></td>
            <td>{j.container_id ? j.container_id.slice(0, 12) : '-'}</td>
            <td>{j.priority}</td>
            <td>{formatStatus(j.status)}</td>
            <td>{j.duration_ms ? `${j.duration_ms}ms` : '-'}</td>
            <td>
              <button
                class={`play-btn ${j.status === 'running' ? 'stop' : ''}`}
                onclick={(e) => runJob(j, e)}
                title={j.status === 'running' ? 'Stop now' : j.status === 'pending' ? 'Enqueue now' : 'Re-run'}
              >
                {#if j.status === 'running'}<Icon name="stop" />{:else if j.status === 'pending'}<Icon name="play" />{:else}<Icon name="rotate" />{/if}
              </button>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</div>

{#if selectedJob}
  {@const modalJob = selectedJobDetail ?? selectedJob}
  <Modal wide onClose={closeModal}>
    <h3 class="job-modal-header">
      <span>Job #{modalJob.id}</span>
      <span><code>{getChallengeName(challenges, getExploitRunInfo(modalJob.exploit_run_id)?.challenge_id)}</code></span>
      <span class={`job-status status-${modalJob.status || 'unknown'}`}>{formatStatus(modalJob.status)}</span>
    </h3>
    <div class="job-info">
      <p><strong>Exploit:</strong> {getExploitName(exploits, getExploitRunInfo(modalJob.exploit_run_id)?.exploit_id)}</p>
      <p><strong>Team:</strong> <span class="truncate">{getTeamDisplay(teams, modalJob.team_id)}</span></p>
      <p><strong>Priority:</strong> {modalJob.priority}</p>
      <p><strong>Start reason:</strong> {modalJob.create_reason || '-'}</p>
      <p><strong>Scheduled at:</strong> {formatTimestamp(modalJob.schedule_at)}</p>
      <p><strong>Started at:</strong> {formatTimestamp(modalJob.started_at)}</p>
      <p><strong>Finished at:</strong> {formatTimestamp(modalJob.finished_at)}</p>
      <p><strong>Duration:</strong> {modalJob.duration_ms ? `${modalJob.duration_ms}ms` : '-'}</p>
      {#if modalJob.container_id}<p><strong>Container:</strong> <code>{modalJob.container_id.slice(0, 12)}</code></p>{/if}
    </div>
    {#if jobDetailLoading}
      <p class="muted">Loading logs...</p>
    {:else if jobDetailError}
      <p class="muted">{jobDetailError}</p>
    {:else}
      {#if selectedJobDetail?.stdout}
        <div class="modal-section-label">Stdout</div>
        <pre class="log-output">{@html renderAnsi(selectedJobDetail.stdout)}</pre>
      {/if}
      {#if selectedJobDetail?.stderr}
        <div class="modal-section-label">Stderr</div>
        <pre class="log-output stderr">{@html renderAnsi(selectedJobDetail.stderr)}</pre>
      {/if}
      {#if !selectedJobDetail?.stdout && !selectedJobDetail?.stderr}
        <p class="muted">No logs available.</p>
      {/if}
    {/if}
    <div class="modal-actions">
      <button onclick={closeModal}>Close</button>
    </div>
  </Modal>
{/if}

<style>
  .draggable { cursor: grab; }
  .dragging { opacity: 0.4; background: #333; }
  .play-btn { background: transparent; border: none; cursor: pointer; font-size: 0.9rem; padding: 0.2rem 0.4rem; opacity: 0.6; color: white; }
  .play-btn:hover { opacity: 1; }
  .play-btn.stop { color: #ff6b6b; }
  .job-modal-header { display: flex; align-items: center; justify-content: space-between; gap: 1rem; }
</style>
