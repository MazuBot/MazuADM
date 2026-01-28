<script>
  import { api } from '$lib/api.js';

  let { teams, onRefresh } = $props();

  let showTeamModal = $state(false);
  let editingTeam = $state(null);
  let teamForm = $state({ team_id: '', team_name: '', default_ip: '', priority: 0, enabled: true });

  function openAddTeam() {
    editingTeam = null;
    teamForm = { team_id: '', team_name: '', default_ip: '', priority: 0, enabled: true };
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

  function onOverlayClick(e) {
    if (e.target === e.currentTarget) closeModal();
  }

  function onOverlayKeydown(e) {
    if (e.key === 'Escape') closeModal();
  }
</script>

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
          <td>{t.enabled ? '✓' : '✗'}</td>
          <td><button class="small" onclick={() => openEditTeam(t)}>Edit</button></td>
        </tr>
      {/each}
    </tbody>
  </table>
</div>

{#if showTeamModal}
  <div
    class="modal-overlay"
    role="button"
    tabindex="0"
    aria-label="Close modal"
    onclick={onOverlayClick}
    onkeydown={onOverlayKeydown}
  >
    <div class="modal" role="dialog" aria-modal="true">
      <h3>{editingTeam ? 'Edit' : 'Add'} Team</h3>
      <label>Team ID <input bind:value={teamForm.team_id} disabled={!!editingTeam} /></label>
      <label>Team Name <input bind:value={teamForm.team_name} /></label>
      <label>Default IP <input bind:value={teamForm.default_ip} placeholder="Optional" /></label>
      <label>Priority <input bind:value={teamForm.priority} type="number" /></label>
      <label class="checkbox"><input type="checkbox" bind:checked={teamForm.enabled} /> Enabled</label>
      <div class="modal-actions">
        {#if editingTeam}<button class="danger" onclick={deleteTeam}>Delete</button>{/if}
        <button onclick={closeModal}>Cancel</button>
        <button onclick={saveTeam}>Save</button>
      </div>
    </div>
  </div>
{/if}
