<script>
  import { app } from '$lib/stores/app.js'
  import SettingsPage from '$lib/pages/SettingsPage.svelte'

  const { settings, loadAll } = app
</script>

<SettingsPage settings={$settings} onRefresh={loadAll} />
