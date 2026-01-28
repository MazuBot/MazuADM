<script>
  import { app } from '$lib/data/stores/app.js'
  import { SettingsPage } from '$lib/features/settings'

  const { settings, loadAll } = app
</script>

<SettingsPage settings={$settings} onRefresh={loadAll} />
