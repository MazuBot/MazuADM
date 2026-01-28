<script>
  import { page } from '$app/stores'
  import { RoundsRoute } from '$lib/features/rounds'

  $: param = $page.params.id
  $: routeId = param && /^\d+$/.test(param) ? Number(param) : null
</script>

<RoundsRoute {routeId} />
