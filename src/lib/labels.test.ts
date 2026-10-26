import { auditDiff, auditLabel } from './labels';

describe('audit labels', () => {
  it('maps known actions and passes unknown ones through', () => {
    expect(auditLabel('checkout.create')).toBe('Checked out');
    expect(auditLabel('weird.thing')).toBe('weird.thing');
  });
  it('summarizes before/after', () => {
    expect(auditDiff('{"status":"available"}', '{"status":"checked_out"}')).toBe('status: available → checked_out');
    expect(auditDiff(null, '{"name":"Driver","cost":5}')).toBe('name: Driver · cost: 5');
    expect(auditDiff('{"a":1}', '{}')).toBe('a: 1 → ∅');
    expect(auditDiff(null, null)).toBe('');
    expect(auditDiff(null, '{"lockedUntil":1767225600000}')).not.toContain('1767225600000');
  });
});
