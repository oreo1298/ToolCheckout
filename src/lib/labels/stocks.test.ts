import { layout, STOCKS, stockById } from './stocks';

describe('label stocks', () => {
  it('fit on their pages with symmetric margins', () => {
    for (const s of STOCKS) {
      const right = s.pageW - (s.left + (s.cols - 1) * s.pitchX + s.labelW);
      const bottom = s.pageH - (s.top + (s.rows - 1) * s.pitchY + s.labelH);
      expect(right).toBeGreaterThanOrEqual(0);
      expect(bottom).toBeGreaterThanOrEqual(0);
      expect(Math.abs(right - s.left)).toBeLessThan(0.6);
      expect(s.pitchX).toBeGreaterThanOrEqual(s.labelW);
    }
  });

  it('lays out row by row and across pages', () => {
    const s = stockById('avery5160'); // 3 × 10
    const items = Array.from({ length: 31 }, (_, i) => i);
    const { pages, placed } = layout(s, items);
    expect(pages).toBe(2);
    expect(placed[0]).toMatchObject({ page: 0, x: s.left, y: s.top });
    expect(placed[1].x).toBeCloseTo(s.left + s.pitchX);
    expect(placed[3].y).toBeCloseTo(s.top + s.pitchY);
    expect(placed[30]).toMatchObject({ page: 1, x: s.left, y: s.top });
  });

  it('can skip used labels on a partial sheet', () => {
    const s = stockById('avery_l7160'); // 3 × 7
    const { placed } = layout(s, ['a'], 4);
    expect(placed[0].x).toBeCloseTo(s.left + s.pitchX);
    expect(placed[0].y).toBeCloseTo(s.top + s.pitchY);
  });

  it('falls back to the first stock for unknown ids', () => {
    expect(stockById('nope').id).toBe('avery5160');
  });
});
