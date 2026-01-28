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
  let challengeFormInitial = $state(null);
  let pendingToggleChallengeId = $state(null);
  let pendingToggleChallengeTimer = null;

  function normalizeField(value) {
    if (typeof value === 'boolean') return value;
    return value ?? '';
  }

  function isChallengeFieldChanged(field) {
    if (!challengeFormInitial) return false;
    return String(normalizeField(challengeForm[field])) !== String(normalizeField(challengeFormInitial[field]));
  }

  function openAddChallenge() {
    editingChallenge = null;
    challengeForm = { name: '', default_port: '', priority: 0, flag_regex: '', enabled: true };
    challengeFormInitial = { ...challengeForm };
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
    challengeFormInitial = { ...challengeForm };
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

  function clearPendingChallengeToggle() {
    pendingToggleChallengeId = null;
    if (pendingToggleChallengeTimer) {
      clearTimeout(pendingToggleChallengeTimer);
      pendingToggleChallengeTimer = null;
    }
  }

  function startPendingChallengeToggle(challengeId) {
    clearPendingChallengeToggle();
    pendingToggleChallengeId = challengeId;
    pendingToggleChallengeTimer = setTimeout(() => {
      pendingToggleChallengeId = null;
      pendingToggleChallengeTimer = null;
    }, 2000);
  }

  function queueChallengeToggle(challenge, e) {
    e.stopPropagation();
    if (pendingToggleChallengeId === challenge.id) {
      startPendingChallengeToggle(challenge.id);
      return;
    }
    startPendingChallengeToggle(challenge.id);
  }

  function confirmChallengeToggle(challenge, e) {
    e.stopPropagation();
    if (pendingToggleChallengeId !== challenge.id) return;
    toggleChallengeEnabled(challenge);
  }

  async function toggleChallengeEnabled(challenge) {
    clearPendingChallengeToggle();
    await api.updateChallenge(challenge.id, {
      name: challenge.name,
      default_port: challenge.default_port ?? null,
      priority: challenge.priority,
      flag_regex: challenge.flag_regex ?? null,
      enabled: !challenge.enabled
    });
    await onRefresh();
  }

</script>

<svelte:window on:click={clearPendingChallengeToggle} />

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
          <td class="enable-cell" class:pending={pendingToggleChallengeId === c.id}>
            <span class="enable-wrap">
              <button
                type="button"
                class="enable-toggle"
                aria-label={`Toggle challenge ${c.name} ${c.enabled ? 'off' : 'on'}`}
                onclick={(e) => queueChallengeToggle(c, e)}
              >
                {c.enabled ? '✓' : '✗'}
              </button>
              {#if pendingToggleChallengeId === c.id}
                <button
                  type="button"
                  class="toggle-popup small"
                  aria-label={`Confirm ${c.enabled ? 'disable' : 'enable'} ${c.name}`}
                  onclick={(e) => confirmChallengeToggle(c, e)}
                >
                  {c.enabled ? '✗' : '✓'}
                </button>
              {/if}
            </span>
          </td>
          <td><button class="small" onclick={() => openEditChallenge(c)}>Edit</button></td>
        </tr>
      {/each}
    </tbody>
  </table>
</div>

{#if showChallengeModal}
  <Modal onClose={closeModal}>
    <form onsubmit={(e) => { e.preventDefault(); saveChallenge(); }}>
      <h3>{editingChallenge ? 'Edit' : 'Add'} Challenge</h3>
      <label class:field-changed={isChallengeFieldChanged('name')}>
        Name <input bind:value={challengeForm.name} />
      </label>
      <label class:field-changed={isChallengeFieldChanged('default_port')}>
        Default Port <input bind:value={challengeForm.default_port} type="number" placeholder="Optional" />
      </label>
      <label class:field-changed={isChallengeFieldChanged('priority')}>
        Priority <input bind:value={challengeForm.priority} type="number" />
      </label>
      <label class:field-changed={isChallengeFieldChanged('flag_regex')}>
        Flag Regex <input bind:value={challengeForm.flag_regex} placeholder="e.g. [A-Za-z0-9]{31}=" />
      </label>
      <label class="checkbox" class:field-changed={isChallengeFieldChanged('enabled')}>
        <input type="checkbox" bind:checked={challengeForm.enabled} /> Enabled
      </label>
      <div class="modal-actions">
        {#if editingChallenge}<button type="button" class="danger" onclick={deleteChallenge}>Delete</button>{/if}
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
  }

  .toggle-popup:focus-visible,
  .toggle-popup:hover {
    border-color: #00d9ff;
  }
</style>
