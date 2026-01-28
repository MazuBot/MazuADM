<script>
  import { app } from '$lib/data/stores/app.js'
  import { ChallengesPage } from '$lib/features/challenges'

  const { challenges, loadAll } = app
</script>

<ChallengesPage challenges={$challenges} onRefresh={loadAll} />
