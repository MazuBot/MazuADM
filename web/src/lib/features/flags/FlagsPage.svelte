<script>
  import { buildStatusOptions } from '$lib/utils/filters.js';
  import { getChallengeName, getTeamDisplay } from '$lib/utils/lookup.js';
  import { formatApiError, pushToast } from '$lib/ui/toastStore.js';

  let {
    rounds,
    flags,
    teams,
    challenges,
    settings,
    selectedFlagRoundId,
    onSelectFlagRound,
    onSubmitFlag
  } = $props();

  let challengeFilterId = $state('');
  let teamFilterId = $state('');
  let statusFilter = $state('');
  let submitRoundId = $state('');
  let submitChallengeId = $state('');
  let submitTeamId = $state('');
  let submitFlagValue = $state('');
  let submitting = $state(false);

  function getSetting(key, fallback) {
    return settings?.find((s) => s.key === key)?.value || fallback;
  }

  $effect(() => {
    if (submitRoundId === '' && selectedFlagRoundId) {
      submitRoundId = String(selectedFlagRoundId);
    }
  });

  function parsePastFlagRounds() {
    const raw = getSetting('past_flag_rounds', '5');
    const parsed = Number.parseInt(raw, 10);
    if (!Number.isFinite(parsed) || parsed < 0) return 0;
    return parsed;
  }

  function buildAllowedRounds() {
    if (!runningRound) return [];
    const minId = runningRound.id - pastFlagRounds;
    return rounds.filter((r) => r.id <= runningRound.id && r.id >= minId);
  }

  function filterFlags() {
    const teamId = teamFilterId ? Number(teamFilterId) : null;
    const challengeId = challengeFilterId ? Number(challengeFilterId) : null;
    return flags.filter((flag) => {
      if (statusFilter && flag.status !== statusFilter) return false;
      if (teamId && Number(flag.team_id) !== teamId) return false;
      if (challengeId && Number(flag.challenge_id) !== challengeId) return false;
      return true;
    });
  }

  async function handleSubmitFlag() {
    if (!onSubmitFlag) return;
    const flagValue = submitFlagValue.trim();
    if (!submitChallengeId || !submitTeamId || !flagValue) {
      pushToast('Select a challenge, team, and flag value.', 'error');
      return;
    }
    submitting = true;
    try {
      await onSubmitFlag({
        round_id: submitRoundId ? Number(submitRoundId) : null,
        challenge_id: Number(submitChallengeId),
        team_id: Number(submitTeamId),
        flag_value: flagValue
      });
      submitFlagValue = '';
      pushToast('Flag submitted.', 'success');
    } catch (error) {
      pushToast(formatApiError(error, 'Failed to submit flag.'), 'error');
    } finally {
      submitting = false;
    }
  }

  let filteredFlags = $derived(filterFlags());
  let availableStatuses = $derived(buildStatusOptions(flags));
  let canResetFilters = $derived(Boolean(challengeFilterId || teamFilterId || statusFilter));
  let pastFlagRounds = $derived(parsePastFlagRounds());
  let runningRound = $derived(rounds.find((r) => r.status === 'running'));
  let allowedRounds = $derived(buildAllowedRounds());
  let hasRunningRound = $derived(Boolean(runningRound));
  let canSubmit = $derived(
    !submitting &&
      hasRunningRound &&
      submitChallengeId !== '' &&
      submitTeamId !== '' &&
      submitFlagValue.trim().length > 0
  );

  $effect(() => {
    if (!submitRoundId || allowedRounds.length === 0) return;
    if (!allowedRounds.some((r) => String(r.id) === String(submitRoundId))) {
      submitRoundId = '';
    }
  });
</script>

<div class="panel">
  <div class="panel-header">
    <h2>Manual Flag Submission</h2>
  </div>
  <div class="controls">
    <select bind:value={submitRoundId} aria-label="Round for manual flag">
      <option value="">Running round</option>
      {#each allowedRounds as r}
        <option value={String(r.id)}>Round {r.id} ({r.status})</option>
      {/each}
    </select>
    <select bind:value={submitChallengeId} aria-label="Challenge for manual flag">
      <option value="">Select challenge</option>
      {#each challenges as c}
        <option value={c.id}>{c.name}</option>
      {/each}
    </select>
    <select bind:value={submitTeamId} aria-label="Team for manual flag">
      <option value="">Select team</option>
      {#each teams as t}
        <option value={t.id}>{getTeamDisplay(teams, t.id)}</option>
      {/each}
    </select>
    <input
      type="text"
      bind:value={submitFlagValue}
      placeholder={'FLAG{...}'}
      maxlength="512"
      aria-label="Flag value"
    />
    <button onclick={handleSubmitFlag} disabled={!canSubmit}>
      {submitting ? 'Submitting...' : 'Submit Flag'}
    </button>
  </div>
  <p class="hint">Leave round empty to target the running round.</p>
  {#if !hasRunningRound}
    <p class="hint">Manual submission requires a running round.</p>
  {/if}
</div>

<div class="controls flag-filters">
  <select
    value={selectedFlagRoundId ?? ''}
    onchange={(e) => onSelectFlagRound(e.target.value ? Number(e.target.value) : null)}
  >
    <option value="">All rounds</option>
    {#each rounds as r}
      <option value={r.id}>Round {r.id}</option>
    {/each}
  </select>
  <select bind:value={challengeFilterId}>
    <option value="">All challenges</option>
    {#each challenges as c}
      <option value={c.id}>{c.name}</option>
    {/each}
  </select>
  <select bind:value={teamFilterId}>
    <option value="">All teams</option>
    {#each teams as t}
      <option value={t.id}>{getTeamDisplay(teams, t.id)}</option>
    {/each}
  </select>
  <select bind:value={statusFilter}>
    <option value="">All statuses</option>
    {#each availableStatuses as entry}
      <option value={entry}>{entry}</option>
    {/each}
  </select>
  <button class="small" type="button" onclick={() => {
    challengeFilterId = '';
    teamFilterId = '';
    statusFilter = '';
  }} disabled={!canResetFilters}>
    Reset Filters
  </button>
</div>

<table>
  <thead>
    <tr>
      <th>ID</th>
      <th>Job</th>
      <th>Round</th>
      <th>Challenge</th>
      <th>Team</th>
      <th>Flag</th>
      <th>Status</th>
    </tr>
  </thead>
  <tbody>
    {#each filteredFlags as f}
      <tr>
        <td>{f.id}</td>
        <td>{f.job_id ?? '-'}</td>
        <td>{f.round_id}</td>
        <td>{getChallengeName(challenges, f.challenge_id)}</td>
        <td><span class="truncate">{getTeamDisplay(teams, f.team_id)}</span></td>
        <td><code>{f.flag_value}</code></td>
        <td>{f.status}</td>
      </tr>
    {/each}
  </tbody>
</table>

<style>
  .flag-filters {
    margin-top: 0.75rem;
  }
</style>
