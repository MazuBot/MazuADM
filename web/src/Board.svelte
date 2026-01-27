<script>
  import { api } from './api.js';

  let { teams, exploits, exploitRuns, challengeId, onRefresh } = $props();

  let showAddExploit = $state(false);
  let newExploit = $state({ name: '', docker_image: '', entrypoint: '', priority: 0, max_per_container: 1, default_counter: 999, auto_add: 'none' });

  let editingRun = $state(null);
  let editForm = $state({ priority: '', sequence: 0, enabled: true });

  let editingExploit = $state(null);
  let exploitForm = $state({ name: '', docker_image: '', entrypoint: '', priority: 0, max_per_container: 1, default_counter: 999, enabled: true });

  let draggingCard = $state(null);

  let filteredExploits = $derived(exploits.filter(e => e.challenge_id === challengeId));

  function getRunsForTeam(teamId) {
    return exploitRuns
      .filter(r => r.challenge_id === challengeId && r.team_id === teamId)
      .sort((a, b) => a.sequence - b.sequence);
  }

  function getExploitName(exploitId) {
    return exploits.find(e => e.id === exploitId)?.name || `Exploit ${exploitId}`;
  }

  function getExploit(exploitId) {
    return exploits.find(e => e.id === exploitId);
  }

  function getTeamName(teamId) {
    return teams.find(t => t.id === teamId)?.team_name || `Team ${teamId}`;
  }

  async function addExploit() {
    await api.createExploit({ 
      name: newExploit.name,
      docker_image: newExploit.docker_image,
      priority: newExploit.priority,
      challenge_id: challengeId,
      entrypoint: newExploit.entrypoint || null,
      max_per_container: newExploit.max_per_container,
      default_counter: newExploit.default_counter,
      auto_add: newExploit.auto_add
    });
    showAddExploit = false;
    newExploit = { name: '', docker_image: '', entrypoint: '', priority: 0, max_per_container: 1, default_counter: 999, auto_add: 'none' };
    onRefresh();
  }

  function openEditExploit(e) {
    editingExploit = e;
    exploitForm = { name: e.name, docker_image: e.docker_image, entrypoint: e.entrypoint || '', priority: e.priority, max_per_container: e.max_per_container, default_counter: e.default_counter, enabled: e.enabled };
  }

  async function saveExploit() {
    await api.updateExploit(editingExploit.id, {
      ...exploitForm,
      entrypoint: exploitForm.entrypoint || null
    });
    editingExploit = null;
    onRefresh();
  }

  async function deleteExploit() {
    if (confirm('Delete this exploit and all its runs?')) {
      await api.deleteExploit(editingExploit.id);
      editingExploit = null;
      onRefresh();
    }
  }

  async function addRun(exploitId, teamId) {
    const runs = getRunsForTeam(teamId);
    const maxSeq = runs.length > 0 ? Math.max(...runs.map(r => r.sequence)) : -1;
    await api.createExploitRun({ exploit_id: exploitId, challenge_id: challengeId, team_id: teamId, sequence: maxSeq + 1 });
    onRefresh();
  }

  function openEditModal(run, e) {
    e.stopPropagation();
    editingRun = run;
    editForm = { priority: run.priority ?? '', sequence: run.sequence, enabled: run.enabled };
  }

  async function saveRun() {
    await api.updateExploitRun(editingRun.id, {
      priority: editForm.priority === '' ? null : Number(editForm.priority),
      sequence: editForm.sequence,
      enabled: editForm.enabled
    });
    editingRun = null;
    onRefresh();
  }

  async function deleteRun() {
    if (confirm('Delete this exploit run?')) {
      await api.deleteExploitRun(editingRun.id);
      editingRun = null;
      onRefresh();
    }
  }

  function onCardDragStart(e, run) {
    draggingCard = run;
    e.dataTransfer.effectAllowed = 'move';
  }

  async function onCardDrop(e, targetRun, teamId) {
    e.preventDefault();
    if (!draggingCard || draggingCard.id === targetRun.id) return;
    if (draggingCard.team_id !== teamId) return;

    const runs = getRunsForTeam(teamId);
    const fromIdx = runs.findIndex(r => r.id === draggingCard.id);
    const toIdx = runs.findIndex(r => r.id === targetRun.id);

    // Reorder
    const reordered = [...runs];
    reordered.splice(fromIdx, 1);
    reordered.splice(toIdx, 0, draggingCard);

    // Batch update sequences
    const updates = reordered.map((r, i) => ({ id: r.id, sequence: i })).filter((u, i) => reordered[i].sequence !== u.sequence);
    if (updates.length > 0) {
      await api.reorderExploitRuns(updates);
    }
    draggingCard = null;
    onRefresh();
  }

  function onColumnDrop(e, teamId) {
    e.preventDefault();
    const exploitId = e.dataTransfer.getData('exploitId');
    if (exploitId) {
      addRun(+exploitId, teamId);
    }
    draggingCard = null;
  }
</script>

<div class="board">
  <div class="sidebar">
    <h3>Exploits</h3>
    {#each filteredExploits as e}
      <div class="exploit-item" class:disabled={!e.enabled} draggable="true" ondragstart={(ev) => ev.dataTransfer.setData('exploitId', e.id)} onclick={() => openEditExploit(e)}>
        {e.name}
      </div>
    {/each}
    <button class="add-btn" onclick={() => showAddExploit = true}>+ Add Exploit</button>
  </div>

  <div class="columns">
    {#each teams as team}
      <div class="column" class:disabled={!team.enabled} ondragover={(e) => e.preventDefault()} ondrop={(e) => onColumnDrop(e, team.id)}>
        <h3>{team.team_name} {!team.enabled ? '(disabled)' : ''}</h3>
        <div class="cards">
          {#each getRunsForTeam(team.id) as run, idx}
            <div 
              class="card" 
              class:disabled={!run.enabled}
              class:dragging={draggingCard?.id === run.id}
              draggable="true"
              ondragstart={(e) => onCardDragStart(e, run)}
              ondragover={(e) => e.preventDefault()}
              ondrop={(e) => onCardDrop(e, run, team.id)}
              ondragend={() => draggingCard = null}
              onclick={(e) => openEditModal(run, e)}
            >
              <span class="card-seq">{idx + 1}</span>
              <span class="card-name">{getExploitName(run.exploit_id)}</span>
              <span class="card-priority">{run.priority ?? 'auto'}</span>
            </div>
          {/each}
        </div>
      </div>
    {/each}
  </div>
</div>

{#if showAddExploit}
  <div class="modal-overlay" onclick={() => showAddExploit = false}>
    <div class="modal" onclick={(e) => e.stopPropagation()}>
      <h3>Add Exploit</h3>
      <input bind:value={newExploit.name} placeholder="Name" />
      <input bind:value={newExploit.docker_image} placeholder="Docker Image" />
      <input bind:value={newExploit.entrypoint} placeholder="Entrypoint (optional)" />
      <input bind:value={newExploit.priority} type="number" placeholder="Priority" />
      <input bind:value={newExploit.max_per_container} type="number" placeholder="Max per container (default: 1)" />
      <input bind:value={newExploit.default_counter} type="number" placeholder="Default counter (default: 999)" />
      <label>Auto-add to teams
        <select bind:value={newExploit.auto_add}>
          <option value="none">Don't add</option>
          <option value="start">Add to start of each team</option>
          <option value="end">Add to end of each team</option>
        </select>
      </label>
      <div class="modal-actions">
        <button onclick={() => showAddExploit = false}>Cancel</button>
        <button onclick={addExploit}>Add</button>
      </div>
    </div>
  </div>
{/if}

{#if editingRun}
  <div class="modal-overlay" onclick={() => editingRun = null}>
    <div class="modal" onclick={(e) => e.stopPropagation()}>
      <h3>Edit Exploit Run</h3>
      <div class="info">
        <p><strong>Exploit:</strong> {getExploitName(editingRun.exploit_id)}</p>
        <p><strong>Team:</strong> {getTeamName(editingRun.team_id)}</p>
        <p><strong>Image:</strong> <code>{getExploit(editingRun.exploit_id)?.docker_image}</code></p>
        <p><strong>Entrypoint:</strong> <code>{getExploit(editingRun.exploit_id)?.entrypoint || 'default'}</code></p>
      </div>
      <label>
        Priority (empty = auto)
        <input bind:value={editForm.priority} type="number" placeholder="Auto" />
      </label>
      <label>
        Sequence
        <input bind:value={editForm.sequence} type="number" />
      </label>
      <label class="checkbox">
        <input type="checkbox" bind:checked={editForm.enabled} />
        Enabled
      </label>
      <div class="modal-actions">
        <button class="danger" onclick={deleteRun}>Delete</button>
        <button onclick={() => editingRun = null}>Cancel</button>
        <button onclick={saveRun}>Save</button>
      </div>
    </div>
  </div>
{/if}

{#if editingExploit}
  <div class="modal-overlay" onclick={() => editingExploit = null}>
    <div class="modal" onclick={(e) => e.stopPropagation()}>
      <h3>Edit Exploit</h3>
      <label>Name <input bind:value={exploitForm.name} /></label>
      <label>Docker Image <input bind:value={exploitForm.docker_image} /></label>
      <label>Entrypoint <input bind:value={exploitForm.entrypoint} placeholder="Optional" /></label>
      <label>Priority <input bind:value={exploitForm.priority} type="number" /></label>
      <label>Max per container <input bind:value={exploitForm.max_per_container} type="number" /></label>
      <label>Default counter <input bind:value={exploitForm.default_counter} type="number" /></label>
      <label class="checkbox"><input type="checkbox" bind:checked={exploitForm.enabled} /> Enabled</label>
      <div class="modal-actions">
        <button class="danger" onclick={deleteExploit}>Delete</button>
        <button onclick={() => editingExploit = null}>Cancel</button>
        <button onclick={saveExploit}>Save</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .board { display: flex; gap: 1rem; height: calc(100vh - 150px); }
  .sidebar { width: 200px; background: #252540; padding: 1rem; border-radius: 8px; }
  .sidebar h3 { margin-top: 0; color: #00d9ff; }
  .exploit-item { background: #1a1a2e; padding: 0.5rem; margin-bottom: 0.5rem; border-radius: 4px; cursor: grab; border: 1px solid #444; }
  .exploit-item:hover { border-color: #00d9ff; }
  .exploit-item.disabled { opacity: 0.5; text-decoration: line-through; }
  .add-btn { width: 100%; margin-top: 0.5rem; }
  .columns { display: flex; gap: 1rem; flex: 1; overflow-x: auto; }
  .column { min-width: 200px; background: #252540; padding: 1rem; border-radius: 8px; }
  .column.disabled { background: #1a1a25; opacity: 0.6; }
  .column h3 { margin-top: 0; font-size: 0.9rem; color: #aaa; }
  .cards { display: flex; flex-direction: column; gap: 0.5rem; min-height: 50px; }
  .card { background: #1a1a2e; padding: 0.75rem; border-radius: 4px; border-left: 3px solid #00d9ff; display: flex; align-items: center; gap: 0.5rem; cursor: pointer; }
  .card:hover { background: #252550; }
  .card.disabled { background: #0d0d15; opacity: 0.6; border-left-color: #444; }
  .card.disabled .card-name { text-decoration: line-through; color: #666; }
  .card.dragging { opacity: 0.4; border: 2px dashed #00d9ff; }
  .card-seq { background: #333; color: #888; font-size: 0.75rem; padding: 0.1rem 0.4rem; border-radius: 3px; }
  .card-name { font-weight: 500; flex: 1; }
  .card-priority { color: #888; font-size: 0.8rem; }
  .modal-overlay { position: fixed; inset: 0; background: rgba(0,0,0,0.7); display: flex; align-items: center; justify-content: center; z-index: 100; }
  .modal { background: #252540; padding: 1.5rem; border-radius: 8px; min-width: 320px; }
  .modal h3 { margin-top: 0; }
  .modal input[type="text"], .modal input[type="number"] { width: 100%; padding: 0.5rem; margin-bottom: 0.5rem; background: #1a1a2e; border: 1px solid #444; color: #eee; border-radius: 4px; box-sizing: border-box; }
  .modal label { display: block; margin-bottom: 0.5rem; color: #aaa; font-size: 0.9rem; }
  .modal label input { margin-top: 0.25rem; }
  .modal label select { display: block; width: 100%; padding: 0.5rem; margin-top: 0.25rem; background: #1a1a2e; border: 1px solid #444; color: #eee; border-radius: 4px; }
  .modal .checkbox { display: flex; align-items: center; gap: 0.5rem; }
  .modal .checkbox input { width: auto; margin: 0; }
  .modal .info { background: #1a1a2e; padding: 0.75rem; border-radius: 4px; margin-bottom: 1rem; }
  .modal .info p { margin: 0.25rem 0; font-size: 0.9rem; }
  .modal .info code { background: #333; padding: 0.1rem 0.3rem; border-radius: 3px; font-size: 0.85rem; }
  .modal-actions { display: flex; gap: 0.5rem; justify-content: flex-end; margin-top: 1rem; }
  .danger { background: #d9534f; }
</style>
