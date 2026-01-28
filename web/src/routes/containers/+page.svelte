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

  onMount(() => {
    resetContainers()
    loadContainers()
  })
</script>

<ContainersPage
  exploits={$exploits}
  exploitRuns={$exploitRuns}
  challenges={$challenges}
  teams={$teams}
  containers={$containers}
  containerRunners={$containerRunners}
  onLoadContainers={loadContainers}
  onLoadRunners={loadRunners}
/>
