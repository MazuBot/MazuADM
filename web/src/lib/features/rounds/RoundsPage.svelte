<script>
  import { onDestroy, onMount } from 'svelte';
  import { AnsiUp } from 'ansi_up';
  import * as api from '$lib/data/api';
  import { roundCreatedAt } from '$lib/data/stores/app.js';
  import Modal from '$lib/ui/Modal.svelte';
  import Icon from '$lib/ui/Icon.svelte';
  import { buildStatusOptions } from '$lib/utils/filters.js';
  import { getChallengeName, getExploitName, getTeamDisplay } from '$lib/utils/lookup.js';
  import { formatApiError, pushToast } from '$lib/ui/toastStore.js';

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
  let dragPreviewEl = $state(null);
  let optimisticPriority = $state(null);
  let dragPreviewPriority = $state(null);
  let dragStartIndex = $state(null);
  let challengeFilterId = $state('');
  let teamFilterId = $state('');
  let statusFilter = $state('');
  let reasonFilter = $state('');
  let searchQuery = $state('');
  let canResetFilters = $derived(Boolean(challengeFilterId || teamFilterId || statusFilter || reasonFilter || searchQuery));

  function getSelectedRound() {
    return rounds.find(r => r.id === selectedRoundId);
  }

  async function handleRunClick() {
    if (!selectedRoundId) return;
    const round = getSelectedRound();
    try {
      if (round && round.status !== 'pending') {
        if (!confirm(`Round ${selectedRoundId} is ${round.status}. Re-running will kill all running jobs and reset all later rounds. Continue?`)) {
          return;
        }
        await api.rerunRound(selectedRoundId);
      } else {
        await onRunRound(selectedRoundId);
      }
    } catch (error) {
      pushToast(formatApiError(error, `Failed to run round #${selectedRoundId}.`), 'error');
    }
  }

  async function handleRerunUnflaggedClick() {
    if (!selectedRoundId) return;
    const round = getSelectedRound();
    if (!round || round.status !== 'running') return;
    if (!confirm(`Rerun all non-flag jobs for running round ${selectedRoundId}?`)) return;
    try {
      await onRerunUnflagged?.(selectedRoundId);
      onRefresh?.();
      pushToast(`Round #${selectedRoundId} re-run (unflagged) started.`, 'success');
    } catch (error) {
      pushToast(formatApiError(error, `Failed to rerun unflagged jobs for round #${selectedRoundId}.`), 'error');
    }
  }

  function getExploitRunInfo(runId) {
    return exploitRuns.find((r) => r.id === runId);
  }

  function normalizeStatus(status) {
    if (!status) return 'unknown';
    return status.split(':')[0] || 'unknown';
  }

  function formatStatus(status) {
    if (!status) return 'unknown';
    if (status === 'flag') return '🚩 FLAG';
    return status;
  }

  function statusClass(status) {
    return `job-status status-${normalizeStatus(status)}`;
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
      const aPriority = getJobPriority(a);
      const bPriority = getJobPriority(b);
      return bPriority - aPriority || a.id - b.id;
    });
  }

  async function handleNewRoundClick() {
    try {
      startNewRoundCooldown(NEW_ROUND_COOLDOWN_MS);
      const id = await onNewRound?.();
    } catch (error) {
      pushToast(formatApiError(error, 'Failed to create round.'), 'error');
    }
  }

  function getReasonBase(reason) {
    if (!reason) return '';
    const idx = reason.indexOf(':');
    return idx >= 0 ? reason.substring(0, idx) : reason;
  }

  function highlight(text) {
    if (!text) return '-';
    const q = searchQuery.trim();
    if (q.length < 2) return text;
    const regex = new RegExp(`(${q.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')})`, 'gi');
    return text.replace(regex, '<mark>$1</mark>');
  }

  let filteredJobs = $derived.by(() => {
    const teamId = teamFilterId ? Number(teamFilterId) : null;
    const challengeId = challengeFilterId ? Number(challengeFilterId) : null;
    const query = searchQuery.toLowerCase().trim();
    return jobs.filter((job) => {
      if (statusFilter && job.status !== statusFilter) return false;
      if (reasonFilter && getReasonBase(job.create_reason) !== reasonFilter) return false;
      if (teamId && Number(job.team_id) !== teamId) return false;
      if (challengeId) {
        const run = getExploitRunInfo(job.exploit_run_id);
        if (!run || Number(run.challenge_id) !== challengeId) return false;
      }
      if (query) {
        const run = getExploitRunInfo(job.exploit_run_id);
        const searchable = [
          String(job.id),
          getChallengeName(challenges, run?.challenge_id),
          getExploitName(exploits, run?.exploit_id),
          job.create_reason || '',
          job.container_id ? job.container_id.slice(0, 12) : ''
        ].join(' ').toLowerCase();
        if (!searchable.includes(query)) return false;
      }
      return true;
    });
  });
  let availableStatuses = $derived(buildStatusOptions(jobs));
  let availableReasons = $derived([...new Set(jobs.map(j => getReasonBase(j.create_reason)))].filter(Boolean).sort());
  let selectedRound = $derived(getSelectedRound());
  let isPendingRound = $derived(selectedRound?.status === 'pending');
  let isJobsCreating = $derived(selectedRound?.status === 'pending' && selectedRound?.jobs_ready === false);
  let newRoundCooldown = $state(false);
  let newRoundCooldownTimer = null;
  let lastRoundCreatedAt = $state(0);

  const NEW_ROUND_COOLDOWN_MS = 3000;

  onMount(() => {
    const onKeydown = (e) => {
      if (e.key === 'Escape' && draggingJob) {
        e.preventDefault();
        cancelDrag();
      }
    };
    window.addEventListener('keydown', onKeydown);
    return () => window.removeEventListener('keydown', onKeydown);
  });

  onDestroy(() => {
    if (newRoundCooldownTimer) {
      clearTimeout(newRoundCooldownTimer);
      newRoundCooldownTimer = null;
    }
  });

  $effect(() => {
    if ($roundCreatedAt && $roundCreatedAt !== lastRoundCreatedAt) {
      lastRoundCreatedAt = $roundCreatedAt;
      startNewRoundCooldown(NEW_ROUND_COOLDOWN_MS);
    }
  });

  function startNewRoundCooldown(ms) {
    newRoundCooldown = true;
    if (newRoundCooldownTimer) clearTimeout(newRoundCooldownTimer);
    newRoundCooldownTimer = setTimeout(() => {
      newRoundCooldown = false;
      newRoundCooldownTimer = null;
    }, ms);
  }

  function getJobPriority(job) {
    if (optimisticPriority?.id === job.id) return optimisticPriority.priority;
    if (draggingJob?.id === job.id && dragPreviewPriority !== null) return dragPreviewPriority;
    return job.priority;
  }

  function cleanupDragPreview() {
    if (dragPreviewEl?.parentNode) {
      dragPreviewEl.parentNode.removeChild(dragPreviewEl);
    }
    dragPreviewEl = null;
  }

  function clearDragOver() {
    dragPreviewPriority = null;
    dragStartIndex = null;
  }

  function cancelDrag() {
    cleanupDragPreview();
    clearDragOver();
    draggingJob = null;
  }

  function onDragStart(e, job) {
    if (job.status !== 'pending') { e.preventDefault(); return; }
    draggingJob = job;
    clearDragOver();
    const pending = sortedJobs(filteredJobs).filter(j => j.status === 'pending');
    dragStartIndex = pending.findIndex(j => j.id === job.id);
    if (!e.dataTransfer) return;
    e.dataTransfer.effectAllowed = 'move';
    cleanupDragPreview();
    const rowEl = e.currentTarget;
    if (!rowEl) return;
    const previewTable = document.createElement('table');
    const previewTbody = document.createElement('tbody');
    const previewRow = rowEl.cloneNode(true);
    previewTbody.appendChild(previewRow);
    previewTable.appendChild(previewTbody);
    previewTable.classList.add('drag-preview');
    const rect = rowEl.getBoundingClientRect();
    previewTable.style.width = `${rect.width}px`;
    previewTable.style.position = 'absolute';
    previewTable.style.top = '-1000px';
    previewTable.style.left = '-1000px';
    document.body.appendChild(previewTable);
    dragPreviewEl = previewTable;
    e.dataTransfer.setDragImage(previewTable, 16, 16);
  }

  function onRowDragOver(e, job) {
    e.preventDefault();
    if (!draggingJob || draggingJob.status !== 'pending') return;
    if (job.status !== 'pending') return;
    if (job.id === draggingJob.id) return;
    const baseTargetPriority = getJobPriority(job);
    const targetPriority = Number.isFinite(baseTargetPriority) ? baseTargetPriority : 0;
    let nextPriority = null;
    if (targetPriority === draggingJob.priority) {
      const pending = sortedJobs(filteredJobs).filter(j => j.status === 'pending');
      const targetIndex = pending.findIndex(j => j.id === job.id);
      if (targetIndex < 0 || dragStartIndex === null || dragStartIndex < 0) return;
      if (targetIndex === dragStartIndex) return;
      nextPriority = targetIndex < dragStartIndex ? targetPriority + 1 : targetPriority - 1;
    } else {
      const rect = e.currentTarget?.getBoundingClientRect?.();
      const midpoint = rect ? rect.top + rect.height / 2 : e.clientY;
      nextPriority = e.clientY < midpoint ? targetPriority + 1 : targetPriority - 1;
    }
    if (nextPriority === null) return;
    if (dragPreviewPriority === nextPriority) return;
    dragPreviewPriority = nextPriority;
  }

  function getDisplayedJobs(list = filteredJobs) {
    const sorted = sortedJobs(list);
    if (!draggingJob || draggingJob.status !== 'pending') return sorted;
    const pending = sorted.filter(j => j.status === 'pending');
    if (dragPreviewPriority === null) return sorted;
    const reordered = [...pending]
      .map((job) => job.id === draggingJob.id ? { ...job, priority: dragPreviewPriority } : job)
      .sort((a, b) => b.priority - a.priority || a.id - b.id);
    let pendingIdx = 0;
    return sorted.map((job) => job.status === 'pending' ? reordered[pendingIdx++] : job);
  }

  async function onDrop(e, targetJob) {
    e.preventDefault();
    cleanupDragPreview();
    if (!draggingJob || draggingJob.id === targetJob.id) { draggingJob = null; return; }
    if (draggingJob.status !== 'pending' || targetJob.status !== 'pending') { draggingJob = null; return; }

    let newPriority = dragPreviewPriority;
    if (newPriority === null || newPriority === undefined) {
      const rect = e.currentTarget?.getBoundingClientRect?.();
      const midpoint = rect ? rect.top + rect.height / 2 : e.clientY;
      const baseTargetPriority = getJobPriority(targetJob);
      const targetPriority = Number.isFinite(baseTargetPriority) ? baseTargetPriority : 0;
      newPriority = e.clientY < midpoint ? targetPriority + 1 : targetPriority - 1;
    }
    if (newPriority === null || newPriority === undefined) { draggingJob = null; return; }
    if (newPriority === draggingJob.priority) { draggingJob = null; return; }
    optimisticPriority = { id: draggingJob.id, priority: newPriority };
    const draggedId = draggingJob.id;
    draggingJob = null;
    clearDragOver();
    try {
      await api.reorderJobs([{ id: draggedId, priority: newPriority }]);
      pushToast('Job order updated.', 'success');
    } catch (error) {
      pushToast(formatApiError(error, 'Failed to update job order.'), 'error');
    } finally {
      await onRefresh?.();
      optimisticPriority = null;
    }
  }

  function onDragEnd() {
    cleanupDragPreview();
    clearDragOver();
    draggingJob = null;
  }

  async function runJob(job, e) {
    e.stopPropagation();
    try {
      if (job.status === 'running') {
        await api.stopJob(job.id);
        pushToast(`Job #${job.id} stopped.`, 'success');
      } else {
        if (job.status === 'pending' && isPendingRound) return;
        await api.enqueueExistingJob(job.id);
        pushToast(`Job #${job.id} enqueued.`, 'success');
      }
    } catch (error) {
      const fallback = job.status === 'running'
        ? `Failed to stop job #${job.id}.`
        : `Failed to enqueue job #${job.id}.`
      pushToast(formatApiError(error, fallback), 'error');
    }
  }

  function jobActionTitle(job) {
    if (job.status === 'running') return 'Stop now';
    if (job.status === 'pending') {
      return isPendingRound ? 'Start the round to enqueue' : 'Enqueue now';
    }
    return 'Re-run';
  }
</script>

<div class="rounds-panel">
  <div class="controls">
    <button onclick={handleNewRoundClick} disabled={newRoundCooldown}>New Round</button>
    <select
      value={selectedRoundId ?? ''}
      onchange={(e) => onSelectRound(e.target.value ? Number(e.target.value) : null)}
    >
      <option value="">Select round</option>
      {#each rounds as r}
        <option value={r.id}>
          Round {r.id} ({r.status}{r.jobs_ready === false ? '*' : ''})
        </option>
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
  <div class="controls">
    <input type="text" bind:value={searchQuery} placeholder="Search..." class="search-input" />
    <select bind:value={challengeFilterId}>
      <option value="">All challenges</option>
      {#each challenges as c}
        <option value={c.id}>{c.name}</option>
      {/each}
    </select>
    <select bind:value={teamFilterId}>
      <option value="">All teams</option>
      {#each teams as t}
        <option value={t.id}>{t.team_name}</option>
      {/each}
    </select>
    {#if availableStatuses.length}
      <select bind:value={statusFilter}>
        <option value="">All statuses</option>
        {#each availableStatuses as entry}
          <option value={entry}>{entry}</option>
        {/each}
      </select>
    {/if}
    {#if availableReasons.length}
      <select bind:value={reasonFilter}>
        <option value="">All reasons</option>
        {#each availableReasons as entry}
          <option value={entry}>{entry}</option>
        {/each}
      </select>
    {/if}
    <button
      class="small"
      type="button"
      onclick={() => {
        challengeFilterId = '';
        teamFilterId = '';
        statusFilter = '';
        reasonFilter = '';
        searchQuery = '';
      }}
      disabled={!canResetFilters}
    >
      Reset Filters
    </button>
  </div>

  {#if isJobsCreating}
    <p class="muted">Creating jobs for this round...</p>
  {/if}

  {#if filteredJobs.length}
    <table>
      <thead>
        <tr>
          <th>ID</th>
          <th>Challenge</th>
          <th>Exploit</th>
          <th>Team</th>
          <th>Create Reason</th>
          <th>Priority</th>
          <th>Status</th>
          <th>Duration</th>
          <th></th>
        </tr>
      </thead>
      <tbody>
        {#each getDisplayedJobs(filteredJobs) as j}
          <tr 
            class={j.status} 
            class:draggable={j.status === 'pending'}
            class:dragging={draggingJob?.id === j.id}
            draggable={j.status === 'pending'}
            ondragstart={(e) => onDragStart(e, j)}
            ondragover={(e) => onRowDragOver(e, j)}
            ondrop={(e) => onDrop(e, j)}
            ondragend={onDragEnd}
            onclick={() => openJob(j)} 
            style="cursor:pointer"
          >
            <td>{@html highlight(String(j.id))}</td>
            <td>{@html highlight(getChallengeName(challenges, getExploitRunInfo(j.exploit_run_id)?.challenge_id))}</td>
            <td>{@html highlight(getExploitName(exploits, getExploitRunInfo(j.exploit_run_id)?.exploit_id))}</td>
            <td><span class="truncate">{getTeamDisplay(teams, j.team_id)}</span></td>
            <td><span class="truncate">{@html highlight(j.create_reason || '-')}</span></td>
            <td>{j.priority}</td>
            <td>{formatStatus(j.status)}</td>
            <td>{j.duration_ms ? `${j.duration_ms}ms` : '-'}</td>
            <td>
              <button
                class={`play-btn ${j.status === 'running' ? 'stop' : ''}`}
                onclick={(e) => runJob(j, e)}
                title={jobActionTitle(j)}
                disabled={j.status === 'pending' && isPendingRound}
                aria-disabled={j.status === 'pending' && isPendingRound}
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
      <span class={statusClass(modalJob.status)}>{formatStatus(modalJob.status)}</span>
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
  .play-btn:disabled { cursor: not-allowed; opacity: 0.3; }
  .play-btn.stop { color: #ff6b6b; }
  .job-modal-header { display: flex; align-items: center; justify-content: space-between; gap: 1rem; }
  :global(.drag-preview) { pointer-events: none; box-shadow: 0 10px 24px rgba(0, 0, 0, 0.35); transform: rotate(-1deg); }
  :global(.drag-preview .play-btn) { display: none; }
  .search-input { width: 150px; }
</style>
