<script lang="ts">
  // In-app yes/no question. window.confirm() looks different on every
  // webview (and isn't touch-sized), so the app asks with its own dialog.
  import type { Snippet } from 'svelte';
  import Modal from './Modal.svelte';

  let {
    title,
    confirmLabel,
    danger = false,
    onconfirm,
    onclose,
    children,
  }: {
    title: string;
    confirmLabel: string;
    danger?: boolean;
    onconfirm: () => void;
    onclose: () => void;
    children: Snippet;
  } = $props();
</script>

<Modal {title} size="sm" {onclose}>
  {@render children()}
  {#snippet footer()}
    <button class="btn" onclick={onclose}>Cancel</button>
    <button class="btn {danger ? 'btn-danger' : 'btn-primary'}" onclick={onconfirm}>{confirmLabel}</button>
  {/snippet}
</Modal>
