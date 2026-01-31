<script>
  import { FontAwesomeIcon } from '@fortawesome/svelte-fontawesome';
  import { faPlay, faPause, faStop, faRotate, faXmark, faPen, faPlus, faGear, faCopy, faBug } from '@fortawesome/free-solid-svg-icons';

  const icons = { play: faPlay, pause: faPause, stop: faStop, rotate: faRotate, xmark: faXmark, pen: faPen, plus: faPlus, gear: faGear, copy: faCopy, bug: faBug };

  let { name } = $props();
</script>

<FontAwesomeIcon icon={icons[name]} />
