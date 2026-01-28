<script>
  import { page } from '$app/stores'
  import { BoardRoute } from '$lib/features/board'

  $: param = $page.params.id
  $: routeId = param && /^\d+$/.test(param) ? Number(param) : null
</script>

<BoardRoute {routeId} />
