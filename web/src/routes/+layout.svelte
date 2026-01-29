<script>
  import '../app.css'
  import { onMount } from 'svelte'
  import { page } from '$app/stores'
  import { app } from '$lib/data/stores/app.js'
  import * as api from '$lib/data/api/index.js'
  import ToastHost from '$lib/ui/ToastHost.svelte'

  const {
    selectedChallengeId,
    selectedRoundId,
    selectedFlagRoundId,
    loadAll,
    start,
    stop
  } = app

  $: pathname = $page.url.pathname

  let beVersion = ''
  let feVersion = import.meta.env.DEV ? 'dev' : __BUILD_GIT_HASH__

  function shortHash(hash) {
    if (!hash) return ''
    return hash.substring(0, 7) + (hash.endsWith('-c') ? '-c' : '')
  }

  function navHref(section) {
    if (section === 'board') return $selectedChallengeId ? `/board/${$selectedChallengeId}` : '/board'
    if (section === 'jobs') return $selectedRoundId ? `/jobs/${$selectedRoundId}` : '/jobs'
    if (section === 'flags') return $selectedFlagRoundId ? `/flags/${$selectedFlagRoundId}` : '/flags'
    return `/${section}`
  }

  onMount(() => {
    start()
    loadAll()
    api.version().then(v => beVersion = shortHash(v.git_hash))
    return () => stop()
  })
</script>

<main>
  <header>
    <div class="title-wrapper">
      <h1>MazuADM</h1>
      <div class="version-info">
        <span>BE: {beVersion}</span>
        <span>FE: {feVersion}</span>
      </div>
    </div>
    <nav>
      <a class:active={pathname.startsWith('/board')} href={navHref('board')}>Board</a>
      <a class:active={pathname.startsWith('/challenges')} href={navHref('challenges')}>Challenges</a>
      <a class:active={pathname.startsWith('/teams')} href={navHref('teams')}>Teams</a>
      <a class:active={pathname.startsWith('/jobs')} href={navHref('jobs')}>Jobs</a>
      <a class:active={pathname.startsWith('/flags')} href={navHref('flags')}>Flags</a>
      <a class:active={pathname.startsWith('/containers')} href={navHref('containers')}>Containers</a>
      <a class:active={pathname.startsWith('/websockets')} href={navHref('websockets')}>WebSockets</a>
      <a class:active={pathname.startsWith('/settings')} href={navHref('settings')}>Settings</a>
    </nav>
  </header>

  <slot />
</main>

<ToastHost />


<style>
  .title-wrapper {
    display: flex;
    align-items: center;
    gap: 0.5em;
  }
  .version-info {
    display: flex;
    flex-direction: column;
    font-size: 0.6em;
    color: #888;
    line-height: 1.2;
  }
</style>
