<script>
  import { page } from '$app/stores'
  import RoundsRoute from '$lib/routes/RoundsRoute.svelte'

  $: param = $page.params.id
  $: routeId = param && /^\d+$/.test(param) ? Number(param) : null
</script>

<RoundsRoute {routeId} />
