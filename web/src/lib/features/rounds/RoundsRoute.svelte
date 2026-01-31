<script>
  import { goto } from '$app/navigation'
  import { app } from '$lib/data/stores/app.js'
  import { resolveRoundSelection } from '$lib/stores/app-logic.js'
  import RoundsPage from './RoundsPage.svelte'

  export let routeId = null

  const {
    ready,
    rounds,
    jobs,
    teams,
    challenges,
    exploits,
    exploitRuns,
    selectedRoundId,
    createRound,
    runRound,
    rerunUnflaggedRound,
    loadJobs,
    loadAll
  } = app

  let lastJobsRound = null
  let followRunning = false

  $: if ($ready) {
    let { selectedId } = resolveRoundSelection({
      rounds: $rounds,
      selectedId: $selectedRoundId,
      routeId
    })

    if (followRunning) {
      const runningId = $rounds.find((round) => round.status === 'running')?.id ?? null
      if (runningId) selectedId = runningId
    }

    if (selectedId !== $selectedRoundId) selectedRoundId.set(selectedId)
    if (selectedId && routeId !== selectedId) {
      goto(`/jobs/${selectedId}`, { replaceState: true, keepFocus: true, noScroll: true })
    }
  }

  $: if ($ready && $selectedRoundId !== lastJobsRound) {
    lastJobsRound = $selectedRoundId
    loadJobs($selectedRoundId)
  }

  function selectRound(id) {
    followRunning = false
    selectedRoundId.set(id)
    if (id) goto(`/jobs/${id}`)
    else goto('/jobs')
  }

  function toggleFollowRunning() {
    followRunning = !followRunning
  }

  async function newRound(target) {
    const id = await createRound(target)
    if (id) goto(`/jobs/${id}`)
    return id
  }

  async function runRoundHandler(id) {
    await runRound(id)
  }

  async function rerunUnflaggedHandler(id) {
    await rerunUnflaggedRound(id)
  }
</script>

<RoundsPage
  rounds={$rounds}
  jobs={$jobs}
  teams={$teams}
  challenges={$challenges}
  exploits={$exploits}
  exploitRuns={$exploitRuns}
  selectedRoundId={$selectedRoundId}
  onSelectRound={selectRound}
  followRunning={followRunning}
  onToggleFollow={toggleFollowRunning}
  onNewRound={newRound}
  onRunRound={runRoundHandler}
  onRerunUnflagged={rerunUnflaggedHandler}
  onRefresh={loadAll}
/>
