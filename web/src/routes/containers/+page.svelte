<script>
  import { onMount } from 'svelte'
  import { app } from '$lib/data/stores/app.js'
  import { ContainersPage } from '$lib/features/containers'

  const {
    exploits,
    exploitRuns,
    challenges,
    teams,
    containers,
    containerRunners,
    loadContainers,
    loadRunners,
    resetContainers
  } = app

  let selectedChallengeId = null

  onMount(() => {
    resetContainers()
    loadContainers()
  })

  function selectChallenge(id) {
    selectedChallengeId = selectedChallengeId === id ? null : id
  }
</script>

<ContainersPage
  exploits={$exploits}
  exploitRuns={$exploitRuns}
  challenges={$challenges}
  teams={$teams}
  containers={$containers}
  selectedChallengeId={selectedChallengeId}
  onSelectChallenge={selectChallenge}
  containerRunners={$containerRunners}
  onLoadContainers={loadContainers}
  onLoadRunners={loadRunners}
/>
