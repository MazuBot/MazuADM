<script>
  import { page } from '$app/stores'
  import BoardRoute from '$lib/routes/BoardRoute.svelte'

  $: param = $page.params.id
  $: routeId = param && /^\d+$/.test(param) ? Number(param) : null
</script>

<BoardRoute {routeId} />
