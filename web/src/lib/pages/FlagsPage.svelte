<script>
  let { rounds, flags, teams, challenges, selectedFlagRoundId, onSelectFlagRound } = $props();

  let challengeFilterId = $state(null);
  let teamFilterId = $state(null);

  function getTeamName(teamId) {
    const t = teams.find((t) => t.id === teamId);
    return t ? `${t.id} (${t.team_name})` : teamId;
  }

  function getChallengeName(challengeId) {
    const c = challenges.find((c) => c.id === challengeId);
    return c ? c.name : challengeId;
  }

  let filteredFlags = $derived(() => {
    return flags.filter((flag) => {
      if (teamFilterId && flag.team_id !== teamFilterId) return false;
      if (challengeFilterId && flag.challenge_id !== challengeFilterId) return false;
      return true;
    });
  });
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
  <select
    value={challengeFilterId ?? ''}
    onchange={(e) => (challengeFilterId = e.target.value ? Number(e.target.value) : null)}
  >
    <option value="">All challenges</option>
    {#each challenges as c}
      <option value={c.id}>{c.name}</option>
    {/each}
  </select>
  <select
    value={teamFilterId ?? ''}
    onchange={(e) => (teamFilterId = e.target.value ? Number(e.target.value) : null)}
  >
    <option value="">All teams</option>
    {#each teams as t}
      <option value={t.id}>{t.team_name}</option>
    {/each}
  </select>
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
