<script>
  import { api } from '$lib/api.js';

  let { exploits, exploitRuns, challenges, teams, containers, containerRunners, onLoadContainers, onLoadRunners } = $props();

  let hasContainers = $derived((containers ?? []).length > 0);

  function getTeamName(teamId) {
    const t = teams.find((t) => t.id === teamId);
    return t ? `${t.id} (${t.team_name})` : teamId;
  }

  function getExploitName(exploitId) {
    const e = exploits.find((e) => e.id === exploitId);
    return e ? e.name : exploitId;
  }

  function getChallengeName(challengeId) {
    const c = challenges.find((c) => c.id === challengeId);
    return c ? c.name : (challengeId ?? '-');
  }

  function getExploitRunName(runId) {
    const run = exploitRuns.find((r) => r.id === runId);
    return run ? getExploitName(run.exploit_id) : runId;
  }

  async function restartContainer(id) {
    await api.restartContainer(id);
    await onLoadContainers();
  }

  async function deleteContainer(id) {
    await api.deleteContainer(id);
    await onLoadContainers();
  }

  async function loadAllRunners() {
    if (!containers?.length) return;
    await Promise.all(containers.map((c) => onLoadRunners(c.id)));
  }

  async function restartAllContainers() {
    if (!containers?.length) return;
    if (!confirm(`Restart all ${containers.length} containers?`)) return;
    await Promise.all(containers.map((c) => api.restartContainer(c.id)));
    await onLoadContainers();
  }

  async function removeAllContainers() {
    if (!containers?.length) return;
    if (!confirm(`Remove all ${containers.length} containers?`)) return;
    await Promise.all(containers.map((c) => api.deleteContainer(c.id)));
    await onLoadContainers();
  }
</script>

<div class="panel">
  <div class="panel-header">
    <h2>Containers</h2>
    <div class="panel-actions">
      <button class="small" onclick={loadAllRunners} disabled={!hasContainers}>Load All</button>
      <button class="small" onclick={restartAllContainers} disabled={!hasContainers}>Restart All</button>
      <button class="small danger" onclick={removeAllContainers} disabled={!hasContainers}>Remove All</button>
    </div>
  </div>
  {#each exploits as exploit}
    {@const expContainers = containers.filter((c) => c.exploit_id === exploit.id)}
    {#if expContainers.length}
      <h3>{getChallengeName(exploit.challenge_id)} / {exploit.name}</h3>
      <table>
        <thead>
          <tr>
            <th>ID</th>
            <th>Container</th>
            <th>Status</th>
            <th>Counter</th>
            <th>Runners</th>
            <th>Actions</th>
          </tr>
        </thead>
        <tbody>
          {#each expContainers as c}
            <tr class={c.status === 'running' ? '' : 'error'}>
              <td>{c.id}</td>
              <td><code>{c.container_id.slice(0, 12)}</code></td>
              <td>{c.status}</td>
              <td>{c.counter}</td>
              <td>
                {#if containerRunners[c.id]}
                  {#each containerRunners[c.id] as r}
                    <div>{getExploitRunName(r.exploit_run_id)} → {getTeamName(r.team_id)}</div>
                  {/each}
                {:else}
                  <button onclick={() => onLoadRunners(c.id)}>Load</button>
                {/if}
              </td>
              <td>
                <button onclick={() => restartContainer(c.id)}>Restart</button>
                <button onclick={() => deleteContainer(c.id)}>Remove</button>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  {/each}
</div>
