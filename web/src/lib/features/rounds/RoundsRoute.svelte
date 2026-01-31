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

  $: if ($ready) {
    const { selectedId } = resolveRoundSelection({
      rounds: $rounds,
      selectedId: $selectedRoundId,
      routeId
    })

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
    selectedRoundId.set(id)
    if (id) goto(`/jobs/${id}`)
    else goto('/jobs')
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
  onNewRound={newRound}
  onRunRound={runRoundHandler}
  onRerunUnflagged={rerunUnflaggedHandler}
  onRefresh={loadAll}
/>
