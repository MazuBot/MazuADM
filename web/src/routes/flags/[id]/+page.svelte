<script>
  import { page } from '$app/stores'
  import FlagsRoute from '$lib/routes/FlagsRoute.svelte'

  $: param = $page.params.id
  $: routeId = param && /^\d+$/.test(param) ? Number(param) : null
</script>

<FlagsRoute {routeId} />
