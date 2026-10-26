// Prints a component on its own: mounts it at the end of <body>, hides the
// rest of the app via CSS while printing, and cleans up afterwards. The system
// print dialog offers "Save as PDF".
import { mount, tick, unmount, type Component } from 'svelte';

let cleanup: (() => void) | null = null;

export async function printWith<P extends Record<string, unknown>>(
  component: Component<P>,
  props: P,
  /** Extra CSS for this print job, e.g. an @page rule with the paper size. */
  pageCss = '@page { margin: 14mm; }',
): Promise<void> {
  cleanup?.();
  const root = document.createElement('div');
  root.className = 'print-root';
  document.body.append(root);
  const style = document.createElement('style');
  style.textContent = pageCss;
  document.head.append(style);
  const inst = mount(component, { target: root, props });
  document.body.classList.add('printing');
  await tick();
  // Let images (barcodes, photos) finish decoding before the snapshot.
  await Promise.all(
    [...root.querySelectorAll('img')].map((img) => (img.complete ? Promise.resolve() : img.decode().catch(() => {}))),
  );

  cleanup = () => {
    cleanup = null;
    window.removeEventListener('afterprint', done);
    document.body.classList.remove('printing');
    void unmount(inst);
    root.remove();
    style.remove();
  };
  const done = () => cleanup?.();
  window.addEventListener('afterprint', done);
  // WebView2 blocks here until the dialog closes; WebKitGTK may return at
  // once and render pages later, so only clean up on afterprint (or on the
  // next print job). Leftovers are invisible: .print-root is hidden on screen.
  window.print();
}
