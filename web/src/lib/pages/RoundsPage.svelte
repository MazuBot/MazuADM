<script>
  import { AnsiUp } from 'ansi_up';
  import { api } from '$lib/api.js';

  let { rounds, jobs, teams, challenges, exploits, exploitRuns, selectedRoundId, onSelectRound, onNewRound, onRunRound, onRefresh } = $props();

  const ansi_up = new AnsiUp();
  function renderAnsi(text) {
    return ansi_up.ansi_to_html(text || '');
  }

  let selectedJob = $state(null);
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

  function getTeamName(teamId) {
    const t = teams.find((t) => t.id === teamId);
    return t ? `${t.id} (${t.team_name})` : teamId;
  }

  function getChallengeName(challengeId) {
    const c = challenges.find((c) => c.id === challengeId);
    return c ? c.name : challengeId;
  }

  function getExploitName(exploitId) {
    const e = exploits.find((e) => e.id === exploitId);
    return e ? e.name : exploitId;
  }

  function getExploitRunInfo(runId) {
    return exploitRuns.find((r) => r.id === runId);
  }

  function closeModal() {
    selectedJob = null;
  }

  function onOverlayClick(e) {
    if (e.target === e.currentTarget) closeModal();
  }

  function onOverlayKeydown(e) {
    if (e.key === 'Escape') closeModal();
  }

  function sortedJobs(list = jobs) {
    return [...list].sort((a, b) => b.priority - a.priority || a.id - b.id);
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

  function statusOptions() {
    return [...new Set((jobs ?? []).map((job) => job.status).filter(Boolean))].sort();
  }

  let filteredJobs = $derived(filterJobs());
  let availableStatuses = $derived(statusOptions());

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
    await api.runExistingJob(job.id);
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
    <select bind:value={statusFilter}>
      <option value="">All statuses</option>
      {#each availableStatuses as status}
        <option value={status}>{status}</option>
      {/each}
    </select>
    <button
      class="small"
      type="button"
      onclick={() => {
        challengeFilterId = '';
        teamFilterId = '';
        statusFilter = '';
      }}
      disabled={!challengeFilterId && !teamFilterId && !statusFilter}
    >
      Reset Filters
    </button>
  </div>

  {#if filteredJobs.length}
    <table>
      <thead>
        <tr>
          <th>ID</th>
          <th>Challenge</th>
          <th>Exploit</th>
          <th>Team</th>
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
            onclick={() => (selectedJob = j)} 
            style="cursor:pointer"
          >
            <td>{j.id}</td>
            <td>{getChallengeName(getExploitRunInfo(j.exploit_run_id)?.challenge_id)}</td>
            <td>{getExploitName(getExploitRunInfo(j.exploit_run_id)?.exploit_id)}</td>
            <td>{getTeamName(j.team_id)}</td>
            <td>{j.container_id ? j.container_id.slice(0, 12) : '-'}</td>
            <td>{j.priority}</td>
            <td>{j.status === 'flag' ? '🚩 FLAG' : j.status}</td>
            <td>{j.duration_ms ? `${j.duration_ms}ms` : '-'}</td>
            <td><button class="play-btn" onclick={(e) => runJob(j, e)} title="Run now">▶</button></td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</div>

{#if selectedJob}
  <div
    class="modal-overlay"
    role="button"
    tabindex="0"
    aria-label="Close modal"
    onclick={onOverlayClick}
    onkeydown={onOverlayKeydown}
  >
    <div class="modal wide" role="dialog" aria-modal="true">
      <h3>Job #{selectedJob.id} - {selectedJob.status}</h3>
      <div class="job-info">
        <p><strong>Exploit:</strong> {getExploitName(getExploitRunInfo(selectedJob.exploit_run_id)?.exploit_id)}</p>
        <p><strong>Team:</strong> {getTeamName(selectedJob.team_id)}</p>
        <p><strong>Priority:</strong> {selectedJob.priority}</p>
        <p><strong>Duration:</strong> {selectedJob.duration_ms ? `${selectedJob.duration_ms}ms` : '-'}</p>
        {#if selectedJob.container_id}<p><strong>Container:</strong> <code>{selectedJob.container_id.slice(0, 12)}</code></p>{/if}
      </div>
      {#if selectedJob.stdout}
        <div class="modal-section-label">Stdout</div>
        <pre class="log-output">{@html renderAnsi(selectedJob.stdout)}</pre>
      {/if}
      {#if selectedJob.stderr}
        <div class="modal-section-label">Stderr</div>
        <pre class="log-output stderr">{@html renderAnsi(selectedJob.stderr)}</pre>
      {/if}
      <div class="modal-actions">
        <button onclick={closeModal}>Close</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .draggable { cursor: grab; }
  .dragging { opacity: 0.4; background: #333; }
  .play-btn { background: transparent; border: none; cursor: pointer; font-size: 0.9rem; padding: 0.2rem 0.4rem; opacity: 0.6; color: white; }
  .play-btn:hover { opacity: 1; }
</style>
