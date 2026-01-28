<script>
  import { onMount } from 'svelte'
  import { app } from '$lib/stores/app.js'
  import ContainersPage from '$lib/pages/ContainersPage.svelte'

  const {
    exploits,
    exploitRuns,
    teams,
    containers,
    containerRunners,
    loadContainers,
    loadRunners,
    resetContainers
  } = app

  onMount(() => {
    resetContainers()
    loadContainers()
  })
</script>

<ContainersPage
  exploits={$exploits}
  exploitRuns={$exploitRuns}
  teams={$teams}
  containers={$containers}
  containerRunners={$containerRunners}
  onLoadContainers={loadContainers}
  onLoadRunners={loadRunners}
/>
