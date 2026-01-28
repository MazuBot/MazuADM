<script>
  import '../app.css'
  import { onMount } from 'svelte'
  import { page } from '$app/stores'
  import { app } from '$lib/data/stores/app.js'

  const {
    selectedChallengeId,
    selectedRoundId,
    selectedFlagRoundId,
    loadAll,
    start,
    stop
  } = app

  $: pathname = $page.url.pathname

  function navHref(section) {
    if (section === 'board') return $selectedChallengeId ? `/board/${$selectedChallengeId}` : '/board'
    if (section === 'rounds') return $selectedRoundId ? `/rounds/${$selectedRoundId}` : '/rounds'
    if (section === 'flags') return $selectedFlagRoundId ? `/flags/${$selectedFlagRoundId}` : '/flags'
    return `/${section}`
  }

  onMount(() => {
    start()
    loadAll()
    return () => stop()
  })
</script>

<main>
  <header>
    <h1>MazuADM</h1>
    <nav>
      <a class:active={pathname.startsWith('/board')} href={navHref('board')}>Board</a>
      <a class:active={pathname.startsWith('/challenges')} href={navHref('challenges')}>Challenges</a>
      <a class:active={pathname.startsWith('/teams')} href={navHref('teams')}>Teams</a>
      <a class:active={pathname.startsWith('/rounds')} href={navHref('rounds')}>Rounds</a>
      <a class:active={pathname.startsWith('/flags')} href={navHref('flags')}>Flags</a>
      <a class:active={pathname.startsWith('/containers')} href={navHref('containers')}>Containers</a>
      <a class:active={pathname.startsWith('/settings')} href={navHref('settings')}>Settings</a>
    </nav>
  </header>

  <slot />
</main>
