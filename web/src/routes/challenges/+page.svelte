<script>
  import { app } from '$lib/data/stores/app.js'
  import ChallengesPage from '$lib/pages/ChallengesPage.svelte'

  const { challenges, loadAll } = app
</script>

<ChallengesPage challenges={$challenges} onRefresh={loadAll} />
