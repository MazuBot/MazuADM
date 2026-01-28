<script>
  import { app } from '$lib/data/stores/app.js'
  import TeamsPage from '$lib/pages/TeamsPage.svelte'

  const { teams, loadAll } = app
</script>

<TeamsPage teams={$teams} onRefresh={loadAll} />
