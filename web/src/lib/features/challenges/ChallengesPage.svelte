<script>
  import * as api from '$lib/data/api';
  import Modal from '$lib/ui/Modal.svelte';

  let { challenges, onRefresh } = $props();

  let showChallengeModal = $state(false);
  let editingChallenge = $state(null);
  let challengeForm = $state({
    name: '',
    default_port: '',
    priority: 0,
    flag_regex: '',
    enabled: true
  });

  function openAddChallenge() {
    editingChallenge = null;
    challengeForm = { name: '', default_port: '', priority: 0, flag_regex: '', enabled: true };
    showChallengeModal = true;
  }

  function openEditChallenge(c) {
    editingChallenge = c;
    challengeForm = {
      name: c.name,
      default_port: c.default_port ?? '',
      priority: c.priority,
      flag_regex: c.flag_regex ?? '',
      enabled: c.enabled
    };
    showChallengeModal = true;
  }

  async function saveChallenge() {
    const data = {
      name: challengeForm.name,
      default_port: challengeForm.default_port === '' ? null : Number(challengeForm.default_port),
      priority: Number(challengeForm.priority),
      flag_regex: challengeForm.flag_regex || null,
      enabled: challengeForm.enabled
    };
    if (editingChallenge) await api.updateChallenge(editingChallenge.id, data);
    else await api.createChallenge(data);

    showChallengeModal = false;
    await onRefresh();
  }

  async function deleteChallenge() {
    if (!editingChallenge) return;
    if (!confirm('Delete this challenge?')) return;

    await api.deleteChallenge(editingChallenge.id);
    showChallengeModal = false;
    await onRefresh();
  }

  function closeModal() {
    showChallengeModal = false;
  }

</script>

<div class="panel">
  <div class="panel-header">
    <h2>Challenges</h2>
    <button onclick={openAddChallenge}>+ Add Challenge</button>
  </div>
  <table>
    <thead>
      <tr>
        <th>ID</th>
        <th>Name</th>
        <th>Port</th>
        <th>Flag Regex</th>
        <th>Priority</th>
        <th>Enabled</th>
        <th></th>
      </tr>
    </thead>
    <tbody>
      {#each challenges as c}
        <tr class:disabled={!c.enabled}>
          <td>{c.id}</td>
          <td>{c.name}</td>
          <td>{c.default_port ?? '-'}</td>
          <td><code>{c.flag_regex ?? '-'}</code></td>
          <td>{c.priority}</td>
          <td>{c.enabled ? '✓' : '✗'}</td>
          <td><button class="small" onclick={() => openEditChallenge(c)}>Edit</button></td>
        </tr>
      {/each}
    </tbody>
  </table>
</div>

{#if showChallengeModal}
  <Modal onClose={closeModal}>
    <h3>{editingChallenge ? 'Edit' : 'Add'} Challenge</h3>
    <label>Name <input bind:value={challengeForm.name} /></label>
    <label>Default Port <input bind:value={challengeForm.default_port} type="number" placeholder="Optional" /></label>
    <label>Priority <input bind:value={challengeForm.priority} type="number" /></label>
    <label>Flag Regex <input bind:value={challengeForm.flag_regex} placeholder="e.g. [A-Za-z0-9]{31}=" /></label>
    <label class="checkbox"><input type="checkbox" bind:checked={challengeForm.enabled} /> Enabled</label>
    <div class="modal-actions">
      {#if editingChallenge}<button class="danger" onclick={deleteChallenge}>Delete</button>{/if}
      <button onclick={closeModal}>Cancel</button>
      <button onclick={saveChallenge}>Save</button>
    </div>
  </Modal>
{/if}
