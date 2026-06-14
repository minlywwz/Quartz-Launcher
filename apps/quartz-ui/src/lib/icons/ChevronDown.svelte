<script lang="ts">
  interface Props {
    size?: number;
    class?: string;
  }

  let { size = 14, class: className = '' }: Props = $props();
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
    d="M6 9l6 6 6-6"
    stroke="currentColor"
    stroke-width="2"
    stroke-linecap="round"
    stroke-linejoin="round"
  />
</svg>
