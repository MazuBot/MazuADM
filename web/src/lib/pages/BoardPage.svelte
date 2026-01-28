<script>
  import Board from '$lib/components/Board.svelte';

  let { challenges, teams, exploits, exploitRuns, selectedChallengeId, onSelectChallenge, onRefresh } = $props();
</script>

{#if challenges.length}
  <div class="challenge-tabs">
    {#each challenges as c}
      <button class:active={selectedChallengeId === c.id} onclick={() => onSelectChallenge(c.id)}>
        {c.name}
      </button>
    {/each}
  </div>
{/if}

<Board {teams} {exploits} {exploitRuns} challengeId={selectedChallengeId} onRefresh={onRefresh} />
