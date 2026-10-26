// Theme resolution: user override → OS (portal/registry) → prefers-color-scheme.
// Pure helpers are exported for tests; ThemeController wires them to the DOM.

export type ThemeOverride = 'system' | 'light' | 'dark';
export type Scheme = 'light' | 'dark';

export interface OsAppearance {
  colorScheme: Scheme | null;
  accent: string | null;
  source?: string;
}

export const DEFAULT_ACCENT = '#1f6feb';

export function resolveTheme(
  override: ThemeOverride,
  os: Scheme | null,
  mediaPrefersDark: boolean,
): Scheme {
  if (override !== 'system') return override;
  if (os) return os;
  return mediaPrefersDark ? 'dark' : 'light';
}

/** Theme to request for native window chrome; null means "follow the OS". */
export function nativeTheme(override: ThemeOverride, os: Scheme | null): Scheme | null {
  return override === 'system' ? os : override;
}

type RGB = [number, number, number];

export function parseHex(hex: string): RGB | null {
  const m = /^#?([0-9a-f]{6})$/i.exec(hex.trim());
  if (!m) return null;
  const n = parseInt(m[1], 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}

export function toHex([r, g, b]: RGB): string {
  return '#' + [r, g, b].map((c) => Math.round(c).toString(16).padStart(2, '0')).join('');
}

export function luminance([r, g, b]: RGB): number {
  const lin = (c: number) => {
    const s = c / 255;
    return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b);
}

export function contrast(a: RGB, b: RGB): number {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
}

function mix(a: RGB, b: RGB, t: number): RGB {
  return [0, 1, 2].map((i) => a[i] + (b[i] - a[i]) * t) as RGB;
}

const LIGHT_SURFACE: RGB = [255, 255, 255];
const DARK_SURFACE: RGB = [26, 29, 35];
const WHITE: RGB = [255, 255, 255];
const BLACK: RGB = [0, 0, 0];

/**
 * Derive accent tokens that stay legible: the accent must reach 3:1 against the
 * surface (WCAG non-text contrast), nudged toward black/white if needed, and
 * text on the accent picks whichever of white/near-black contrasts more.
 */
export function accentTokens(accentHex: string | null, scheme: Scheme): Record<string, string> {
  let accent = parseHex(accentHex ?? '') ?? parseHex(DEFAULT_ACCENT)!;
  const surface = scheme === 'dark' ? DARK_SURFACE : LIGHT_SURFACE;
  const toward = scheme === 'dark' ? WHITE : BLACK;
  for (let i = 0; i < 10 && contrast(accent, surface) < 3; i++) {
    accent = mix(accent, toward, 0.12);
  }
  const onAccent = contrast(WHITE, accent) >= contrast([17, 19, 23], accent) ? '#ffffff' : '#111317';
  const soft = mix(accent, surface, scheme === 'dark' ? 0.78 : 0.88);
  return {
    '--accent': toHex(accent),
    '--accent-contrast': onAccent,
    '--accent-soft': toHex(soft),
  };
}

export class ThemeController {
  override: ThemeOverride = 'system';
  os: OsAppearance = { colorScheme: null, accent: null };
  current: Scheme = 'light';
  private media: MediaQueryList | null;
  private listeners = new Set<(s: Scheme) => void>();

  constructor(
    private root: HTMLElement,
    private applyNative?: (t: Scheme | null) => void,
  ) {
    this.media = typeof matchMedia === 'function' ? matchMedia('(prefers-color-scheme: dark)') : null;
    this.media?.addEventListener('change', () => this.apply());
  }

  setOverride(o: ThemeOverride) {
    this.override = o;
    this.apply();
  }

  setOs(a: OsAppearance) {
    this.os = a;
    this.apply();
  }

  onChange(fn: (s: Scheme) => void): () => void {
    this.listeners.add(fn);
    return () => this.listeners.delete(fn);
  }

  apply() {
    const scheme = resolveTheme(this.override, this.os.colorScheme, this.media?.matches ?? false);
    this.current = scheme;
    this.root.dataset.theme = scheme;
    this.root.style.colorScheme = scheme;
    for (const [k, v] of Object.entries(accentTokens(this.os.accent, scheme))) {
      this.root.style.setProperty(k, v);
    }
    this.applyNative?.(nativeTheme(this.override, this.os.colorScheme));
    this.listeners.forEach((fn) => fn(scheme));
  }
}
