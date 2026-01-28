<script>
  import { onMount, tick } from 'svelte';
  import { fade, scale } from 'svelte/transition';

  export let onClose = () => {};
  export let wide = false;
  export let ariaLabel = 'Close modal';
  let modalEl;

  function onOverlayClick(e) {
    if (e.target === e.currentTarget) onClose();
  }

  function onOverlayKeydown(e) {
    if (e.key === 'Escape') {
      e.preventDefault();
      onClose();
    }
  }

  function onWindowKeydown(e) {
    if (e.key !== 'Escape') return;
    e.preventDefault();
    onClose();
  }

  onMount(() => {
    tick().then(() => {
      const target = modalEl?.querySelector('[data-autofocus]:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled])');
      target?.focus();
    });
    window.addEventListener('keydown', onWindowKeydown);
    return () => window.removeEventListener('keydown', onWindowKeydown);
  });
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
    bind:this={modalEl}
  >
    <slot />
  </div>
</div>
