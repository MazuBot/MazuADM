<script>
  import { app } from '$lib/stores/app.js'
  import TeamsPage from '$lib/pages/TeamsPage.svelte'

  const { teams, loadAll } = app
</script>

<TeamsPage teams={$teams} onRefresh={loadAll} />
