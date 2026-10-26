import { render } from '@testing-library/svelte';
import LabelSheet from './LabelSheet.svelte';
import { stockById } from './stocks';

describe('LabelSheet', () => {
  it('renders a scalable Code 128 barcode and the tag for every label', () => {
    const items = [
      { tag: 'T-0001', name: 'Driver' },
      { tag: 'T-0002', name: 'Spudger' },
    ];
    const { container } = render(LabelSheet, { props: { stock: stockById('avery5160'), items, shopName: 'Bench' } });
    const labels = container.querySelectorAll('.label');
    expect(labels).toHaveLength(2);
    expect(labels[0].textContent).toContain('T-0001');
    expect(labels[0].textContent).toContain('Bench');
    const svg = container.querySelector('svg.bc')!;
    expect(svg.getAttribute('viewBox')).toMatch(/^0 0 \d+/);
    expect(svg.getAttribute('preserveAspectRatio')).toBe('none');
    expect(svg.querySelectorAll('rect').length).toBeGreaterThan(20);
  });

  it('applies the calibration offset and shrinks the page box', () => {
    const s = stockById('avery5160');
    const { container } = render(LabelSheet, { props: { stock: s, items: [{ tag: 'T-1', name: '' }], offsetX: -6.35, offsetY: 2 } });
    const label = container.querySelector('.label') as HTMLElement;
    expect(parseFloat(label.style.left)).toBeCloseTo(s.left - 6.35);
    expect(parseFloat(label.style.top)).toBeCloseTo(s.top + 2);
    const page = container.querySelector('.page') as HTMLElement;
    // Box ends 1 mm past the last label (sheets have wider margins than that).
    expect(parseFloat(page.style.width)).toBeCloseTo(s.left + 2 * s.pitchX + s.labelW - 6.35 + 1);
    expect(parseFloat(page.style.height)).toBeCloseTo(s.top + 9 * s.pitchY + s.labelH + 2 + 1);
    expect(parseFloat(page.style.height)).toBeLessThan(s.pageH);
  });

  it('positions labels in millimetres and splits pages', () => {
    const s = stockById('avery_l7160'); // 21 per page
    const items = Array.from({ length: 22 }, (_, i) => ({ tag: `T-${i}`, name: '' }));
    const { container } = render(LabelSheet, { props: { stock: s, items } });
    expect(container.querySelectorAll('.page')).toHaveLength(2);
    const first = container.querySelector('.label') as HTMLElement;
    expect(first.style.left).toBe(`${s.left}mm`);
  });
});
