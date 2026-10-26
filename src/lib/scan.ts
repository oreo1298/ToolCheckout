// Barcode scanner capture. USB scanners behave like a keyboard: they type the
// code very fast and press Enter. We buffer printable keys typed while no text
// field has focus and hand the code over on Enter. Typing a tag by hand and
// pressing Enter works the same way, which keeps everything keyboard-usable.

/** A pause longer than this starts a new code (people type slower than scanners). */
export const RESET_AFTER_MS = 1500;

export class ScanBuffer {
  private chars = '';
  private last = 0;

  /** Feed one key; returns the finished code on Enter, else null. */
  key(key: string, t: number): string | null {
    if (t - this.last > RESET_AFTER_MS) this.chars = '';
    this.last = t;
    if (key === 'Enter' || key === 'Tab') {
      const code = this.chars.trim();
      this.chars = '';
      return code.length >= 2 ? code : null;
    }
    if (key === 'Escape') {
      this.chars = '';
      return null;
    }
    if (key === 'Backspace') {
      this.chars = this.chars.slice(0, -1);
      return null;
    }
    if (key.length === 1) this.chars += key;
    return null;
  }

  get pending(): string {
    return this.chars;
  }

  clear() {
    this.chars = '';
  }
}

/** True when the key should go to a text field instead of the scan buffer. */
export function isEditable(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  if (target.closest('[data-no-scan]')) return true;
  if (target.isContentEditable) return true;
  if (target instanceof HTMLTextAreaElement || target instanceof HTMLSelectElement) return true;
  if (target instanceof HTMLInputElement) {
    return !['checkbox', 'radio', 'button', 'submit', 'reset', 'range', 'color', 'file'].includes(target.type);
  }
  return false;
}

/**
 * Listens on the window while active. Returns an unsubscribe function.
 * `onPending` reports the partial code so the UI can show what's being read.
 */
export function listenForScans(
  onScan: (code: string) => void,
  onPending?: (partial: string) => void,
  /** While false, this listener ignores keys entirely (e.g. a sheet is on top). */
  isActive: () => boolean = () => true,
): () => void {
  const buf = new ScanBuffer();
  const handler = (e: KeyboardEvent) => {
    if (!isActive()) {
      buf.clear();
      return;
    }
    if (e.defaultPrevented || e.ctrlKey || e.metaKey || e.altKey) return;
    if (isEditable(e.target)) return;
    // Let open PIN pads and dialogs' own key handling win for digits/Enter.
    if ((e.target as HTMLElement | null)?.closest?.('.pinpad')) return;
    const code = buf.key(e.key, performance.now());
    if (code) {
      e.preventDefault();
      onScan(code);
    } else if (e.key.length === 1 && buf.pending) {
      // Don't let a scanner's keystrokes trigger buttons (e.g. Space).
      e.preventDefault();
    }
    onPending?.(buf.pending);
  };
  window.addEventListener('keydown', handler);
  return () => window.removeEventListener('keydown', handler);
}
