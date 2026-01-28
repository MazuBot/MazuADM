<script>
  import { fade, scale } from 'svelte/transition';

  export let onClose = () => {};
  export let wide = false;
  export let ariaLabel = 'Close modal';

  function onOverlayClick(e) {
    if (e.target === e.currentTarget) onClose();
  }

  function onOverlayKeydown(e) {
    if (e.key === 'Escape' || e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      onClose();
    }
  }
</script>

<div
  class="modal-overlay"
  role="button"
  tabindex="0"
  aria-label={ariaLabel}
  transition:fade|global={{ duration: 100 }}
  onclick={onOverlayClick}
  onkeydown={onOverlayKeydown}
>
  <div
    class={wide ? 'modal wide' : 'modal'}
    role="dialog"
    aria-modal="true"
    transition:scale|global={{ duration: 100, start: 0.98 }}
  >
    <slot />
  </div>
</div>
