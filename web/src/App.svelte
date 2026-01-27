<script>
  import { api } from './api.js';
  import Board from './Board.svelte';
  import { AnsiUp } from 'ansi_up';

  const ansi_up = new AnsiUp();
  function renderAnsi(text) {
    return ansi_up.ansi_to_html(text || '');
  }

  let challenges = $state([]);
  let teams = $state([]);
  let exploits = $state([]);
  let exploitRuns = $state([]);
  let rounds = $state([]);
  let selectedChallenge = $state(null);
  let selectedRound = $state(null);
  let selectedFlagRound = $state(null);
  let jobs = $state([]);
  let flags = $state([]);
  let settings = $state([]);
  let containers = $state([]);
  let containerRunners = $state({});

  // Parse hash: #tab or #tab/id
  function parseHash() {
    const h = location.hash.slice(1);
    const [t, id] = h.split('/');
    return { tab: t || 'board', id: id ? parseInt(id) : null };
  }

  let tab = $state(parseHash().tab);

  function setTab(t) {
    tab = t;
    history.pushState(null, '', '#' + t);
  }

  function updateHash() {
    let h = tab;
    if (tab === 'board' && selectedChallenge) h = `board/${selectedChallenge}`;
    else if (tab === 'rounds' && selectedRound) h = `rounds/${selectedRound}`;
    else if (tab === 'flags' && selectedFlagRound) h = `flags/${selectedFlagRound}`;
    history.replaceState(null, '', '#' + h);
  }

  function applyHash() {
    const { tab: t, id } = parseHash();
    tab = t;
    if (t === 'board' && id) selectedChallenge = id;
    else if (t === 'rounds' && id) { selectedRound = id; loadJobs(); }
    else if (t === 'flags' && id) { selectedFlagRound = id; loadFlags(); }
  }

  window.addEventListener('popstate', applyHash);

  // Challenge form
  let showChallengeModal = $state(false);
  let editingChallenge = $state(null);
  let challengeForm = $state({ name: '', default_port: '', priority: 0, flag_regex: '', enabled: true });

  // Team form
  let showTeamModal = $state(false);
  let editingTeam = $state(null);
  let teamForm = $state({ team_id: '', team_name: '', default_ip: '', priority: 0, enabled: true });

  async function load() {
    challenges = await api.challenges();
    teams = await api.teams();
    exploits = await api.exploits();
    exploitRuns = await api.exploitRuns();
    rounds = await api.rounds();
    settings = await api.settings();
    
    const { tab: t, id } = parseHash();
    if (t === 'board' && id && challenges.find(c => c.id === id)) selectedChallenge = id;
    else if (challenges.length && !selectedChallenge) selectedChallenge = challenges[0].id;
    
    if (t === 'rounds' && id && rounds.find(r => r.id === id)) { selectedRound = id; loadJobs(); }
    else if (rounds.length && !selectedRound) {
      const nonPending = rounds.filter(r => r.status !== 'pending');
      selectedRound = nonPending.length ? nonPending[0].id : rounds[0].id;
      loadJobs();
    }

    if (t === 'flags' && id && rounds.find(r => r.id === id)) { selectedFlagRound = id; }
    loadFlags();
  }

  async function loadJobs() {
    if (selectedRound) {
      jobs = await api.jobs(selectedRound);
      updateHash();
    }
  }

  async function loadFlags() {
    flags = await api.flags(selectedFlagRound);
    updateHash();
  }

  function selectChallenge(id) {
    selectedChallenge = id;
    updateHash();
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
    teamForm = { team_id: '', team_name: '', default_ip: '', priority: 0, enabled: true };
    showTeamModal = true;
  }

  function openEditTeam(t) {
    editingTeam = t;
    teamForm = { team_id: t.team_id, team_name: t.team_name, default_ip: t.default_ip ?? '', priority: t.priority, enabled: t.enabled };
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

  // Job detail modal
  let selectedJob = $state(null);

  function getTeamName(teamId) {
    const t = teams.find(t => t.id === teamId);
    return t ? `${t.id} (${t.team_name})` : teamId;
  }

  function getChallengeName(challengeId) {
    const c = challenges.find(c => c.id === challengeId);
    return c ? c.name : challengeId;
  }

  function getExploitName(exploitId) {
    const e = exploits.find(e => e.id === exploitId);
    return e ? e.name : exploitId;
  }

  function getExploitRunInfo(runId) {
    return exploitRuns.find(r => r.id === runId);
  }

  function getExploitRunName(runId) {
    const run = exploitRuns.find(r => r.id === runId);
    return run ? getExploitName(run.exploit_id) : runId;
  }

  async function loadContainers() {
    containers = await api.containers();
  }

  async function loadRunners(containerId) {
    containerRunners[containerId] = await api.containerRunners(containerId);
  }

  $effect(() => { load(); });
  $effect(() => { if (selectedRound) loadJobs(); });
  $effect(() => { if (tab === 'containers') loadContainers(); });
</script>

<main>
  <header>
    <h1>MazuADM</h1>
    <nav>
      <button class:active={tab === 'board'} onclick={() => setTab('board')}>Board</button>
      <button class:active={tab === 'challenges'} onclick={() => setTab('challenges')}>Challenges</button>
      <button class:active={tab === 'teams'} onclick={() => setTab('teams')}>Teams</button>
      <button class:active={tab === 'rounds'} onclick={() => setTab('rounds')}>Rounds</button>
      <button class:active={tab === 'flags'} onclick={() => setTab('flags')}>Flags</button>
      <button class:active={tab === 'containers'} onclick={() => setTab('containers')}>Containers</button>
      <button class:active={tab === 'settings'} onclick={() => setTab('settings')}>Settings</button>
    </nav>
  </header>

  {#if tab === 'board'}
    <div class="challenge-tabs">
      {#each challenges as c}
        <button class:active={selectedChallenge === c.id} onclick={() => selectChallenge(c.id)}>{c.name}</button>
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
        <thead><tr><th>ID</th><th>Name</th><th>Port</th><th>Flag Regex</th><th>Priority</th><th>Enabled</th><th></th></tr></thead>
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

  {:else if tab === 'teams'}
    <div class="panel">
      <div class="panel-header">
        <h2>Teams</h2>
        <button onclick={openAddTeam}>+ Add Team</button>
      </div>
      <table>
        <thead><tr><th>ID</th><th>Team ID</th><th>Name</th><th>Default IP</th><th>Priority</th><th>Enabled</th><th></th></tr></thead>
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

  {:else if tab === 'rounds'}
    <div class="rounds-panel">
      <div class="controls">
        <button onclick={newRound}>New Round</button>
        <select bind:value={selectedRound} onchange={() => { loadJobs(); updateHash(); }}>
          <option value={null}>Select round</option>
          {#each rounds as r}
            <option value={r.id}>Round {r.id} ({r.status})</option>
          {/each}
        </select>
        <button onclick={runRound} disabled={!selectedRound}>Run</button>
      </div>
      {#if jobs.length}
        <table>
          <thead><tr><th>ID</th><th>Challenge</th><th>Exploit</th><th>Team</th><th>Container</th><th>Priority</th><th>Status</th><th>Duration</th></tr></thead>
          <tbody>
            {#each [...jobs].sort((a, b) => b.priority - a.priority || a.id - b.id) as j}
              <tr class={j.status} onclick={() => selectedJob = j} style="cursor:pointer">
                <td>{j.id}</td>
                <td>{getChallengeName(getExploitRunInfo(j.exploit_run_id)?.challenge_id)}</td>
                <td>{getExploitName(getExploitRunInfo(j.exploit_run_id)?.exploit_id)}</td>
                <td>{getTeamName(j.team_id)}</td>
                <td>{j.container_id ? j.container_id.slice(0, 12) : '-'}</td>
                <td>{j.priority}</td>
                <td>{j.status === 'flag' ? '🚩 FLAG' : j.status}</td>
                <td>{j.duration_ms ? `${j.duration_ms}ms` : '-'}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}
    </div>

  {:else if tab === 'flags'}
    <div class="controls">
      <select bind:value={selectedFlagRound} onchange={() => loadFlags()}>
        <option value={null}>All rounds</option>
        {#each rounds as r}
          <option value={r.id}>Round {r.id}</option>
        {/each}
      </select>
    </div>
    <table>
      <thead><tr><th>ID</th><th>Round</th><th>Challenge</th><th>Team</th><th>Flag</th><th>Status</th></tr></thead>
      <tbody>
        {#each flags as f}
          <tr><td>{f.id}</td><td>{f.round_id}</td><td>{getChallengeName(f.challenge_id)}</td><td>{getTeamName(f.team_id)}</td><td><code>{f.flag_value}</code></td><td>{f.status}</td></tr>
        {/each}
      </tbody>
    </table>

  {:else if tab === 'settings'}
    <div class="panel">
      <h2>Settings</h2>
      <div class="settings-grid">
        <div class="setting-row">
          <label>concurrent_limit</label>
          <input value={settings.find(s => s.key === 'concurrent_limit')?.value || '10'} 
                 onchange={(e) => api.updateSetting('concurrent_limit', e.target.value).then(load)} />
        </div>
        <div class="setting-row">
          <label>worker_timeout</label>
          <input value={settings.find(s => s.key === 'worker_timeout')?.value || '60'} 
                 onchange={(e) => api.updateSetting('worker_timeout', e.target.value).then(load)} />
        </div>
        <div class="setting-row">
          <label>max_flags_per_job</label>
          <input value={settings.find(s => s.key === 'max_flags_per_job')?.value || '50'} 
                 onchange={(e) => api.updateSetting('max_flags_per_job', e.target.value).then(load)} />
        </div>
        <div class="setting-row">
          <label>skip_on_flag</label>
          <select onchange={(e) => api.updateSetting('skip_on_flag', e.target.value).then(load)}>
            <option value="false" selected={settings.find(s => s.key === 'skip_on_flag')?.value !== 'true'}>No</option>
            <option value="true" selected={settings.find(s => s.key === 'skip_on_flag')?.value === 'true'}>Yes</option>
          </select>
        </div>
        <div class="setting-row">
          <label>sequential_per_target</label>
          <select onchange={(e) => api.updateSetting('sequential_per_target', e.target.value).then(load)}>
            <option value="false" selected={settings.find(s => s.key === 'sequential_per_target')?.value !== 'true'}>No</option>
            <option value="true" selected={settings.find(s => s.key === 'sequential_per_target')?.value === 'true'}>Yes</option>
          </select>
        </div>
      </div>
      <p class="hint">skip_on_flag: Skip remaining exploits for a chal/team once a flag is found in this round.</p>
      <p class="hint">sequential_per_target: Run exploits sequentially per chal/team (don't run multiple exploits for same target at once).</p>
    </div>

  {:else if tab === 'containers'}
    <div class="panel">
      <div class="panel-header">
        <h2>Containers</h2>
      </div>
      {#each exploits as exploit}
        {@const expContainers = containers.filter(c => c.exploit_id === exploit.id)}
        {#if expContainers.length}
          <h3>{exploit.name}</h3>
          <table>
            <thead><tr><th>ID</th><th>Container</th><th>Status</th><th>Counter</th><th>Runners</th><th>Actions</th></tr></thead>
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
                      <button onclick={() => loadRunners(c.id)}>Load</button>
                    {/if}
                  </td>
                  <td>
                    <button onclick={() => api.restartContainer(c.id).then(loadContainers)}>Restart</button>
                    <button onclick={() => api.deleteContainer(c.id).then(loadContainers)}>Remove</button>
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        {/if}
      {/each}
    </div>
  {/if}
</main>

{#if selectedJob}
  <div class="modal-overlay" onclick={() => selectedJob = null}>
    <div class="modal wide" onclick={(e) => e.stopPropagation()}>
      <h3>Job #{selectedJob.id} - {selectedJob.status}</h3>
      <div class="job-info">
        <p><strong>Exploit:</strong> {getExploitName(getExploitRunInfo(selectedJob.exploit_run_id)?.exploit_id)}</p>
        <p><strong>Team:</strong> {getTeamName(selectedJob.team_id)}</p>
        <p><strong>Priority:</strong> {selectedJob.priority}</p>
        <p><strong>Duration:</strong> {selectedJob.duration_ms ? `${selectedJob.duration_ms}ms` : '-'}</p>
        {#if selectedJob.container_id}<p><strong>Container:</strong> <code>{selectedJob.container_id.slice(0, 12)}</code></p>{/if}
      </div>
      {#if selectedJob.stdout}
        <label>Stdout</label>
        <pre class="log-output">{@html renderAnsi(selectedJob.stdout)}</pre>
      {/if}
      {#if selectedJob.stderr}
        <label>Stderr</label>
        <pre class="log-output stderr">{@html renderAnsi(selectedJob.stderr)}</pre>
      {/if}
      <div class="modal-actions">
        <button onclick={() => selectedJob = null}>Close</button>
      </div>
    </div>
  </div>
{/if}

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
      <label class="checkbox"><input type="checkbox" bind:checked={teamForm.enabled} /> Enabled</label>
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
  tr.flag { background: #2a4a2a; }
  tr.failed, tr.error, tr.timeout, tr.ole { background: #3a1a1a; }
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
  .modal.wide { min-width: 500px; max-width: 700px; }
  .job-info { background: #1a1a2e; padding: 0.75rem; border-radius: 4px; margin-bottom: 1rem; }
  .job-info p { margin: 0.25rem 0; }
  .log-output { background: #0d0d15; padding: 0.75rem; border-radius: 4px; font-size: 0.85rem; max-height: 200px; overflow: auto; white-space: pre-wrap; word-break: break-all; margin: 0.25rem 0 1rem 0; }
  .log-output.stderr { border-left: 3px solid #d9534f; }
  .settings-grid { display: flex; flex-direction: column; gap: 0.75rem; margin: 1rem 0; }
  .setting-row { display: flex; align-items: center; gap: 1rem; }
  .setting-row label { min-width: 150px; color: #aaa; }
  .setting-row input { flex: 1; max-width: 200px; padding: 0.5rem; background: #1a1a2e; border: 1px solid #444; color: #eee; border-radius: 4px; }
  .hint { color: #666; font-size: 0.85rem; margin-top: 1rem; }
</style>
