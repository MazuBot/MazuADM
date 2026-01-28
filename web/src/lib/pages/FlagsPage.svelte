<script>
  let { rounds, flags, teams, challenges, selectedFlagRoundId, onSelectFlagRound } = $props();

  let challengeFilterId = $state('');
  let teamFilterId = $state('');
  let statusFilter = $state('');

  function getTeamName(teamId) {
    const t = teams.find((t) => t.id === teamId);
    return t ? `${t.id} (${t.team_name})` : teamId;
  }

  function getChallengeName(challengeId) {
    const c = challenges.find((c) => c.id === challengeId);
    return c ? c.name : challengeId;
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

  let filteredFlags = $derived(filterFlags());
  let availableStatuses = $derived([...new Set((flags ?? []).map((flag) => flag.status).filter(Boolean))].sort());
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
        <td>{getChallengeName(f.challenge_id)}</td>
        <td>{getTeamName(f.team_id)}</td>
        <td><code>{f.flag_value}</code></td>
        <td>{f.status}</td>
      </tr>
    {/each}
  </tbody>
</table>
