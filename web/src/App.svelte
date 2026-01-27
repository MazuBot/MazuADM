<script>
  import { api } from './api.js';
  import Board from './Board.svelte';

  let challenges = $state([]);
  let teams = $state([]);
  let exploits = $state([]);
  let exploitRuns = $state([]);
  let rounds = $state([]);
  let selectedChallenge = $state(null);
  let selectedRound = $state(null);
  let jobs = $state([]);
  let flags = $state([]);
  let tab = $state('board');

  // Challenge form
  let showChallengeModal = $state(false);
  let editingChallenge = $state(null);
  let challengeForm = $state({ name: '', default_port: '', priority: 0, flag_regex: '', enabled: true });

  // Team form
  let showTeamModal = $state(false);
  let editingTeam = $state(null);
  let teamForm = $state({ team_id: '', team_name: '', default_ip: '', priority: 0 });

  async function load() {
    challenges = await api.challenges();
    teams = await api.teams();
    exploits = await api.exploits();
    exploitRuns = await api.exploitRuns();
    rounds = await api.rounds();
    if (challenges.length && !selectedChallenge) selectedChallenge = challenges[0].id;
  }

  async function loadJobs() {
    if (selectedRound) {
      jobs = await api.jobs(selectedRound);
      flags = await api.flags(selectedRound);
    }
  }

  async function newRound() {
    const id = await api.createRound();
    await load();
    selectedRound = id;
    await loadJobs();
  }

  async function runRound() {
    if (selectedRound) {
      await api.runRound(selectedRound);
      pollJobs();
    }
  }

  function pollJobs() {
    const interval = setInterval(async () => {
      await loadJobs();
      if (jobs.every(j => j.status !== 'pending' && j.status !== 'running')) clearInterval(interval);
    }, 1000);
  }

  // Challenge CRUD
  function openAddChallenge() {
    editingChallenge = null;
    challengeForm = { name: '', default_port: '', priority: 0, flag_regex: '', enabled: true };
    showChallengeModal = true;
  }

  function openEditChallenge(c) {
    editingChallenge = c;
    challengeForm = { name: c.name, default_port: c.default_port ?? '', priority: c.priority, flag_regex: c.flag_regex ?? '', enabled: c.enabled };
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
    if (editingChallenge) {
      await api.updateChallenge(editingChallenge.id, data);
    } else {
      await api.createChallenge(data);
    }
    showChallengeModal = false;
    load();
  }

  async function deleteChallenge() {
    if (confirm('Delete this challenge?')) {
      await api.deleteChallenge(editingChallenge.id);
      showChallengeModal = false;
      load();
    }
  }

  // Team CRUD
  function openAddTeam() {
    editingTeam = null;
    teamForm = { team_id: '', team_name: '', default_ip: '', priority: 0 };
    showTeamModal = true;
  }

  function openEditTeam(t) {
    editingTeam = t;
    teamForm = { team_id: t.team_id, team_name: t.team_name, default_ip: t.default_ip ?? '', priority: t.priority };
    showTeamModal = true;
  }

  async function saveTeam() {
    const data = {
      team_id: teamForm.team_id,
      team_name: teamForm.team_name,
      default_ip: teamForm.default_ip || null,
      priority: Number(teamForm.priority)
    };
    if (editingTeam) {
      await api.updateTeam(editingTeam.id, data);
    } else {
      await api.createTeam(data);
    }
    showTeamModal = false;
    load();
  }

  async function deleteTeam() {
    if (confirm('Delete this team?')) {
      await api.deleteTeam(editingTeam.id);
      showTeamModal = false;
      load();
    }
  }

  $effect(() => { load(); });
  $effect(() => { if (selectedRound) loadJobs(); });
</script>

<main>
  <header>
    <h1>MazuADM</h1>
    <nav>
      <button class:active={tab === 'board'} onclick={() => tab = 'board'}>Board</button>
      <button class:active={tab === 'challenges'} onclick={() => tab = 'challenges'}>Challenges</button>
      <button class:active={tab === 'teams'} onclick={() => tab = 'teams'}>Teams</button>
      <button class:active={tab === 'rounds'} onclick={() => tab = 'rounds'}>Rounds</button>
      <button class:active={tab === 'flags'} onclick={() => tab = 'flags'}>Flags</button>
    </nav>
  </header>

  {#if tab === 'board'}
    <div class="challenge-tabs">
      {#each challenges as c}
        <button class:active={selectedChallenge === c.id} onclick={() => selectedChallenge = c.id}>{c.name}</button>
      {/each}
    </div>
    <Board {teams} {exploits} {exploitRuns} challengeId={selectedChallenge} onRefresh={load} />

  {:else if tab === 'challenges'}
    <div class="panel">
      <div class="panel-header">
        <h2>Challenges</h2>
        <button onclick={openAddChallenge}>+ Add Challenge</button>
      </div>
      <table>
        <thead><tr><th>ID</th><th>Name</th><th>Port</th><th>Priority</th><th>Enabled</th><th></th></tr></thead>
        <tbody>
          {#each challenges as c}
            <tr class:disabled={!c.enabled}>
              <td>{c.id}</td>
              <td>{c.name}</td>
              <td>{c.default_port ?? '-'}</td>
              <td>{c.priority}</td>
              <td>{c.enabled ? '✓' : '✗'}</td>
              <td><button class="small" onclick={() => openEditChallenge(c)}>Edit</button></td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>

  {:else if tab === 'teams'}
    <div class="panel">
      <div class="panel-header">
        <h2>Teams</h2>
        <button onclick={openAddTeam}>+ Add Team</button>
      </div>
      <table>
        <thead><tr><th>ID</th><th>Team ID</th><th>Name</th><th>Default IP</th><th>Priority</th><th></th></tr></thead>
        <tbody>
          {#each teams as t}
            <tr>
              <td>{t.id}</td>
              <td>{t.team_id}</td>
              <td>{t.team_name}</td>
              <td>{t.default_ip ?? '-'}</td>
              <td>{t.priority}</td>
              <td><button class="small" onclick={() => openEditTeam(t)}>Edit</button></td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>

  {:else if tab === 'rounds'}
    <div class="rounds-panel">
      <div class="controls">
        <button onclick={newRound}>New Round</button>
        <select bind:value={selectedRound}>
          <option value={null}>Select round</option>
          {#each rounds as r}
            <option value={r.id}>Round {r.id} ({r.status})</option>
          {/each}
        </select>
        <button onclick={runRound} disabled={!selectedRound}>Run</button>
      </div>
      {#if jobs.length}
        <table>
          <thead><tr><th>ID</th><th>Run</th><th>Team</th><th>Priority</th><th>Status</th></tr></thead>
          <tbody>
            {#each jobs as j}
              <tr class={j.status}><td>{j.id}</td><td>{j.exploit_run_id}</td><td>{j.team_id}</td><td>{j.priority}</td><td>{j.status}</td></tr>
            {/each}
          </tbody>
        </table>
      {/if}
    </div>

  {:else if tab === 'flags'}
    <table>
      <thead><tr><th>ID</th><th>Round</th><th>Challenge</th><th>Team</th><th>Flag</th><th>Status</th></tr></thead>
      <tbody>
        {#each flags as f}
          <tr><td>{f.id}</td><td>{f.round_id}</td><td>{f.challenge_id}</td><td>{f.team_id}</td><td><code>{f.flag_value}</code></td><td>{f.status}</td></tr>
        {/each}
      </tbody>
    </table>
  {/if}
</main>

{#if showChallengeModal}
  <div class="modal-overlay" onclick={() => showChallengeModal = false}>
    <div class="modal" onclick={(e) => e.stopPropagation()}>
      <h3>{editingChallenge ? 'Edit' : 'Add'} Challenge</h3>
      <label>Name <input bind:value={challengeForm.name} /></label>
      <label>Default Port <input bind:value={challengeForm.default_port} type="number" placeholder="Optional" /></label>
      <label>Priority <input bind:value={challengeForm.priority} type="number" /></label>
      <label>Flag Regex <input bind:value={challengeForm.flag_regex} placeholder="e.g. [A-Za-z0-9]{31}=" /></label>
      <label class="checkbox"><input type="checkbox" bind:checked={challengeForm.enabled} /> Enabled</label>
      <div class="modal-actions">
        {#if editingChallenge}<button class="danger" onclick={deleteChallenge}>Delete</button>{/if}
        <button onclick={() => showChallengeModal = false}>Cancel</button>
        <button onclick={saveChallenge}>Save</button>
      </div>
    </div>
  </div>
{/if}

{#if showTeamModal}
  <div class="modal-overlay" onclick={() => showTeamModal = false}>
    <div class="modal" onclick={(e) => e.stopPropagation()}>
      <h3>{editingTeam ? 'Edit' : 'Add'} Team</h3>
      <label>Team ID <input bind:value={teamForm.team_id} disabled={!!editingTeam} /></label>
      <label>Team Name <input bind:value={teamForm.team_name} /></label>
      <label>Default IP <input bind:value={teamForm.default_ip} placeholder="Optional" /></label>
      <label>Priority <input bind:value={teamForm.priority} type="number" /></label>
      <div class="modal-actions">
        {#if editingTeam}<button class="danger" onclick={deleteTeam}>Delete</button>{/if}
        <button onclick={() => showTeamModal = false}>Cancel</button>
        <button onclick={saveTeam}>Save</button>
      </div>
    </div>
  </div>
{/if}

<style>
  :global(body) { margin: 0; font-family: system-ui; background: #1a1a2e; color: #eee; }
  main { max-width: 1400px; margin: 0 auto; padding: 1rem; }
  header { display: flex; justify-content: space-between; align-items: center; border-bottom: 1px solid #333; margin-bottom: 1rem; }
  h1 { margin: 0; color: #00d9ff; }
  h2 { margin: 0; color: #eee; }
  nav button, .challenge-tabs button { background: #252540; border: none; color: #aaa; padding: 0.5rem 1rem; cursor: pointer; margin-right: 0.25rem; }
  nav button.active, .challenge-tabs button.active { background: #00d9ff; color: #000; }
  .controls { display: flex; gap: 0.5rem; margin-bottom: 1rem; }
  button { background: #00d9ff; border: none; color: #000; padding: 0.5rem 1rem; cursor: pointer; border-radius: 4px; }
  button:disabled { opacity: 0.5; }
  button.small { padding: 0.25rem 0.5rem; font-size: 0.85rem; }
  button.danger { background: #d9534f; }
  select { background: #252540; color: #eee; border: 1px solid #444; padding: 0.5rem; }
  table { width: 100%; border-collapse: collapse; }
  th, td { padding: 0.5rem; text-align: left; border-bottom: 1px solid #333; }
  tr.success { background: #1a3a1a; }
  tr.failed, tr.error { background: #3a1a1a; }
  tr.running { background: #3a3a1a; }
  tr.disabled { opacity: 0.5; }
  code { background: #333; padding: 0.2rem 0.4rem; border-radius: 3px; }
  .panel { background: #252540; padding: 1rem; border-radius: 8px; }
  .panel-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem; }
  .modal-overlay { position: fixed; inset: 0; background: rgba(0,0,0,0.7); display: flex; align-items: center; justify-content: center; z-index: 100; }
  .modal { background: #252540; padding: 1.5rem; border-radius: 8px; min-width: 320px; }
  .modal h3 { margin-top: 0; }
  .modal label { display: block; margin-bottom: 0.75rem; color: #aaa; font-size: 0.9rem; }
  .modal label input { display: block; width: 100%; padding: 0.5rem; margin-top: 0.25rem; background: #1a1a2e; border: 1px solid #444; color: #eee; border-radius: 4px; box-sizing: border-box; }
  .modal label input:disabled { opacity: 0.5; }
  .modal .checkbox { display: flex; align-items: center; gap: 0.5rem; flex-direction: row; }
  .modal .checkbox input { display: inline; width: auto; margin: 0; }
  .modal-actions { display: flex; gap: 0.5rem; justify-content: flex-end; margin-top: 1rem; }
</style>
