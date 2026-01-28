<script>
  import { AnsiUp } from 'ansi_up';

  let { rounds, jobs, teams, challenges, exploits, exploitRuns, selectedRoundId, onSelectRound, onNewRound, onRunRound } = $props();

  const ansi_up = new AnsiUp();
  function renderAnsi(text) {
    return ansi_up.ansi_to_html(text || '');
  }

  let selectedJob = $state(null);

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
    <button onclick={() => selectedRoundId && onRunRound(selectedRoundId)} disabled={!selectedRoundId}>Run</button>
  </div>

  {#if jobs.length}
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
        </tr>
      </thead>
      <tbody>
        {#each [...jobs].sort((a, b) => b.priority - a.priority || a.id - b.id) as j}
          <tr class={j.status} onclick={() => (selectedJob = j)} style="cursor:pointer">
            <td>{j.id}</td>
            <td>{getChallengeName(getExploitRunInfo(j.exploit_run_id)?.challenge_id)}</td>
            <td>{getExploitName(getExploitRunInfo(j.exploit_run_id)?.exploit_id)}</td>
            <td>{getTeamName(j.team_id)}</td>
            <td>{j.container_id ? j.container_id.slice(0, 12) : '-'}</td>
            <td>{j.priority}</td>
            <td>{j.status === 'flag' ? '🚩 FLAG' : j.status}</td>
            <td>{j.duration_ms ? `${j.duration_ms}ms` : '-'}</td>
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
