<script>
  import { goto } from '$app/navigation'
  import { app } from '$lib/data/stores/app.js'
  import { resolveFlagSelection } from '$lib/stores/app-logic.js'
  import FlagsPage from './FlagsPage.svelte'

  export let routeId = null

  const { ready, rounds, flags, teams, challenges, selectedFlagRoundId, loadFlags, submitFlag } = app

  let lastFlagsRound = undefined

  $: if ($ready) {
    const { selectedId } = resolveFlagSelection({
      rounds: $rounds,
      routeId
    })

    if (selectedId !== $selectedFlagRoundId) selectedFlagRoundId.set(selectedId)
    if (routeId && !selectedId) {
      goto('/flags', { replaceState: true, keepFocus: true, noScroll: true })
    }
  }

  $: if ($ready && $selectedFlagRoundId !== lastFlagsRound) {
    lastFlagsRound = $selectedFlagRoundId
    loadFlags($selectedFlagRoundId)
  }

  function selectFlagRound(id) {
    selectedFlagRoundId.set(id)
    if (id) goto(`/flags/${id}`)
    else goto('/flags')
  }
</script>

<FlagsPage
  rounds={$rounds}
  flags={$flags}
  teams={$teams}
  challenges={$challenges}
  selectedFlagRoundId={$selectedFlagRoundId}
  onSelectFlagRound={selectFlagRound}
  onSubmitFlag={submitFlag}
/>
