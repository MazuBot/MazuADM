<script>
  import { toasts, removeToast } from './toastStore.js'
</script>

<div class="toast-stack" aria-live="polite">
  {#each $toasts as toast (toast.id)}
    <div
      class={`toast ${toast.variant}`}
      role="status"
      onclick={() => removeToast(toast.id)}
    >
      {toast.message}
    </div>
  {/each}
</div>

<style>
  .toast-stack { position: fixed; right: 1.25rem; bottom: 1.25rem; display: flex; flex-direction: column; gap: 0.5rem; z-index: 30; }
  .toast { background: #1a1a2e; border: 1px solid #00d9ff; color: #f2fbff; padding: 0.65rem 0.9rem; border-radius: 8px; font-size: 0.85rem; box-shadow: 0 10px 20px rgba(0, 0, 0, 0.35); max-width: 280px; cursor: pointer; }
  .toast.success { border-color: #00d9ff; }
  .toast.error { border-color: #ff6b6b; color: #ffecec; }
</style>
