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
  let statusFilters = $state(new Set());
  let searchQuery = $state('');
  let submitRoundId = $state('');
  let submitChallengeId = $state('');
  let submitTeamId = $state('');
  let submitFlagValue = $state('');
  let submitting = $state(false);
  let roundSelectionTouched = $state(false);

  function getSetting(key, fallback) {
    return settings?.find((s) => s.key === key)?.value || fallback;
  }

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
    const query = searchQuery.trim().toLowerCase();
    return flags.filter((flag) => {
      if (statusFilters.size > 0 && !statusFilters.has(flag.status)) return false;
      if (teamId && Number(flag.team_id) !== teamId) return false;
      if (challengeId && Number(flag.challenge_id) !== challengeId) return false;
      if (query) {
        const jobMatch = flag.job_id != null && String(flag.job_id).includes(query);
        const flagMatch = flag.flag_value.toLowerCase().includes(query);
        if (!jobMatch && !flagMatch) return false;
      }
      return true;
    });
  }

  function highlight(text) {
    if (!text) return '-';
    const q = searchQuery.trim();
    if (q.length < 2) return text;
    const regex = new RegExp(`(${q.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')})`, 'gi');
    return text.replace(regex, '<mark>$1</mark>');
  }

  function resolveChallengeId() {
    if (submitChallengeId) return Number(submitChallengeId);
    if (challengeFilterId) return Number(challengeFilterId);
    return challenges?.[0]?.id ?? null;
  }

  function resolveTeamId() {
    if (submitTeamId) return Number(submitTeamId);
    if (teamFilterId) return Number(teamFilterId);
    return teams?.[0]?.id ?? null;
  }

  async function handleSubmitFlag() {
    if (!onSubmitFlag) return;
    const flagValue = submitFlagValue.trim();
    if (!flagValue) {
      pushToast('Enter a flag value.', 'error');
      return;
    }
    const challengeId = resolveChallengeId();
    const teamId = resolveTeamId();
    if (!challengeId || !teamId) {
      pushToast('Select a challenge and team, or set filters to choose defaults.', 'error');
      return;
    }
    submitting = true;
    try {
      await onSubmitFlag({
        round_id: submitRoundId ? Number(submitRoundId) : null,
        challenge_id: challengeId,
        team_id: teamId,
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
  let canResetFilters = $derived(Boolean(challengeFilterId || teamFilterId || statusFilters.size > 0 || searchQuery));
  let pastFlagRounds = $derived(parsePastFlagRounds());
  let runningRound = $derived(rounds.find((r) => r.status === 'running'));
  let allowedRounds = $derived(buildAllowedRounds());
  let hasRunningRound = $derived(Boolean(runningRound));
  let canSubmit = $derived(
    !submitting &&
      hasRunningRound &&
      challenges.length > 0 &&
      teams.length > 0 &&
      submitFlagValue.trim().length > 0
  );

  $effect(() => {
    if (roundSelectionTouched) return;
    if (runningRound) {
      const next = String(runningRound.id);
      if (submitRoundId !== next) submitRoundId = next;
    } else if (submitRoundId !== '') {
      submitRoundId = '';
    }
  });

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
    <select
      bind:value={submitRoundId}
      aria-label="Round for manual flag"
      onchange={(e) => {
        roundSelectionTouched = true;
        if (!e.currentTarget?.value) {
          submitRoundId = '';
        }
      }}
    >
      <option value="">Unset</option>
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
  <p class="hint">Round defaults to running; choose blank to leave it unset.</p>
  {#if !hasRunningRound}
    <p class="hint">Manual submission requires a running round.</p>
  {/if}
</div>

<div class="controls flag-filters">
  <input type="text" bind:value={searchQuery} placeholder="Search job/flag..." />
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
  <div class="status-filters">
    {#each availableStatuses as status}
      <label class="status-checkbox">
        <input
          type="checkbox"
          checked={statusFilters.has(status)}
          onchange={() => {
            const next = new Set(statusFilters);
            if (next.has(status)) next.delete(status);
            else next.add(status);
            statusFilters = next;
          }}
        />
        {status}
      </label>
    {/each}
  </div>
  <button class="small" type="button" onclick={() => {
    challengeFilterId = '';
    teamFilterId = '';
    statusFilters = new Set();
    searchQuery = '';
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
      <tr class:row-success={f.status === 'success'}>
        <td>{f.id}</td>
        <td>{@html highlight(String(f.job_id ?? '-'))}</td>
        <td>{f.round_id}</td>
        <td>{getChallengeName(challenges, f.challenge_id)}</td>
        <td><span class="truncate">{getTeamDisplay(teams, f.team_id)}</span></td>
        <td><code>{@html highlight(f.flag_value)}</code></td>
        <td>{f.status}</td>
      </tr>
    {/each}
  </tbody>
</table>

<style>
  .flag-filters {
    margin-top: 0.75rem;
  }
  .status-filters {
    display: flex;
    gap: 0.75rem;
    align-items: center;
  }
  .status-checkbox {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    cursor: pointer;
  }
  .row-success {
    background-color: rgba(34, 197, 94, 0.15);
  }
</style>
