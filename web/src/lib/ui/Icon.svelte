<script>
  import { FontAwesomeIcon } from '@fortawesome/svelte-fontawesome';
  import { faPlay, faPause, faStop, faRotate, faXmark, faPen, faPlus, faGear } from '@fortawesome/free-solid-svg-icons';

  const icons = { play: faPlay, pause: faPause, stop: faStop, rotate: faRotate, xmark: faXmark, pen: faPen, plus: faPlus, gear: faGear };

  let { name } = $props();
</script>

<FontAwesomeIcon icon={icons[name]} />
