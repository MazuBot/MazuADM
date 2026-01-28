<script>
  import * as api from '$lib/data/api';
  import Modal from '$lib/ui/Modal.svelte';
  import { getExploitName, getTeamName } from '$lib/utils/lookup.js';

  let { teams, exploits, exploitRuns, challengeId, onRefresh } = $props();

  const emptyExploit = { name: '', docker_image: '', entrypoint: '', priority: 0, max_per_container: 1, default_counter: 999, timeout_secs: 0, auto_add: 'none', insert_into_rounds: false };
  let showAddExploit = $state(false);
  let newExploit = $state({ ...emptyExploit });
  let newExploitInitial = $state({ ...emptyExploit });

  let editingRun = $state(null);
  let editForm = $state({ priority: '', sequence: 0, enabled: true });
  let editFormInitial = $state(null);

  let editingExploit = $state(null);
  let exploitForm = $state({ name: '', docker_image: '', entrypoint: '', priority: 0, max_per_container: 1, default_counter: 999, timeout_secs: 0, enabled: true });
  let exploitFormInitial = $state(null);

  let editingRelation = $state(null);
  let relationForm = $state({ addr: '', port: '' });
  let relationFormInitial = $state(null);

  let draggingCard = $state(null);

  let filteredExploits = $derived(exploits.filter(e => e.challenge_id === challengeId));

  function normalizeField(value) {
    if (typeof value === 'boolean') return value;
    return value ?? '';
  }

  function isNewExploitChanged(field) {
    if (!newExploitInitial) return false;
    return String(normalizeField(newExploit[field])) !== String(normalizeField(newExploitInitial[field]));
  }

  function isEditRunChanged(field) {
    if (!editFormInitial) return false;
    return String(normalizeField(editForm[field])) !== String(normalizeField(editFormInitial[field]));
  }

  function isExploitFieldChanged(field) {
    if (!exploitFormInitial) return false;
    return String(normalizeField(exploitForm[field])) !== String(normalizeField(exploitFormInitial[field]));
  }

  function isRelationFieldChanged(field) {
    if (!relationFormInitial) return false;
    return String(normalizeField(relationForm[field])) !== String(normalizeField(relationFormInitial[field]));
  }

  function onCardKeydown(e, run) {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      openEditModal(run, e);
    }
  }

  async function runNow(run, e) {
    e.stopPropagation();
    await api.runSingleJob(run.id, run.team_id);
  }

  function getRunsForTeam(teamId) {
    return exploitRuns
      .filter(r => r.challenge_id === challengeId && r.team_id === teamId)
      .sort((a, b) => a.sequence - b.sequence);
  }

  function getExploit(exploitId) {
    return exploits.find(e => e.id === exploitId);
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
      timeout_secs: newExploit.timeout_secs || 0,
      auto_add: newExploit.auto_add,
      insert_into_rounds: newExploit.insert_into_rounds
    });
    showAddExploit = false;
    newExploit = { ...emptyExploit };
    newExploitInitial = { ...emptyExploit };
    onRefresh();
  }

  function openAddExploit() {
    showAddExploit = true;
    newExploit = { ...emptyExploit };
    newExploitInitial = { ...emptyExploit };
  }

  function openEditExploit(e) {
    editingExploit = e;
    exploitForm = { name: e.name, docker_image: e.docker_image, entrypoint: e.entrypoint || '', priority: e.priority, max_per_container: e.max_per_container, default_counter: e.default_counter, timeout_secs: e.timeout_secs || 0, enabled: e.enabled };
    exploitFormInitial = { ...exploitForm };
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

  async function openRelationModal(team) {
    editingRelation = team;
    const rel = await api.getRelation(challengeId, team.id);
    relationForm = { addr: rel?.addr || '', port: rel?.port || '' };
    relationFormInitial = { ...relationForm };
  }

  async function saveRelation() {
    await api.updateRelation(challengeId, editingRelation.id, {
      addr: relationForm.addr || null,
      port: relationForm.port ? Number(relationForm.port) : null
    });
    editingRelation = null;
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
    editFormInitial = { ...editForm };
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
      <button
        type="button"
        class="exploit-item"
        class:disabled={!e.enabled}
        draggable="true"
        ondragstart={(ev) => ev.dataTransfer.setData('exploitId', e.id)}
        onclick={() => openEditExploit(e)}
      >
        {e.name}
      </button>
    {/each}
    <button class="add-btn" onclick={openAddExploit}>+ Add Exploit</button>
  </div>

  <div class="columns">
    {#each teams as team}
      <div
        class="column"
        class:disabled={!team.enabled}
        role="list"
        aria-label={`Runs for ${team.team_name}`}
        ondragover={(e) => e.preventDefault()}
        ondrop={(e) => onColumnDrop(e, team.id)}
      >
        <h3>
          {team.team_name} {!team.enabled ? '(disabled)' : ''}
          <button
            type="button"
            class="gear"
            aria-label={`Edit connection for ${team.team_name}`}
            onclick={(e) => { e.stopPropagation(); openRelationModal(team); }}
          >
            ⚙️
          </button>
        </h3>
        <div class="cards">
          {#each getRunsForTeam(team.id) as run, idx}
            <div 
              class="card" 
              class:disabled={!run.enabled || !getExploit(run.exploit_id)?.enabled}
              class:dragging={draggingCard?.id === run.id}
              role="button"
              tabindex="0"
              aria-disabled={!run.enabled || !getExploit(run.exploit_id)?.enabled}
              draggable="true"
              ondragstart={(e) => onCardDragStart(e, run)}
              ondragover={(e) => e.preventDefault()}
              ondrop={(e) => onCardDrop(e, run, team.id)}
              ondragend={() => draggingCard = null}
              onclick={(e) => openEditModal(run, e)}
              onkeydown={(e) => onCardKeydown(e, run)}
            >
              <span class="card-seq">{idx + 1}</span>
              <span class="card-name">{getExploitName(exploits, run.exploit_id)}</span>
              <span class="card-priority">{run.priority ?? 'auto'}</span>
              <button
                type="button"
                class="card-play"
                title="Run now"
                aria-label="Run now"
                onclick={(e) => runNow(run, e)}
              >
                ▶
              </button>
            </div>
          {/each}
        </div>
      </div>
    {/each}
  </div>
</div>

{#if showAddExploit}
  <Modal onClose={() => showAddExploit = false}>
    <form onsubmit={(e) => { e.preventDefault(); addExploit(); }}>
      <h3>Add Exploit</h3>
      <label class:field-changed={isNewExploitChanged('name')}>
        Name <input type="text" bind:value={newExploit.name} />
      </label>
      <label class:field-changed={isNewExploitChanged('docker_image')}>
        Docker Image <input type="text" bind:value={newExploit.docker_image} />
      </label>
      <label class:field-changed={isNewExploitChanged('entrypoint')}>
        Entrypoint <input type="text" bind:value={newExploit.entrypoint} placeholder="Leave empty to use image CMD" />
      </label>
      <label class:field-changed={isNewExploitChanged('priority')}>
        Priority <input bind:value={newExploit.priority} type="number" />
      </label>
      <label class:field-changed={isNewExploitChanged('max_per_container')}>
        Max per container <input bind:value={newExploit.max_per_container} type="number" placeholder="Default: 1" />
      </label>
      <label class:field-changed={isNewExploitChanged('default_counter')}>
        Default counter <input bind:value={newExploit.default_counter} type="number" placeholder="Default: 999" />
      </label>
      <label class:field-changed={isNewExploitChanged('timeout_secs')}>
        Timeout (secs) <input bind:value={newExploit.timeout_secs} type="number" placeholder="0 = use global" />
      </label>
      <label class:field-changed={isNewExploitChanged('auto_add')}>
        Auto-add to teams
        <select bind:value={newExploit.auto_add}>
          <option value="none">Don't add</option>
          <option value="start">Add to start of each team</option>
          <option value="end">Add to end of each team</option>
        </select>
      </label>
      <label class="checkbox" class:field-changed={isNewExploitChanged('insert_into_rounds')}>
        <input type="checkbox" bind:checked={newExploit.insert_into_rounds} /> Insert jobs into active rounds
      </label>
      <div class="modal-actions">
        <button type="button" onclick={() => showAddExploit = false}>Cancel</button>
        <button type="submit">Add</button>
      </div>
    </form>
  </Modal>
{/if}

{#if editingRun}
  <Modal onClose={() => editingRun = null}>
    <form onsubmit={(e) => { e.preventDefault(); saveRun(); }}>
      <h3>Edit Exploit Run</h3>
      <div class="info">
        <p><strong>Exploit:</strong> {getExploitName(exploits, editingRun.exploit_id)}</p>
        <p><strong>Team:</strong> {getTeamName(teams, editingRun.team_id)}</p>
        <p><strong>Image:</strong> <code>{getExploit(editingRun.exploit_id)?.docker_image}</code></p>
        <p><strong>Entrypoint:</strong> <code>{getExploit(editingRun.exploit_id)?.entrypoint || '(image CMD)'}</code></p>
      </div>
      <label class:field-changed={isEditRunChanged('priority')}>
        Priority <input bind:value={editForm.priority} type="number" placeholder="Auto" />
      </label>
      <label class:field-changed={isEditRunChanged('sequence')}>
        Sequence <input bind:value={editForm.sequence} type="number" />
      </label>
      <label class="checkbox" class:field-changed={isEditRunChanged('enabled')}>
        <input type="checkbox" bind:checked={editForm.enabled} /> Enabled
      </label>
      <div class="modal-actions">
        <button type="button" class="danger" onclick={deleteRun}>Delete</button>
        <button type="button" onclick={() => editingRun = null}>Cancel</button>
        <button type="submit">Save</button>
      </div>
    </form>
  </Modal>
{/if}

{#if editingExploit}
  <Modal onClose={() => editingExploit = null}>
    <form onsubmit={(e) => { e.preventDefault(); saveExploit(); }}>
      <h3>Edit Exploit</h3>
      <label class:field-changed={isExploitFieldChanged('name')}>
        Name <input type="text" bind:value={exploitForm.name} />
      </label>
      <label class:field-changed={isExploitFieldChanged('docker_image')}>
        Docker Image <input type="text" bind:value={exploitForm.docker_image} />
      </label>
      <label class:field-changed={isExploitFieldChanged('entrypoint')}>
        Entrypoint <input type="text" bind:value={exploitForm.entrypoint} placeholder="Leave empty to use image CMD" />
      </label>
      <label class:field-changed={isExploitFieldChanged('priority')}>
        Priority <input bind:value={exploitForm.priority} type="number" />
      </label>
      <label class:field-changed={isExploitFieldChanged('max_per_container')}>
        Max per container <input bind:value={exploitForm.max_per_container} type="number" />
      </label>
      <label class:field-changed={isExploitFieldChanged('default_counter')}>
        Default counter <input bind:value={exploitForm.default_counter} type="number" />
      </label>
      <label class:field-changed={isExploitFieldChanged('timeout_secs')}>
        Timeout (secs) <input bind:value={exploitForm.timeout_secs} type="number" placeholder="0 = use global" />
      </label>
      <label class="checkbox" class:field-changed={isExploitFieldChanged('enabled')}>
        <input type="checkbox" bind:checked={exploitForm.enabled} /> Enabled
      </label>
      <div class="modal-actions">
        <button type="button" class="danger" onclick={deleteExploit}>Delete</button>
        <button type="button" onclick={() => editingExploit = null}>Cancel</button>
        <button type="submit">Save</button>
      </div>
    </form>
  </Modal>
{/if}

{#if editingRelation}
  <Modal onClose={() => editingRelation = null}>
    <form onsubmit={(e) => { e.preventDefault(); saveRelation(); }}>
      <h3>Connection for {editingRelation.team_name}</h3>
      <p class="hint">Leave empty to use team default IP + challenge default port</p>
      <label class:field-changed={isRelationFieldChanged('addr')}>
        IP/Host <input type="text" bind:value={relationForm.addr} placeholder="Team default" />
      </label>
      <label class:field-changed={isRelationFieldChanged('port')}>
        Port <input bind:value={relationForm.port} type="number" placeholder="Challenge default" />
      </label>
      <div class="modal-actions">
        <button type="button" onclick={() => editingRelation = null}>Cancel</button>
        <button type="submit">Save</button>
      </div>
    </form>
  </Modal>
{/if}

<style>
  .board { display: flex; gap: 1rem; height: calc(100vh - 150px); }
  .sidebar { width: 200px; background: #252540; padding: 1rem; border-radius: 8px; }
  .sidebar h3 { margin-top: 0; color: #00d9ff; }
  .exploit-item { background: #1a1a2e; padding: 0.5rem; margin-bottom: 0.5rem; border-radius: 4px; cursor: grab; border: 1px solid #444; width: 100%; text-align: left; color: inherit; }
  .exploit-item { appearance: none; background-color: #1a1a2e; }
  .exploit-item:hover { border-color: #00d9ff; }
  .exploit-item.disabled { opacity: 0.5; text-decoration: line-through; }
  .add-btn { width: 100%; margin-top: 0.5rem; }
  .columns { display: flex; gap: 1rem; flex: 1; overflow-x: auto; }
  .column { min-width: 200px; background: #252540; padding: 1rem; border-radius: 8px; }
  .column.disabled { background: #1a1a25; opacity: 0.6; }
  .column h3 { margin-top: 0; font-size: 0.9rem; color: #aaa; display: flex; justify-content: space-between; align-items: center; }
  .gear { cursor: pointer; opacity: 0.5; font-size: 0.8rem; background: transparent; border: none; padding: 0; color: inherit; }
  .gear:hover { opacity: 1; }
  .hint { color: #666; font-size: 0.85rem; margin: 0.5rem 0; }
  .cards { display: flex; flex-direction: column; gap: 0.5rem; min-height: 50px; }
  .card { background: #1a1a2e; padding: 0.75rem; border-radius: 4px; border-left: 3px solid #00d9ff; display: flex; align-items: center; gap: 0.5rem; cursor: pointer; }
  .card:hover { background: #252550; }
  .card.disabled { background: #0d0d15; opacity: 0.6; border-left-color: #444; }
  .card.disabled .card-name { text-decoration: line-through; color: #666; }
  .card.dragging { opacity: 0.4; border: 2px dashed #00d9ff; }
  .card-seq { background: #333; color: #888; font-size: 0.75rem; padding: 0.1rem 0.4rem; border-radius: 3px; }
  .card-name { font-weight: 500; flex: 1; }
  .card-priority { color: #888; font-size: 0.8rem; }
  .card-play { cursor: pointer; opacity: 0.5; font-size: 0.7rem; margin-left: auto; background: transparent; border: none; padding: 0; color: inherit; }
  .card-play:hover { opacity: 1; }
  .danger { background: #d9534f; }
</style>
