import { accentTokens, contrast, nativeTheme, parseHex, resolveTheme, ThemeController } from './theme';

describe('resolveTheme', () => {
  it('honours an explicit override', () => {
    expect(resolveTheme('dark', 'light', false)).toBe('dark');
    expect(resolveTheme('light', 'dark', true)).toBe('light');
  });
  it('prefers the OS portal/registry value over the media query', () => {
    expect(resolveTheme('system', 'dark', false)).toBe('dark');
    expect(resolveTheme('system', 'light', true)).toBe('light');
  });
  it('falls back to prefers-color-scheme', () => {
    expect(resolveTheme('system', null, true)).toBe('dark');
    expect(resolveTheme('system', null, false)).toBe('light');
  });
});

describe('nativeTheme', () => {
  it('follows the OS when set to system', () => {
    expect(nativeTheme('system', null)).toBeNull();
    expect(nativeTheme('system', 'dark')).toBe('dark');
    expect(nativeTheme('light', 'dark')).toBe('light');
  });
});

describe('accentTokens', () => {
  it('keeps at least 3:1 contrast against the surface in both schemes', () => {
    for (const hex of ['#1f6feb', '#ffeb3b', '#0a0a40', '#00ff00', '#777777']) {
      const light = parseHex(accentTokens(hex, 'light')['--accent'])!;
      const dark = parseHex(accentTokens(hex, 'dark')['--accent'])!;
      expect(contrast(light, [255, 255, 255])).toBeGreaterThanOrEqual(3);
      expect(contrast(dark, [26, 29, 35])).toBeGreaterThanOrEqual(3);
    }
  });
  it('falls back to the default accent for junk input', () => {
    expect(accentTokens('nope', 'light')['--accent']).toBe('#1f6feb');
  });
});

describe('ThemeController', () => {
  it('updates data-theme live when the OS appearance changes', () => {
    const root = document.createElement('html');
    const native: Array<string | null> = [];
    const c = new ThemeController(root, (t) => native.push(t));
    c.setOs({ colorScheme: 'dark', accent: '#ff8800' });
    expect(root.dataset.theme).toBe('dark');
    c.setOs({ colorScheme: 'light', accent: null });
    expect(root.dataset.theme).toBe('light');
    c.setOverride('dark');
    expect(root.dataset.theme).toBe('dark');
    expect(native.at(-1)).toBe('dark');
  });
});
