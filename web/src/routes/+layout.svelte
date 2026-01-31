<script>
  import '../app.css'
  import { onMount } from 'svelte'
  import { page } from '$app/stores'
  import { app } from '$lib/data/stores/app.js'
  import * as api from '$lib/data/api/index.js'
  import { getUser, setUser, setOnAuthError, setOnConnect } from '$lib/websocket.js'
  import ToastHost from '$lib/ui/ToastHost.svelte'

  let { children } = $props();

  const {
    selectedChallengeId,
    selectedRoundId,
    selectedFlagRoundId,
    wsConnections,
    loadAll,
    loadContainers,
    start,
    restart,
    stop
  } = app

  let pathname = $derived($page.url.pathname)

  let beVersion = $state('')
  let feVersion = import.meta.env.DEV ? 'dev' : __BUILD_GIT_HASH__
  let showUserModal = $state(false)
  let userInput = $state('')

  function shortHash(hash) {
    if (!hash) return ''
    return hash.substring(0, 7) + (hash.includes('(Changed)') ? '-c' : '')
  }

  function navHref(section) {
    if (section === 'board') return $selectedChallengeId ? `/board/${$selectedChallengeId}` : '/board'
    if (section === 'jobs') return $selectedRoundId ? `/jobs/${$selectedRoundId}` : '/jobs'
    if (section === 'flags') return $selectedFlagRoundId ? `/flags/${$selectedFlagRoundId}` : '/flags'
    return `/${section}`
  }

  function isValidUser(u) {
    return u.length >= 3 && u.length <= 16 && /^[a-zA-Z0-9]+$/.test(u)
  }

  function submitUser() {
    const u = userInput.trim()
    if (isValidUser(u)) {
      setUser(u)
      showUserModal = false
      restart()
    }
  }

  onMount(() => {
    setOnAuthError(() => {
      userInput = ''
      showUserModal = true
    })
    setOnConnect((isReconnect) => {
      api.version().then(v => beVersion = shortHash(v.git_hash))
      if (!isReconnect) return
      loadAll()
      if (pathname.startsWith('/containers')) {
        loadContainers()
      }
      if (pathname.startsWith('/websockets')) {
        api.listWsConnections().then(data => wsConnections.set(data))
      }
    })
    if (!getUser()) {
      showUserModal = true
    } else {
      start()
    }
    loadAll()
    api.version().then(v => beVersion = shortHash(v.git_hash))
    return () => stop()
  })

  let userValid = $derived(isValidUser(userInput.trim()))
</script>

{#if showUserModal}
  <div class="modal-overlay">
    <div class="modal">
      <h2>Enter Your Name</h2>
      <p>3-16 alphanumeric characters (a-z, A-Z, 0-9)</p>
      <input 
        type="text" 
        bind:value={userInput} 
        placeholder="Your name" 
        maxlength="16" 
        pattern="[a-zA-Z0-9]+"
        class:invalid={userInput.length > 0 && !userValid}
        onkeydown={(e) => e.key === 'Enter' && userValid && submitUser()} 
      />
      <button onclick={submitUser} disabled={!userValid}>Continue</button>
    </div>
  </div>
{/if}

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

  {@render children?.()}
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
  .modal-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.7);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }
  .modal {
    background: #1e1e1e;
    padding: 2rem;
    border-radius: 8px;
    max-width: 400px;
    width: 90%;
  }
  .modal h2 {
    margin-top: 0;
  }
  .modal input {
    width: 100%;
    padding: 0.5rem;
    margin: 1rem 0;
    box-sizing: border-box;
  }
  .modal input.invalid {
    border-color: #e74c3c;
    outline-color: #e74c3c;
  }
  .modal button {
    width: 100%;
    padding: 0.5rem;
  }
  .modal button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
