import { countdown, dueLabel, duration, initials, loanPeriod, money, parseMoney, todayAt, when } from './format';

const MIN = 60_000;

describe('duration', () => {
  it('formats compactly', () => {
    expect(duration(30_000)).toBe('<1m');
    expect(duration(45 * MIN)).toBe('45m');
    expect(duration(3 * 60 * MIN)).toBe('3h');
    expect(duration(200 * MIN)).toBe('3h 20m');
    expect(duration(52 * 60 * MIN)).toBe('2d 4h');
  });
  it('labels due vs overdue', () => {
    expect(dueLabel(1000 + 90 * MIN, 1000)).toBe('due in 1h 30m');
    expect(dueLabel(1000, 1000 + 3 * 60 * MIN)).toBe('3h overdue');
  });
  it('counts down', () => {
    expect(countdown(65_000)).toBe('1:05');
    expect(countdown(-5)).toBe('0:00');
  });
});

describe('money', () => {
  it('formats and parses cents', () => {
    expect(money(1250)).toBe('$12.50');
    expect(money(null)).toBe('—');
    expect(parseMoney('$12.5')).toBe(1250);
    expect(parseMoney('  ')).toBeNull();
    expect(parseMoney('abc')).toBeNull();
    expect(parseMoney('12,99')).toBe(1299);
  });
});

describe('when', () => {
  it('names nearby days', () => {
    const noon = new Date(2026, 0, 5, 12, 0).getTime();
    expect(when(noon + 60 * MIN, noon)).not.toMatch(/day/);
    expect(when(noon + 24 * 60 * MIN, noon)).toMatch(/^Tomorrow/);
    expect(when(noon - 24 * 60 * MIN, noon)).toMatch(/^Yesterday/);
  });
});

describe('misc', () => {
  it('initials', () => {
    expect(initials('Ana María Ruiz')).toBe('AR');
    expect(initials('bo')).toBe('BO');
    expect(initials('  ')).toBe('?');
  });
  it('loan periods', () => {
    expect(loanPeriod(240)).toBe('4 hours');
    expect(loanPeriod(1440)).toBe('1 day');
    expect(loanPeriod(90)).toBe('90 minutes');
  });
  it('todayAt rolls to tomorrow once past', () => {
    const noon = new Date(2026, 0, 5, 12, 0).getTime();
    expect(new Date(todayAt('17:30', noon)).getHours()).toBe(17);
    expect(new Date(todayAt('09:00', noon)).getDate()).toBe(6);
  });
});
