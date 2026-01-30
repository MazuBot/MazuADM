<script>
  import Board from './Board.svelte';

  let { challenges, teams, exploits, exploitRuns, settings, selectedChallengeId, onSelectChallenge, onRefresh } = $props();
</script>

{#if challenges.length}
  <div class="challenge-tabs">
    {#each challenges as c}
      <button class:active={selectedChallengeId === c.id} class:disabled={!c.enabled} onclick={() => onSelectChallenge(c.id)}>
        {c.name}
      </button>
    {/each}
  </div>
{/if}

<Board {challenges} {teams} {exploits} {exploitRuns} {settings} challengeId={selectedChallengeId} onRefresh={onRefresh} />
