<script>
  import * as api from '$lib/data/api';
  import { getChallengeName, getTeamDisplay } from '$lib/utils/lookup.js';
  import { formatApiError, pushToast } from '$lib/ui/toastStore.js';

  let { exploits, exploitRuns, challenges, teams, containers, selectedChallengeId, onSelectChallenge, onLoadContainers } = $props();

  let hasContainers = $derived((containers ?? []).length > 0);
  let filteredExploits = $derived(
    selectedChallengeId ? exploits.filter((exploit) => exploit.challenge_id === selectedChallengeId) : exploits
  );
  let restarting = $state(new Set());

  function markRestarting(ids) {
    const next = new Set(restarting);
    for (const id of ids) {
      next.add(id);
    }
    restarting = next;
  }

  function clearRestarting(ids) {
    const next = new Set(restarting);
    for (const id of ids) {
      next.delete(id);
    }
    restarting = next;
  }

  async function restartContainer(id) {
    markRestarting([id]);
    const shortId = id.slice(0, 12);
    try {
      await api.restartContainer(id);
      await onLoadContainers();
      pushToast(`Container restarted: ${shortId}.`, 'success');
    } catch (error) {
      pushToast(formatApiError(error, `Failed to restart container ${shortId}.`), 'error');
    } finally {
      clearRestarting([id]);
    }
  }

  async function deleteContainer(id) {
    const shortId = id.slice(0, 12);
    try {
      await api.deleteContainer(id);
      await onLoadContainers();
      pushToast(`Container removed: ${shortId}.`, 'success');
    } catch (error) {
      pushToast(formatApiError(error, `Failed to remove container ${shortId}.`), 'error');
    }
  }

  async function reloadContainers() {
    await onLoadContainers();
  }

  async function restartAllContainers() {
    if (!containers?.length) return;
    if (!confirm(`Restart all ${containers.length} containers?`)) return;
    const ids = containers.map((c) => c.id);
    markRestarting(ids);
    try {
      const result = await api.restartAllContainers();
      await onLoadContainers();
      const total = result?.total ?? containers.length;
      const success = result?.success ?? 0;
      const failed = result?.failed ?? 0;
      const message = failed > 0
        ? `Restarted ${success}/${total} containers (${failed} failed).`
        : `Restarted ${success} containers.`;
      pushToast(message, failed > 0 ? 'error' : 'success');
    } catch (error) {
      pushToast(formatApiError(error, 'Failed to restart all containers.'), 'error');
    } finally {
      clearRestarting(ids);
    }
  }

  async function removeAllContainers() {
    if (!containers?.length) return;
    if (!confirm(`Remove all ${containers.length} containers?`)) return;
    try {
      const result = await api.removeAllContainers();
      await onLoadContainers();
      const total = result?.total ?? containers.length;
      const success = result?.success ?? 0;
      const failed = result?.failed ?? 0;
      const message = failed > 0
        ? `Removed ${success}/${total} containers (${failed} failed).`
        : `Removed ${success} containers.`;
      pushToast(message, failed > 0 ? 'error' : 'success');
    } catch (error) {
      pushToast(formatApiError(error, 'Failed to remove all containers.'), 'error');
    }
  }
</script>

{#if challenges.length}
  <div class="challenge-tabs">
    <button class:active={selectedChallengeId === null} onclick={() => onSelectChallenge(null)}>
      All
    </button>
    {#each challenges as c}
      <button class:active={selectedChallengeId === c.id} onclick={() => onSelectChallenge(c.id)}>
        {c.name}
      </button>
    {/each}
  </div>
{/if}

<div class="panel">
  <div class="panel-header">
    <h2>Containers</h2>
    <div class="panel-actions">
      <button class="small" onclick={reloadContainers}>Reload</button>
      <button class="small" onclick={restartAllContainers} disabled={!hasContainers || restarting.size > 0}>Restart All</button>
      <button class="small danger" onclick={removeAllContainers} disabled={!hasContainers}>Remove All</button>
    </div>
  </div>
  {#each filteredExploits as exploit}
    {@const expContainers = containers.filter((c) => c.exploit_id === exploit.id)}
    {#if expContainers.length}
      <h3>{getChallengeName(challenges, exploit.challenge_id)} / {exploit.name}</h3>
      <table class="containers-table">
        <colgroup>
          <col style="width: 16%" />
          <col style="width: 10%" />
          <col style="width: 8%" />
          <col style="width: 10%" />
          <col style="width: 40%" />
          <col style="width: 16%" />
        </colgroup>
        <thead>
          <tr>
            <th>Container</th>
            <th>Status</th>
            <th>Counter</th>
            <th>Execs</th>
            <th>Affinity</th>
            <th>Actions</th>
          </tr>
        </thead>
        <tbody>
          {#each expContainers as c}
            <tr class={c.status === 'running' ? '' : 'error'}>
              <td><code>{c.id.slice(0, 12)}</code></td>
              <td>{c.status}</td>
              <td>{c.counter}</td>
              <td>{c.running_execs}/{c.max_execs}</td>
              <td class="runners-cell">
                {#if c.affinity_runs?.length}
                  {#each c.affinity_runs as runId}
                    {@const run = exploitRuns.find((r) => r.id === runId)}
                    <div>
                      {#if run}
                        {getTeamDisplay(teams, run.team_id)}
                      {:else}
                        {runId}
                      {/if}
                    </div>
                  {/each}
                {:else}
                  <span class="muted">-</span>
                {/if}
              </td>
              <td>
                <button onclick={() => restartContainer(c.id)} disabled={restarting.has(c.id)}>Restart</button>
                <button onclick={() => deleteContainer(c.id)}>Remove</button>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  {/each}
</div>
