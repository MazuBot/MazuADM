<script>
  import { onMount } from 'svelte';
  import { api } from './api.js';
  import { connect, disconnect } from './websocket.js';
  import { buildHash, parseHash } from './router.js';

  import BoardPage from './pages/BoardPage.svelte';
  import ChallengesPage from './pages/ChallengesPage.svelte';
  import ContainersPage from './pages/ContainersPage.svelte';
  import FlagsPage from './pages/FlagsPage.svelte';
  import RoundsPage from './pages/RoundsPage.svelte';
  import SettingsPage from './pages/SettingsPage.svelte';
  import TeamsPage from './pages/TeamsPage.svelte';

  let route = $state(parseHash());

  let challenges = $state([]);
  let teams = $state([]);
  let exploits = $state([]);
  let exploitRuns = $state([]);
  let rounds = $state([]);
  let jobs = $state([]);
  let flags = $state([]);
  let settings = $state([]);
  let containers = $state([]);
  let containerRunners = $state({});

  let selectedChallenge = $state(null);
  let selectedRound = $state(null);
  let selectedFlagRound = $state(null);

  function routeHref(page, id = null) {
    if (page === 'board') return buildHash('board', selectedChallenge);
    if (page === 'rounds') return buildHash('rounds', selectedRound);
    if (page === 'flags') return buildHash('flags', selectedFlagRound);
    return buildHash(page, id);
  }

  function navigate(page, id = null, { replace = false } = {}) {
    const hash = buildHash(page, id);
    if (replace) {
      history.replaceState(null, '', hash);
      route = parseHash(hash);
      applyRoute();
      return;
    }
    location.hash = hash;
  }

  function pickDefaultRound() {
    if (!rounds.length) return null;
    const nonPending = rounds.filter((r) => r.status !== 'pending');
    return nonPending.length ? nonPending[0].id : rounds[0].id;
  }

  function isValidId(list, id) {
    return id && list.some((x) => x.id === id);
  }

  function ensureSelections() {
    if (!isValidId(challenges, selectedChallenge)) selectedChallenge = challenges[0]?.id ?? null;
    if (!isValidId(rounds, selectedRound)) selectedRound = pickDefaultRound();
    if (selectedFlagRound && !isValidId(rounds, selectedFlagRound)) selectedFlagRound = null;
  }

  async function loadAll() {
    const [c, t, e, r, ro, s] = await Promise.all([
      api.challenges(),
      api.teams(),
      api.exploits(),
      api.exploitRuns(),
      api.rounds(),
      api.settings()
    ]);

    challenges = c;
    teams = t;
    exploits = e;
    exploitRuns = r;
    rounds = ro;
    settings = s;

    ensureSelections();
    applyRoute();

    connect(handleWsMessage);
  }

  async function loadJobs(roundId) {
    if (!roundId) return;
    const result = await api.jobs(roundId);
    if (roundId === selectedRound) jobs = result;
  }

  async function loadFlags(roundId) {
    flags = await api.flags(roundId);
  }

  async function loadContainers() {
    containers = await api.containers();
  }

  async function loadRunners(containerId) {
    containerRunners[containerId] = await api.containerRunners(containerId);
  }

  function applyRoute() {
    ensureSelections();

    if (route.page === 'board') {
      if (isValidId(challenges, route.id)) selectedChallenge = route.id;
      else if (!selectedChallenge && challenges.length) selectedChallenge = challenges[0].id;

      if (selectedChallenge) {
        const desired = buildHash('board', selectedChallenge);
        if (location.hash !== desired) navigate('board', selectedChallenge, { replace: true });
      }
      return;
    }

    if (route.page === 'rounds') {
      if (isValidId(rounds, route.id)) selectedRound = route.id;
      else if (!selectedRound) selectedRound = pickDefaultRound();

      if (selectedRound) {
        const desired = buildHash('rounds', selectedRound);
        if (location.hash !== desired) navigate('rounds', selectedRound, { replace: true });
        loadJobs(selectedRound);
      }
      return;
    }

    if (route.page === 'flags') {
      selectedFlagRound = isValidId(rounds, route.id) ? route.id : null;
      const desired = buildHash('flags', selectedFlagRound);
      if (location.hash !== desired) navigate('flags', selectedFlagRound, { replace: true });
      loadFlags(selectedFlagRound);
      return;
    }

    if (route.page === 'containers') {
      loadContainers();
      return;
    }
  }

  async function selectChallenge(id) {
    selectedChallenge = id;
    navigate('board', id);
  }

  async function selectRound(id) {
    selectedRound = id;
    navigate('rounds', id);
  }

  async function selectFlagRound(id) {
    selectedFlagRound = id;
    navigate('flags', id);
  }

  async function newRound() {
    const id = await api.createRound();
    selectedRound = id;
    jobs = [];
    navigate('rounds', id);
  }

  async function runRound(id) {
    if (!id) return;
    await api.runRound(id);
  }

  function handleWsMessage(msg) {
    const { type, data } = msg;
    switch (type) {
      case 'challenge_created':
        challenges = [...challenges, data];
        break;
      case 'challenge_updated':
        challenges = challenges.map((c) => (c.id === data.id ? data : c));
        break;
      case 'challenge_deleted':
        challenges = challenges.filter((c) => c.id !== data);
        break;
      case 'team_created':
        teams = [...teams, data];
        break;
      case 'team_updated':
        teams = teams.map((t) => (t.id === data.id ? data : t));
        break;
      case 'team_deleted':
        teams = teams.filter((t) => t.id !== data);
        break;
      case 'exploit_created':
        exploits = [...exploits, data];
        break;
      case 'exploit_updated':
        exploits = exploits.map((e) => (e.id === data.id ? data : e));
        break;
      case 'exploit_deleted':
        exploits = exploits.filter((e) => e.id !== data);
        break;
      case 'exploit_run_created':
        exploitRuns = [...exploitRuns, data];
        break;
      case 'exploit_run_updated':
        exploitRuns = exploitRuns.map((r) => (r.id === data.id ? data : r));
        break;
      case 'exploit_run_deleted':
        exploitRuns = exploitRuns.filter((r) => r.id !== data);
        break;
      case 'exploit_runs_reordered':
        loadAll();
        break;
      case 'round_created':
        rounds = [data, ...rounds];
        break;
      case 'round_updated':
        rounds = rounds.map((r) => (r.id === data.id ? data : r));
        break;
      case 'job_updated':
        if (data.round_id === selectedRound) {
          const idx = jobs.findIndex((j) => j.id === data.id);
          if (idx >= 0) jobs = [...jobs.slice(0, idx), data, ...jobs.slice(idx + 1)];
          else jobs = [...jobs, data];
        }
        break;
      case 'flag_created':
        if (!selectedFlagRound || data.round_id === selectedFlagRound) {
          flags = [...flags, data];
        }
        break;
      case 'setting_updated':
        settings = settings.map((s) => (s.key === data.key ? { ...s, value: data.value } : s));
        break;
      case 'relation_updated':
        break;
    }
  }

  onMount(() => {
    if (!location.hash || location.hash === '#') {
      history.replaceState(null, '', buildHash('board'));
    }

    route = parseHash();
    applyRoute();

    const onHashChange = () => {
      route = parseHash();
      applyRoute();
    };
    window.addEventListener('hashchange', onHashChange);

    loadAll();

    return () => {
      window.removeEventListener('hashchange', onHashChange);
      disconnect();
    };
  });

</script>

<main>
  <header>
    <h1>MazuADM</h1>
    <nav>
      <a class:active={route.page === 'board'} href={routeHref('board')}>Board</a>
      <a class:active={route.page === 'challenges'} href={routeHref('challenges')}>Challenges</a>
      <a class:active={route.page === 'teams'} href={routeHref('teams')}>Teams</a>
      <a class:active={route.page === 'rounds'} href={routeHref('rounds')}>Rounds</a>
      <a class:active={route.page === 'flags'} href={routeHref('flags')}>Flags</a>
      <a class:active={route.page === 'containers'} href={routeHref('containers')}>Containers</a>
      <a class:active={route.page === 'settings'} href={routeHref('settings')}>Settings</a>
    </nav>
  </header>

  {#if route.page === 'board'}
    <BoardPage
      {challenges}
      {teams}
      {exploits}
      {exploitRuns}
      selectedChallengeId={selectedChallenge}
      onSelectChallenge={selectChallenge}
      onRefresh={loadAll}
    />
  {:else if route.page === 'challenges'}
    <ChallengesPage {challenges} onRefresh={loadAll} />
  {:else if route.page === 'teams'}
    <TeamsPage {teams} onRefresh={loadAll} />
  {:else if route.page === 'rounds'}
    <RoundsPage
      {rounds}
      {jobs}
      {teams}
      {challenges}
      {exploits}
      {exploitRuns}
      selectedRoundId={selectedRound}
      onSelectRound={selectRound}
      onNewRound={newRound}
      onRunRound={runRound}
    />
  {:else if route.page === 'flags'}
    <FlagsPage
      {rounds}
      {flags}
      {teams}
      {challenges}
      selectedFlagRoundId={selectedFlagRound}
      onSelectFlagRound={selectFlagRound}
    />
  {:else if route.page === 'containers'}
    <ContainersPage
      {exploits}
      {exploitRuns}
      {teams}
      {containers}
      {containerRunners}
      onLoadContainers={loadContainers}
      onLoadRunners={loadRunners}
    />
  {:else if route.page === 'settings'}
    <SettingsPage {settings} onRefresh={loadAll} />
  {/if}
</main>
