<script>
  import FilterBar from '$lib/ui/FilterBar.svelte';
  import { buildStatusOptions } from '$lib/utils/filters.js';
  import { getChallengeName, getTeamName } from '$lib/utils/lookup.js';

  let { rounds, flags, teams, challenges, selectedFlagRoundId, onSelectFlagRound } = $props();

  let challengeFilterId = $state('');
  let teamFilterId = $state('');
  let statusFilter = $state('');

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

  let filteredFlags = $derived(filterFlags());
  let availableStatuses = $derived(buildStatusOptions(flags));
</script>

<div class="controls">
  <select
    value={selectedFlagRoundId ?? ''}
    onchange={(e) => onSelectFlagRound(e.target.value ? Number(e.target.value) : null)}
  >
    <option value="">All rounds</option>
    {#each rounds as r}
      <option value={r.id}>Round {r.id}</option>
    {/each}
  </select>
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

<table>
  <thead>
    <tr>
      <th>ID</th>
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
        <td>{f.round_id}</td>
        <td>{getChallengeName(challenges, f.challenge_id)}</td>
        <td>{getTeamName(teams, f.team_id)}</td>
        <td><code>{f.flag_value}</code></td>
        <td>{f.status}</td>
      </tr>
    {/each}
  </tbody>
</table>
