import { conditionRank, isDowngrade, needsRepair } from './domain';

describe('condition scale', () => {
  it('is ordered New → Broken like the backend', () => {
    expect(conditionRank('new')).toBe(0);
    expect(conditionRank('broken')).toBe(5);
    expect(isDowngrade('worn', 'good')).toBe(true);
    expect(isDowngrade('good', 'good')).toBe(false);
    expect(isDowngrade('new', 'fair')).toBe(false);
    expect(needsRepair('damaged') && needsRepair('broken') && !needsRepair('worn')).toBe(true);
  });
});
