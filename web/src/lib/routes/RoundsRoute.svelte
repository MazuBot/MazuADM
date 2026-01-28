<script>
  import { goto } from '$app/navigation'
  import { app } from '$lib/data/stores/app.js'
  import { resolveRoundSelection } from '$lib/stores/app-logic.js'
  import RoundsPage from '$lib/pages/RoundsPage.svelte'

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
      goto(`/rounds/${selectedId}`, { replaceState: true, keepFocus: true, noScroll: true })
    }
  }

  $: if ($ready && $selectedRoundId !== lastJobsRound) {
    lastJobsRound = $selectedRoundId
    loadJobs($selectedRoundId)
  }

  function selectRound(id) {
    selectedRoundId.set(id)
    if (id) goto(`/rounds/${id}`)
    else goto('/rounds')
  }

  async function newRound() {
    const id = await createRound()
    if (id) goto(`/rounds/${id}`)
  }

  async function runRoundHandler(id) {
    await runRound(id)
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
  onRefresh={loadAll}
/>
