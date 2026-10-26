<script lang="ts">
  import { hue, initials } from '../format';
  let { name, size = 40, muted = false }: { name: string; size?: number; muted?: boolean } = $props();
</script>

<span
  class="avatar"
  class:muted
  style="--h:{hue(name)}; width:{size}px; height:{size}px; font-size:{Math.round(size * 0.4)}px"
  aria-hidden="true">{initials(name)}</span
>

<style>
  .avatar {
    flex: none;
    display: inline-grid;
    place-items: center;
    border-radius: 50%;
    font-weight: 700;
    letter-spacing: 0.02em;
    background: hsl(var(--h) 60% 88%);
    color: hsl(var(--h) 55% 28%);
  }
  :global(:root[data-theme='dark']) .avatar {
    background: hsl(var(--h) 35% 28%);
    color: hsl(var(--h) 70% 85%);
  }
  .avatar.muted {
    filter: grayscale(1);
    opacity: 0.6;
  }
</style>
