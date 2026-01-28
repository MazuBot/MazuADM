<script>
  import * as api from '$lib/data/api';
  import { getChallengeName, getExploitName, getExploitRunName, getTeamDisplay } from '$lib/utils/lookup.js';

  let { exploits, exploitRuns, challenges, teams, containers, containerRunners, onLoadContainers, onLoadRunners } = $props();

  let hasContainers = $derived((containers ?? []).length > 0);

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
      <h3>{getChallengeName(challenges, exploit.challenge_id)} / {exploit.name}</h3>
      <table class="containers-table">
        <colgroup>
          <col style="width: 18%" />
          <col style="width: 12%" />
          <col style="width: 10%" />
          <col style="width: 10%" />
          <col style="width: 30%" />
          <col style="width: 20%" />
        </colgroup>
        <thead>
          <tr>
            <th>Container</th>
            <th>Status</th>
            <th>Counter</th>
            <th>Execs</th>
            <th>Jobs</th>
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
                {#if containerRunners[c.id]}
                  {#each containerRunners[c.id] as r}
                    <div>
                      {#if r.exploit_run_id}
                        {getExploitRunName(exploitRuns, exploits, r.exploit_run_id)}
                      {:else}
                        Ad-hoc
                      {/if}
                      → <span class="truncate">{getTeamDisplay(teams, r.team_id)}</span> ({r.status})
                    </div>
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
