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
    d="M8 5v14l11-7L8 5z"
    fill="currentColor"
  />
</svg>
