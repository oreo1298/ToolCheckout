import { isEditable, listenForScans, RESET_AFTER_MS, ScanBuffer } from './scan';

function feed(buf: ScanBuffer, text: string, start = 0, gap = 8) {
  let t = start;
  let out: string | null = null;
  for (const ch of text) {
    out = buf.key(ch, (t += gap)) ?? out;
  }
  return { out: buf.key('Enter', t + gap), t: t + gap };
}

describe('ScanBuffer', () => {
  it('returns a scanned code on Enter', () => {
    expect(feed(new ScanBuffer(), 'T-0001').out).toBe('T-0001');
  });

  it('ignores lone Enters and single characters', () => {
    const b = new ScanBuffer();
    expect(b.key('Enter', 0)).toBeNull();
    expect(feed(b, 'x', 10).out).toBeNull();
  });

  it('starts over after a long pause', () => {
    const b = new ScanBuffer();
    b.key('Z', 0);
    b.key('Z', 10);
    const { out } = feed(b, 'BADGE-7', 10 + RESET_AFTER_MS + 1);
    expect(out).toBe('BADGE-7');
  });

  it('supports manual typing with backspace', () => {
    const b = new ScanBuffer();
    for (const k of ['T', '-', '0', '0', '9', 'Backspace', '0', '2']) b.key(k, 100);
    expect(b.key('Enter', 120)).toBe('T-0002');
  });

  it('Tab-terminated scanners work too', () => {
    const b = new ScanBuffer();
    for (const k of 'E1042') b.key(k, 1);
    expect(b.key('Tab', 2)).toBe('E1042');
  });
});

describe('listenForScans', () => {
  it('captures keys outside text fields and skips inputs', () => {
    const seen: string[] = [];
    const stop = listenForScans((c) => seen.push(c));
    const type = (target: EventTarget, text: string) => {
      for (const ch of [...text, 'Enter']) {
        target.dispatchEvent(new KeyboardEvent('keydown', { key: ch, bubbles: true, cancelable: true }));
      }
    };
    type(document.body, 'T-0007');
    const input = document.createElement('input');
    document.body.append(input);
    type(input, 'T-0008');
    stop();
    type(document.body, 'T-0009');
    expect(seen).toEqual(['T-0007']);
  });

  it('stays out of the way while inactive', () => {
    const seen: string[] = [];
    let active = false;
    const stop = listenForScans((c) => seen.push(c), undefined, () => active);
    const type = (text: string) => {
      for (const ch of [...text, 'Enter']) {
        document.body.dispatchEvent(new KeyboardEvent('keydown', { key: ch, bubbles: true, cancelable: true }));
      }
    };
    // Inactive: keys pass through untouched, so a sheet's listener still sees them.
    let prevented = false;
    const spy = (e: KeyboardEvent) => (prevented ||= e.defaultPrevented);
    window.addEventListener('keydown', spy);
    type('T-0001');
    window.removeEventListener('keydown', spy);
    expect(prevented).toBe(false);
    active = true;
    type('T-0002');
    stop();
    expect(seen).toEqual(['T-0002']);
  });

  it('knows which elements are editable', () => {
    expect(isEditable(document.createElement('textarea'))).toBe(true);
    const cb = document.createElement('input');
    cb.type = 'checkbox';
    expect(isEditable(cb)).toBe(false);
    expect(isEditable(document.createElement('button'))).toBe(false);
  });
});
