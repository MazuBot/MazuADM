<script>
  let {
    challengeId = $bindable(''),
    teamId = $bindable(''),
    status = $bindable(''),
    reason = $bindable(''),
    search = $bindable(''),
    challenges = [],
    teams = [],
    statuses = [],
    reasons = [],
    onReset = () => {}
  } = $props();

  let showStatus = $derived((statuses ?? []).length > 0);
  let showReason = $derived((reasons ?? []).length > 0);
  let canReset = $derived(Boolean(challengeId || teamId || status || reason || search));
</script>

<div class="controls">
  <input type="text" bind:value={search} placeholder="Search..." class="search-input" />
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
  {#if showReason}
    <select bind:value={reason}>
      <option value="">All reasons</option>
      {#each reasons as entry}
        <option value={entry}>{entry}</option>
      {/each}
    </select>
  {/if}
  <button class="small" type="button" onclick={onReset} disabled={!canReset}>
    Reset Filters
  </button>
</div>

<style>
  .search-input {
    width: 150px;
  }
</style>
