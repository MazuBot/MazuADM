<script>
  let {
    challengeId = $bindable(''),
    teamId = $bindable(''),
    status = $bindable(''),
    challenges = [],
    teams = [],
    statuses = [],
    onReset = () => {}
  } = $props();

  let showStatus = $derived((statuses ?? []).length > 0);
  let canReset = $derived(Boolean(challengeId || teamId || status));
</script>

<div class="controls">
  <select bind:value={challengeId}>
    <option value="">All challenges</option>
    {#each challenges as c}
      <option value={c.id}>{c.name}</option>
    {/each}
  </select>
  <select bind:value={teamId}>
    <option value="">All teams</option>
    {#each teams as t}
      <option value={t.id}>{t.team_name}</option>
    {/each}
  </select>
  {#if showStatus}
    <select bind:value={status}>
      <option value="">All statuses</option>
      {#each statuses as entry}
        <option value={entry}>{entry}</option>
      {/each}
    </select>
  {/if}
  <button class="small" type="button" onclick={onReset} disabled={!canReset}>
    Reset Filters
  </button>
</div>
