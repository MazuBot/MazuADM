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
    loadContainers
  } = app

  let selectedChallengeId = null

  onMount(() => {
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
  onLoadContainers={loadContainers}
/>
