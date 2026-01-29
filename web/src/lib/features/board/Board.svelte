<script>
  import { onMount, tick } from 'svelte';
  import * as api from '$lib/data/api';
  import Modal from '$lib/ui/Modal.svelte';
  import { getChallengeName, getExploitName, getTeamDisplay } from '$lib/utils/lookup.js';

  let { challenges, teams, exploits, exploitRuns, challengeId, onRefresh } = $props();

  function getNewExploitDefaults() {
    return {
      name: '',
      docker_image: '',
      entrypoint: '',
      max_per_container: Math.max(teams.length, 1),
      max_containers: 0,
      default_counter: 999,
      timeout_secs: 0,
      auto_add: 'end',
      insert_into_rounds: true
    };
  }

  let showAddExploit = $state(false);
  let selectedExploitId = $state(null);
  let selectedTeamId = $state(null);
  let lastScrollKey = '';
  let newExploit = $state(getNewExploitDefaults());
  let newExploitInitial = $state(getNewExploitDefaults());

  let editingRun = $state(null);
  let editForm = $state({ priority: '', sequence: 0, enabled: true });
  let editFormInitial = $state(null);

  let editingExploit = $state(null);
  let exploitForm = $state({ name: '', docker_image: '', entrypoint: '', max_per_container: 1, max_containers: 0, default_counter: 999, timeout_secs: 0, enabled: true });
  let exploitFormInitial = $state(null);

  let editingRelation = $state(null);
  let relationForm = $state({ addr: '', port: '' });
  let relationFormInitial = $state(null);

  let draggingCard = $state(null);
  let dragOverIndex = $state(null);
  let dragOverTeamId = $state(null);
  let dragPreviewEl = null;
  let optimisticSequences = $state(new Map());

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

  function onExploitKeydown(e, exploitId) {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      toggleExploitSelection(exploitId);
    }
  }

  function toggleExploitSelection(exploitId) {
    selectedExploitId = selectedExploitId === exploitId ? null : exploitId;
  }

  function teamAnchor(team) {
    return team?.team_id || String(team?.id ?? '');
  }

  async function scrollToTeamAnchor(anchor) {
    await tick();
    const el = document.getElementById(anchor);
    if (el) {
      el.scrollIntoView({ behavior: 'smooth', block: 'nearest', inline: 'start' });
    }
  }

  function applyHashSelection(shouldScroll) {
    const hash = window.location.hash.slice(1);
    if (!hash) {
      selectedTeamId = null;
      return;
    }
    selectedTeamId = hash;
    if (shouldScroll && teams.some((team) => team.team_id === hash)) {
      scrollToTeamAnchor(hash);
    }
  }

  async function jumpToTeam(team, e) {
    e?.preventDefault();
    const anchor = teamAnchor(team);
    selectedTeamId = anchor || null;
    const hash = `#${anchor}`;
    if (window.location.hash !== hash) {
      history.replaceState(null, '', hash);
    }
    await scrollToTeamAnchor(anchor);
  }

  onMount(() => {
    const handleHash = () => {
      applyHashSelection(true);
    };
    handleHash();
    window.addEventListener('hashchange', handleHash);
    return () => window.removeEventListener('hashchange', handleHash);
  });

  onMount(() => {
    const onKeydown = (e) => {
      if (e.key === 'Escape' && draggingCard) {
        e.preventDefault();
        cancelDrag();
      }
    };
    window.addEventListener('keydown', onKeydown);
    return () => window.removeEventListener('keydown', onKeydown);
  });

  $effect(() => {
    const hash = window.location.hash.slice(1);
    if (!hash) {
      selectedTeamId = null;
      return;
    }
    selectedTeamId = hash;
    if (!teams.some((team) => team.team_id === hash)) return;
    const scrollKey = `${challengeId}:${hash}`;
    if (lastScrollKey === scrollKey) return;
    lastScrollKey = scrollKey;
    scrollToTeamAnchor(hash);
  });

  async function runNow(run, e) {
    e.stopPropagation();
    await api.enqueueSingleJob(run.id, run.team_id);
  }

  function getSequence(run) {
    const override = optimisticSequences.get(run.id);
    return override ?? run.sequence;
  }

  function getRunsForTeam(teamId) {
    return exploitRuns
      .filter(r => r.challenge_id === challengeId && r.team_id === teamId)
      .sort((a, b) => getSequence(a) - getSequence(b));
  }

  function getExploit(exploitId) {
    return exploits.find(e => e.id === exploitId);
  }

  async function addExploit() {
    await api.createExploit({ 
      name: newExploit.name,
      docker_image: newExploit.docker_image,
      challenge_id: challengeId,
      entrypoint: newExploit.entrypoint || null,
      max_per_container: newExploit.max_per_container,
      max_containers: newExploit.max_containers,
      default_counter: newExploit.default_counter,
      timeout_secs: newExploit.timeout_secs || 0,
      auto_add: newExploit.auto_add,
      insert_into_rounds: newExploit.insert_into_rounds
    });
    showAddExploit = false;
    const defaults = getNewExploitDefaults();
    newExploit = { ...defaults };
    newExploitInitial = { ...defaults };
    onRefresh();
  }

  function openAddExploit() {
    showAddExploit = true;
    const defaults = getNewExploitDefaults();
    newExploit = { ...defaults };
    newExploitInitial = { ...defaults };
  }

  function openEditExploit(e) {
    editingExploit = e;
    exploitForm = { name: e.name, docker_image: e.docker_image, entrypoint: e.entrypoint || '', max_per_container: e.max_per_container, max_containers: e.max_containers, default_counter: e.default_counter, timeout_secs: e.timeout_secs || 0, enabled: e.enabled };
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

  async function deleteExploitFromList(exploit, e) {
    e.stopPropagation();
    if (confirm('Delete this exploit and all its runs?')) {
      await api.deleteExploit(exploit.id);
      if (editingExploit?.id === exploit.id) {
        editingExploit = null;
      }
      if (selectedExploitId === exploit.id) {
        selectedExploitId = null;
      }
      onRefresh();
    }
  }

  function editExploitFromList(exploit, e) {
    e.stopPropagation();
    openEditExploit(exploit);
  }

  async function toggleExploitEnabled(exploit, e) {
    e.stopPropagation();
    await api.updateExploit(exploit.id, {
      name: exploit.name,
      docker_image: exploit.docker_image,
      entrypoint: exploit.entrypoint,
      enabled: !exploit.enabled
    });
    onRefresh();
  }

  async function appendExploitToAllTeams(exploitId, e) {
    e.stopPropagation();
    if (!confirm('Append this exploit to all teams?')) {
      return;
    }
    for (const team of teams) {
      const hasRun = exploitRuns.some((r) => r.challenge_id === challengeId && r.team_id === team.id && r.exploit_id === exploitId);
      if (hasRun) continue;
      const runs = getRunsForTeam(team.id);
      const maxSeq = runs.length > 0 ? Math.max(...runs.map(r => r.sequence)) : -1;
      await api.createExploitRun({ exploit_id: exploitId, challenge_id: challengeId, team_id: team.id, sequence: maxSeq + 1 });
    }
    onRefresh();
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
    const rawPriority = editForm.priority;
    const priority =
      rawPriority === '' || rawPriority === null || rawPriority === undefined
        ? null
        : Number(rawPriority);
    await api.updateExploitRun(editingRun.id, {
      priority: Number.isNaN(priority) ? null : priority,
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

  async function deleteRunFromCard(run, e) {
    e.stopPropagation();
    if (confirm('Delete this exploit run?')) {
      await api.deleteExploitRun(run.id);
      onRefresh();
    }
  }

  async function toggleRunEnabled(run, e) {
    e.stopPropagation();
    await api.updateExploitRun(run.id, { enabled: !run.enabled });
    onRefresh();
  }

  function cleanupDragPreview() {
    if (dragPreviewEl) {
      dragPreviewEl.remove();
      dragPreviewEl = null;
    }
  }

  function clearDragOver() {
    dragOverIndex = null;
    dragOverTeamId = null;
  }

  function setOptimisticOrder(runs) {
    const next = new Map(optimisticSequences);
    runs.forEach((run, i) => next.set(run.id, i));
    optimisticSequences = next;
  }

  function clearOptimisticOrder(runIds) {
    if (!optimisticSequences.size) return;
    if (!runIds) {
      optimisticSequences = new Map();
      return;
    }
    const next = new Map(optimisticSequences);
    runIds.forEach((id) => next.delete(id));
    optimisticSequences = next;
  }

  function cancelDrag() {
    cleanupDragPreview();
    clearDragOver();
    draggingCard = null;
  }

  function getDisplayedRuns(teamId, baseRuns) {
    const runs = baseRuns ?? getRunsForTeam(teamId);
    if (!draggingCard || draggingCard.team_id !== teamId) return runs;
    const fromIdx = runs.findIndex(r => r.id === draggingCard.id);
    if (fromIdx < 0) return runs;
    let targetIdx = dragOverTeamId === teamId ? (dragOverIndex ?? fromIdx) : fromIdx;
    if (targetIdx === fromIdx) return runs;
    const reordered = [...runs];
    reordered.splice(fromIdx, 1);
    if (targetIdx > fromIdx) targetIdx -= 1;
    const clampedIdx = Math.max(0, Math.min(reordered.length, targetIdx));
    reordered.splice(clampedIdx, 0, draggingCard);
    return reordered;
  }

  function onCardDragStart(e, run) {
    draggingCard = run;
    dragOverIndex = null;
    dragOverTeamId = run.team_id;
    if (e.dataTransfer) {
      e.dataTransfer.effectAllowed = 'move';
      cleanupDragPreview();

      const cardEl = e.currentTarget;
      if (cardEl) {
        const preview = cardEl.cloneNode(true);
        preview.classList.add('drag-preview');
        preview.style.width = `${cardEl.offsetWidth}px`;
        preview.style.height = `${cardEl.offsetHeight}px`;
        preview.style.position = 'absolute';
        preview.style.top = '-1000px';
        preview.style.left = '-1000px';
        document.body.appendChild(preview);
        dragPreviewEl = preview;
        e.dataTransfer.setDragImage(preview, 16, 16);
      }
    }
  }

  async function onCardDrop(e, teamId) {
    e.preventDefault();
    cleanupDragPreview();
    if (!draggingCard || draggingCard.team_id !== teamId) {
      clearDragOver();
      draggingCard = null;
      return;
    }

    const runs = getRunsForTeam(teamId);
    const fromIdx = runs.findIndex(r => r.id === draggingCard.id);
    if (fromIdx < 0) {
      clearDragOver();
      draggingCard = null;
      return;
    }
    let targetIdx = dragOverTeamId === teamId ? (dragOverIndex ?? fromIdx) : fromIdx;
    if (targetIdx === fromIdx) {
      clearDragOver();
      draggingCard = null;
      return;
    }

    // Reorder
    const reordered = [...runs];
    reordered.splice(fromIdx, 1);
    if (targetIdx > fromIdx) targetIdx -= 1;
    const clampedIdx = Math.max(0, Math.min(reordered.length, targetIdx));
    reordered.splice(clampedIdx, 0, draggingCard);

    const updates = reordered
      .map((r, i) => ({ id: r.id, sequence: i }))
      .filter((u, i) => reordered[i].sequence !== u.sequence);
    const runIds = reordered.map((r) => r.id);
    setOptimisticOrder(reordered);
    clearDragOver();
    draggingCard = null;
    try {
      if (updates.length > 0) {
        await api.reorderExploitRuns(updates);
      }
      await onRefresh();
    } finally {
      clearOptimisticOrder(runIds);
    }
  }

  function onColumnDrop(e, teamId) {
    e.preventDefault();
    cleanupDragPreview();
    const exploitId = e.dataTransfer.getData('exploitId');
    if (exploitId) {
      addRun(+exploitId, teamId);
    }
    clearDragOver();
    draggingCard = null;
  }

  function onCardDragEnd() {
    cancelDrag();
  }

  function onCardDragOver(e, run, teamId, baseIndex) {
    e.preventDefault();
    if (!draggingCard || draggingCard.team_id !== teamId) return;
    if (run.id === draggingCard.id) return;
    const rect = e.currentTarget?.getBoundingClientRect?.();
    const midpoint = rect ? rect.top + rect.height / 2 : e.clientY;
    const targetIndex = e.clientY < midpoint ? baseIndex : baseIndex + 1;
    if (dragOverIndex === targetIndex && dragOverTeamId === teamId) return;
    dragOverIndex = targetIndex;
    dragOverTeamId = teamId;
  }
</script>

<div class="board">
  <div class="sidebar">
    <h3>Exploits</h3>
    {#each filteredExploits as e}
      <div
        class="exploit-item"
        class:disabled={!e.enabled}
        class:selected={selectedExploitId === e.id}
        role="button"
        tabindex="0"
        aria-pressed={selectedExploitId === e.id}
        draggable="true"
        ondragstart={(ev) => ev.dataTransfer.setData('exploitId', e.id)}
        onclick={() => toggleExploitSelection(e.id)}
        onkeydown={(ev) => onExploitKeydown(ev, e.id)}
      >
        <span class="exploit-name">{e.name}</span>
        <div class="exploit-actions">
          <button
            type="button"
            class="exploit-action"
            title={e.enabled ? 'Disable exploit' : 'Enable exploit'}
            aria-label={e.enabled ? 'Disable exploit' : 'Enable exploit'}
            onclick={(ev) => toggleExploitEnabled(e, ev)}
          >
            {e.enabled ? '⏸' : '▶'}
          </button>
          <button
            type="button"
            class="exploit-action"
            title="Append to all teams"
            aria-label="Append to all teams"
            onclick={(ev) => appendExploitToAllTeams(e.id, ev)}
          >
            +
          </button>
          <button
            type="button"
            class="exploit-action"
            title="Edit exploit"
            aria-label="Edit exploit"
            onclick={(ev) => editExploitFromList(e, ev)}
          >
            ✎
          </button>
          <button
            type="button"
            class="exploit-action"
            title="Delete exploit"
            aria-label="Delete exploit"
            onclick={(ev) => deleteExploitFromList(e, ev)}
          >
            ✕
          </button>
        </div>
      </div>
    {/each}
    <button class="add-btn" onclick={openAddExploit}>+ Add Exploit</button>
  </div>

  <div class="columns">
    {#each teams as team}
      {@const baseRuns = getRunsForTeam(team.id)}
      {@const orderIndex = new Map(baseRuns.map((r, i) => [r.id, i]))}
      <div
        class="column"
        class:disabled={!team.enabled}
        class:highlighted={selectedTeamId === team.team_id}
        id={teamAnchor(team)}
        role="list"
        aria-label={`Runs for ${team.team_name}`}
        ondragover={(e) => e.preventDefault()}
        ondrop={(e) => onColumnDrop(e, team.id)}
      >
        <h3>
          <a class="team-link" href={`#${team.team_id}`} onclick={(e) => jumpToTeam(team, e)}>
            <span class="truncate">{getTeamDisplay(teams, team.id)}</span>
          </a>
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
          {#each getDisplayedRuns(team.id, baseRuns) as run (run.id)}
            <div 
              class="card" 
              class:disabled={!run.enabled || !getExploit(run.exploit_id)?.enabled}
              class:dragging={draggingCard?.id === run.id}
              class:highlighted={selectedExploitId && run.exploit_id === selectedExploitId}
              role="button"
              tabindex="0"
              aria-disabled={!run.enabled || !getExploit(run.exploit_id)?.enabled}
              draggable="true"
              ondragstart={(e) => onCardDragStart(e, run)}
              ondragover={(e) => onCardDragOver(e, run, team.id, orderIndex.get(run.id) ?? 0)}
              ondrop={(e) => onCardDrop(e, team.id)}
              ondragend={onCardDragEnd}
              onclick={(e) => openEditModal(run, e)}
              onkeydown={(e) => onCardKeydown(e, run)}
            >
              <span class="card-seq">{(orderIndex.get(run.id) ?? 0) + 1}</span>
              <span class="card-name">{getExploitName(exploits, run.exploit_id)}</span>
              <span class="card-priority">{run.priority ?? 'auto'}</span>
              <div class="card-actions">
                <button
                  type="button"
                  class="exploit-action"
                  title={run.enabled ? 'Disable run' : 'Enable run'}
                  aria-label={run.enabled ? 'Disable run' : 'Enable run'}
                  onclick={(e) => toggleRunEnabled(run, e)}
                >
                  {run.enabled ? '⏸' : '▶'}
                </button>
                <button
                  type="button"
                  class="exploit-action"
                  title="Delete run"
                  aria-label="Delete run"
                  onclick={(e) => deleteRunFromCard(run, e)}
                >
                  ✕
                </button>
              </div>
              <button
                type="button"
                class="card-play"
                title="Enqueue now"
                aria-label="Enqueue now"
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
      <h3 class="modal-title">
        <span>Add Exploit</span>
        <code>{getChallengeName(challenges, challengeId)}</code>
      </h3>
      <label class:field-changed={isNewExploitChanged('name')}>
        Name <input type="text" bind:value={newExploit.name} />
      </label>
      <label class:field-changed={isNewExploitChanged('docker_image')}>
        Docker Image <input type="text" bind:value={newExploit.docker_image} />
      </label>
      <label class:field-changed={isNewExploitChanged('entrypoint')}>
        Entrypoint <input type="text" bind:value={newExploit.entrypoint} placeholder="Leave empty to use image CMD" />
      </label>
      <label class:field-changed={isNewExploitChanged('max_per_container')}>
        Max per container <input bind:value={newExploit.max_per_container} type="number" placeholder={`Default: ${Math.max(teams.length, 1)}`} />
      </label>
      <label class:field-changed={isNewExploitChanged('max_containers')}>
        Max containers <input bind:value={newExploit.max_containers} type="number" placeholder="0 = unlimited" />
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
      <h3 class="modal-title">
        <span>Edit Exploit Run</span>
        <code>{getChallengeName(challenges, challengeId)}</code>
      </h3>
      <div class="info">
        <p><strong>Exploit:</strong> {getExploitName(exploits, editingRun.exploit_id)}</p>
      <p><strong>Team:</strong> <span class="truncate">{getTeamDisplay(teams, editingRun.team_id)}</span></p>
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
      <h3 class="modal-title">
        <span>Edit Exploit</span>
        <code>{getChallengeName(challenges, challengeId)}</code>
      </h3>
      <label class:field-changed={isExploitFieldChanged('name')}>
        Name <input type="text" bind:value={exploitForm.name} />
      </label>
      <label class:field-changed={isExploitFieldChanged('docker_image')}>
        Docker Image <input type="text" bind:value={exploitForm.docker_image} />
      </label>
      <label class:field-changed={isExploitFieldChanged('entrypoint')}>
        Entrypoint <input type="text" bind:value={exploitForm.entrypoint} placeholder="Leave empty to use image CMD" />
      </label>
      <label class:field-changed={isExploitFieldChanged('max_per_container')}>
        Max per container <input bind:value={exploitForm.max_per_container} type="number" />
      </label>
      <label class:field-changed={isExploitFieldChanged('max_containers')}>
        Max containers <input bind:value={exploitForm.max_containers} type="number" placeholder="0 = unlimited" />
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
      <h3 class="modal-title">
        <span>Connection for <span class="truncate">{getTeamDisplay(teams, editingRelation.id)}</span></span>
        <code>{getChallengeName(challenges, challengeId)}</code>
      </h3>
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
  .exploit-item { background: #1a1a2e; padding: 0.5rem; margin-bottom: 0.5rem; border-radius: 4px; cursor: grab; border: 1px solid #444; width: 100%; text-align: left; color: inherit; position: relative; overflow: visible; box-sizing: border-box; }
  .exploit-item { appearance: none; background-color: #1a1a2e; }
  .exploit-item:hover { border-color: #00d9ff; }
  .exploit-item.disabled { opacity: 0.5; text-decoration: line-through; }
  .exploit-item.selected { border-color: #00d9ff; box-shadow: 0 0 0 1px #00d9ff inset; }
  .exploit-name { display: block; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .exploit-actions { position: absolute; top: -0.4rem; right: -0.2rem; display: flex; gap: 0.2rem; opacity: 0; pointer-events: none; z-index: 1; }
  .exploit-item:hover .exploit-actions, .exploit-item:focus-within .exploit-actions { opacity: 1; pointer-events: auto; }
  .exploit-action { background: #fff; border: none; color: #333; font-size: 0.7rem; line-height: 1; width: 1rem; height: 1rem; padding: 0; display: flex; align-items: center; justify-content: center; border-radius: 50%; aspect-ratio: 1; box-sizing: border-box; appearance: none; cursor: pointer; }
  .exploit-action:hover { color: #111; }
  .add-btn { width: 100%; margin-top: 0.5rem; }
  .columns { display: flex; gap: 1rem; flex: 1; overflow-x: auto; }
  .column { min-width: 200px; background: #252540; padding: 1rem; border-radius: 8px; }
  .column.highlighted { box-shadow: 0 0 0 2px #00d9ff inset; }
  .column.disabled { background: #1a1a25; opacity: 0.6; }
  .column h3 { margin-top: 0; font-size: 0.9rem; color: #aaa; display: flex; justify-content: space-between; align-items: center; gap: 0.5rem; min-width: 0; }
  .team-link { color: inherit; text-decoration: none; display: inline-flex; align-items: center; gap: 0.35rem; flex: 1 1 auto; min-width: 0; }
  .team-link .truncate { max-width: 100%; }
  .team-link:hover { color: #00d9ff; }
  .gear { cursor: pointer; opacity: 0.5; font-size: 0.8rem; background: transparent; border: none; padding: 0; color: inherit; }
  .gear:hover { opacity: 1; }
  .hint { color: #666; font-size: 0.85rem; margin: 0.5rem 0; }
  .cards { display: flex; flex-direction: column; gap: 0.5rem; min-height: 50px; }
  .card { background: #1a1a2e; padding: 0.75rem 1.75rem 0.75rem 0.75rem; border-radius: 4px; border-left: 3px solid #00d9ff; display: flex; align-items: center; gap: 0.5rem; cursor: pointer; position: relative; }
  .card:hover { background: #252550; }
  .card.disabled { background: #0d0d15; opacity: 0.6; border-left-color: #444; }
  .card.disabled .card-name { text-decoration: line-through; color: #666; }
  .card.dragging { opacity: 0.4; border: 2px dashed #00d9ff; }
  .card.highlighted { box-shadow: 0 0 0 1px #00d9ff inset; background: #202044; }
  :global(.drag-preview) { pointer-events: none; box-shadow: 0 10px 24px rgba(0, 0, 0, 0.35); transform: rotate(-1deg); }
  :global(.drag-preview .card-actions),
  :global(.drag-preview .card-play) { display: none; }
  .card-seq { background: #333; color: #888; font-size: 0.75rem; padding: 0.1rem 0.4rem; border-radius: 3px; }
  .card-name { font-weight: 500; flex: 1; }
  .card-priority { color: #888; font-size: 0.8rem; }
  .card-actions { position: absolute; top: -0.4rem; right: -0.2rem; display: flex; gap: 0.2rem; opacity: 0; pointer-events: none; z-index: 1; }
  .card:hover .card-actions, .card:focus-within .card-actions { opacity: 1; pointer-events: auto; }
  .card-play { cursor: pointer; opacity: 0.5; font-size: 0.7rem; margin-left: auto; background: transparent; border: none; padding: 0; color: inherit; }
  .card-play:hover { opacity: 1; }
  .danger { background: #d9534f; }
  .modal-title { display: flex; align-items: center; justify-content: space-between; gap: 1rem; }
  .modal-title code { margin-left: auto; text-align: right; }
</style>
