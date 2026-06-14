<script lang="ts">

  interface Props {

    title: string;

    showBack?: boolean;

    onback?: () => void;

  }

  let { title, showBack = false, onback }: Props = $props();
</script>



<header class="panel-header">

  <div class="panel-header-start">

    {#if showBack && onback}

      <button class="back-btn liquid-control" onclick={onback} aria-label="Back">

        <svg width="14" height="14" viewBox="0 0 14 14" aria-hidden="true">

          <path

            d="M8.5 2.5L4 7l4.5 4.5"

            fill="none"

            stroke="currentColor"

            stroke-width="1.6"

            stroke-linecap="round"

            stroke-linejoin="round"

          />

        </svg>

      </button>

    {/if}

  </div>



  <h1 class="panel-title">{title}</h1>

</header>



<style>

  .panel-header {

    display: grid;

    grid-template-columns: 1fr auto 1fr;

    align-items: center;

    padding: 14px 20px 10px;

    flex-shrink: 0;

  }

  .panel-header-start {

    justify-self: start;

  }

  .back-btn {

    display: flex;

    align-items: center;

    justify-content: center;

    width: 36px;

    height: 36px;

    border-radius: 50%;

    color: var(--text-secondary);

    transition: background var(--duration-fast) var(--ease-smooth);

  }

  .back-btn:hover {

    background: var(--control-bg-hover);

    color: var(--text-primary);

  }

  .panel-title {

    font-size: 15px;

    font-weight: 600;

    letter-spacing: -0.02em;

    color: var(--text-primary);

    text-align: center;

    grid-column: 2;

  }
</style>

