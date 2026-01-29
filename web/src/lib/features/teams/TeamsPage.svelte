<script>
  import * as api from '$lib/data/api';
  import Modal from '$lib/ui/Modal.svelte';

  let { teams, onRefresh } = $props();

  let showTeamModal = $state(false);
  let editingTeam = $state(null);
  let teamForm = $state({ team_id: '', team_name: '', default_ip: '', priority: 0, enabled: true });
  let teamFormInitial = $state(null);
  let pendingToggleTeamId = $state(null);
  let pendingToggleTeamTimer = null;

  function normalizeField(value) {
    if (typeof value === 'boolean') return value;
    return value ?? '';
  }

  function isTeamFieldChanged(field) {
    if (!teamFormInitial) return false;
    return String(normalizeField(teamForm[field])) !== String(normalizeField(teamFormInitial[field]));
  }

  function openAddTeam() {
    editingTeam = null;
    teamForm = { team_id: '', team_name: '', default_ip: '', priority: 0, enabled: true };
    teamFormInitial = { ...teamForm };
    showTeamModal = true;
  }

  function openEditTeam(t) {
    editingTeam = t;
    teamForm = {
      team_id: t.team_id,
      team_name: t.team_name,
      default_ip: t.default_ip ?? '',
      priority: t.priority,
      enabled: t.enabled
    };
    teamFormInitial = { ...teamForm };
    showTeamModal = true;
  }

  async function saveTeam() {
    const data = {
      team_id: teamForm.team_id,
      team_name: teamForm.team_name,
      default_ip: teamForm.default_ip || null,
      priority: Number(teamForm.priority),
      enabled: teamForm.enabled
    };
    if (editingTeam) await api.updateTeam(editingTeam.id, data);
    else await api.createTeam(data);

    showTeamModal = false;
    await onRefresh();
  }

  async function deleteTeam() {
    if (!editingTeam) return;
    if (!confirm('Delete this team?')) return;

    await api.deleteTeam(editingTeam.id);
    showTeamModal = false;
    await onRefresh();
  }

  function closeModal() {
    showTeamModal = false;
  }

  function clearPendingTeamToggle() {
    pendingToggleTeamId = null;
    if (pendingToggleTeamTimer) {
      clearTimeout(pendingToggleTeamTimer);
      pendingToggleTeamTimer = null;
    }
  }

  function startPendingTeamToggle(teamId) {
    clearPendingTeamToggle();
    pendingToggleTeamId = teamId;
    pendingToggleTeamTimer = setTimeout(() => {
      pendingToggleTeamId = null;
      pendingToggleTeamTimer = null;
    }, 2000);
  }

  function queueTeamToggle(team, e) {
    e.stopPropagation();
    if (pendingToggleTeamId === team.id) {
      startPendingTeamToggle(team.id);
      return;
    }
    startPendingTeamToggle(team.id);
  }

  function confirmTeamToggle(team, e) {
    e.stopPropagation();
    if (pendingToggleTeamId !== team.id) return;
    toggleTeamEnabled(team);
  }

  async function toggleTeamEnabled(team) {
    clearPendingTeamToggle();
    await api.updateTeam(team.id, {
      team_id: team.team_id,
      team_name: team.team_name,
      default_ip: team.default_ip ?? null,
      priority: team.priority,
      enabled: !team.enabled
    });
    await onRefresh();
  }

</script>

<svelte:window on:click={clearPendingTeamToggle} />

<div class="panel">
  <div class="panel-header">
    <h2>Teams</h2>
    <button onclick={openAddTeam}>+ Add Team</button>
  </div>
  <table>
    <thead>
      <tr>
        <th>ID</th>
        <th>Team ID</th>
        <th>Name</th>
        <th>Default IP</th>
        <th>Priority</th>
        <th>Enabled</th>
        <th></th>
      </tr>
    </thead>
    <tbody>
      {#each teams as t}
        <tr class:disabled={!t.enabled}>
          <td>{t.id}</td>
          <td>{t.team_id}</td>
          <td>{t.team_name}</td>
          <td>{t.default_ip ?? '-'}</td>
          <td>{t.priority}</td>
          <td class="enable-cell" class:pending={pendingToggleTeamId === t.id}>
            <span class="enable-wrap">
              <button
                type="button"
                class="enable-toggle"
                aria-label={`Toggle team ${t.team_name} ${t.enabled ? 'off' : 'on'}`}
                onclick={(e) => queueTeamToggle(t, e)}
              >
                {t.enabled ? '✓' : '✗'}
              </button>
              {#if pendingToggleTeamId === t.id}
                <button
                  type="button"
                  class="toggle-popup small"
                  aria-label={`Confirm ${t.enabled ? 'disable' : 'enable'} ${t.team_name}`}
                  onclick={(e) => confirmTeamToggle(t, e)}
                >
                  {t.enabled ? '✗' : '✓'}
                </button>
              {/if}
            </span>
          </td>
          <td><button class="small" onclick={() => openEditTeam(t)}>Edit</button></td>
        </tr>
      {/each}
    </tbody>
  </table>
</div>

{#if showTeamModal}
  <Modal onClose={closeModal}>
    <form onsubmit={(e) => { e.preventDefault(); saveTeam(); }}>
      <h3>{editingTeam ? 'Edit' : 'Add'} Team</h3>
      <label class:field-changed={isTeamFieldChanged('team_id')}>
        Team ID <input bind:value={teamForm.team_id} disabled={!!editingTeam} />
      </label>
      <label class:field-changed={isTeamFieldChanged('team_name')}>
        Team Name <input bind:value={teamForm.team_name} />
      </label>
      <label class:field-changed={isTeamFieldChanged('default_ip')}>
        Default IP <input bind:value={teamForm.default_ip} placeholder="Optional" />
      </label>
      <label class:field-changed={isTeamFieldChanged('priority')}>
        Priority <input bind:value={teamForm.priority} type="number" min="0" max="99" />
      </label>
      <label class="checkbox" class:field-changed={isTeamFieldChanged('enabled')}>
        <input type="checkbox" bind:checked={teamForm.enabled} /> Enabled
      </label>
      <div class="modal-actions">
        {#if editingTeam}<button type="button" class="danger" onclick={deleteTeam}>Delete</button>{/if}
        <button type="button" onclick={closeModal}>Cancel</button>
        <button type="submit">Save</button>
      </div>
    </form>
  </Modal>
{/if}

<style>
  .enable-cell {
    white-space: nowrap;
  }

  .enable-wrap {
    position: relative;
    display: inline-flex;
    align-items: center;
  }

  .enable-toggle {
    background: transparent;
    border: none;
    color: inherit;
    cursor: pointer;
    font: inherit;
    padding: 0;
    text-align: left;
  }

  .enable-cell.pending .enable-toggle {
    color: #00d9ff;
  }

  .toggle-popup {
    position: absolute;
    top: 50%;
    left: calc(100% + 0.5rem);
    transform: translateY(-50%);
    min-width: 1.6rem;
    padding: 0.2rem 0.45rem;
    white-space: nowrap;
    z-index: 5;
    font: inherit;
  }

  .toggle-popup:focus-visible,
  .toggle-popup:hover {
    border-color: #00d9ff;
  }
</style>
