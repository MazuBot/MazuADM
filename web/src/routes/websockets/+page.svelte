<script>
  import { onMount, onDestroy } from 'svelte'
  import * as api from '$lib/data/api'
  import { app } from '$lib/data/stores/app.js'

  const { wsConnections } = app

  let now = $state(Date.now())
  let interval

  onMount(() => {
    api.listWsConnections().then(data => wsConnections.set(data))
    interval = setInterval(() => now = Date.now(), 1000)
  })

  onDestroy(() => clearInterval(interval))

  function formatDuration(connectedAt) {
    const secs = Math.floor((now - new Date(connectedAt).getTime()) / 1000)
    if (secs < 60) return `${secs}s`
    if (secs < 3600) return `${Math.floor(secs / 60)}m ${secs % 60}s`
    const h = Math.floor(secs / 3600)
    const m = Math.floor((secs % 3600) / 60)
    return `${h}h ${m}m`
  }

  function formatTime(ts) {
    return new Date(ts).toLocaleString()
  }
</script>

<header>
  <h1>WebSocket Connections</h1>
</header>

{#if $wsConnections.length}
  <table>
    <thead>
      <tr>
        <th>ID</th>
        <th>Client IP</th>
        <th>Client Name</th>
        <th>User</th>
        <th>Subscribed Events</th>
        <th>Connected At</th>
        <th>Duration</th>
      </tr>
    </thead>
    <tbody>
      {#each $wsConnections as c}
        <tr>
          <td>{c.id.slice(0, 8)}</td>
          <td>{c.client_ip}</td>
          <td>{c.client_name}</td>
          <td>{c.user}</td>
          <td>{c.subscribed_events.length ? c.subscribed_events.join(', ') : '(all)'}</td>
          <td>{formatTime(c.connected_at)}</td>
          <td>{formatDuration(c.connected_at)}</td>
        </tr>
      {/each}
    </tbody>
  </table>
{:else}
  <p>No active WebSocket connections.</p>
{/if}
