<script lang="ts">
  interface Props {
    size?: number;
    class?: string;
  }

  let { size = 20, class: className = '' }: Props = $props();
</script>


<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  class={className}
  aria-hidden="true"
>
  <path
    d="M12 5v14M5 12h14"
    stroke="currentColor"
    stroke-width="2"
    stroke-linecap="round"
  />
</svg>
