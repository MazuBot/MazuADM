<script>
  import { goto } from '$app/navigation'
  import { app } from '$lib/data/stores/app.js'
  import { resolveBoardSelection } from '$lib/stores/app-logic.js'
  import BoardPage from './BoardPage.svelte'

  export let routeId = null

  const { ready, challenges, teams, exploits, exploitRuns, selectedChallengeId, loadAll } = app

  $: if ($ready) {
    const { selectedId } = resolveBoardSelection({
      challenges: $challenges,
      selectedId: $selectedChallengeId,
      routeId
    })

    if (selectedId !== $selectedChallengeId) selectedChallengeId.set(selectedId)
    if (selectedId && routeId !== selectedId) {
      const hash = typeof window !== 'undefined' ? window.location.hash : ''
      goto(`/board/${selectedId}${hash}`, { replaceState: true, keepFocus: true, noScroll: true })
    }
  }

  function selectChallenge(id) {
    selectedChallengeId.set(id)
    const hash = typeof window !== 'undefined' ? window.location.hash : ''
    goto(`/board/${id}${hash}`)
  }
</script>

<BoardPage
  challenges={$challenges}
  teams={$teams}
  exploits={$exploits}
  exploitRuns={$exploitRuns}
  selectedChallengeId={$selectedChallengeId}
  onSelectChallenge={selectChallenge}
  onRefresh={loadAll}
/>
