// Display metadata for domain enums.
import type { Condition, ConditionSource, ToolStatus } from './api';

export const CONDITIONS: { value: Condition; label: string; hint: string; tone: Tone }[] = [
  { value: 'new', label: 'New', hint: 'Unused, like out of the box', tone: 'success' },
  { value: 'good', label: 'Good', hint: 'Works perfectly, light marks', tone: 'success' },
  { value: 'fair', label: 'Fair', hint: 'Works, visible wear', tone: 'info' },
  { value: 'worn', label: 'Worn', hint: 'Works, near end of life', tone: 'warning' },
  { value: 'damaged', label: 'Damaged', hint: 'Partly works or unsafe — needs repair', tone: 'danger' },
  { value: 'broken', label: 'Broken', hint: 'Does not work', tone: 'danger' },
];

export type Tone = 'success' | 'info' | 'warning' | 'danger' | 'neutral' | 'accent';

export const conditionInfo = (c: Condition) => CONDITIONS.find((x) => x.value === c)!;
export const conditionRank = (c: Condition) => CONDITIONS.findIndex((x) => x.value === c);
export const isDowngrade = (after: Condition, before: Condition) => conditionRank(after) > conditionRank(before);
export const needsRepair = (c: Condition) => c === 'damaged' || c === 'broken';

export const STATUSES: Record<ToolStatus, { label: string; tone: Tone }> = {
  available: { label: 'Available', tone: 'success' },
  checked_out: { label: 'Checked out', tone: 'accent' },
  awaiting_inspection: { label: 'Awaiting inspection', tone: 'warning' },
  out_for_repair: { label: 'Out for repair', tone: 'info' },
  lost: { label: 'Lost', tone: 'danger' },
  retired: { label: 'Retired', tone: 'neutral' },
};

export const CONDITION_SOURCES: Record<ConditionSource, string> = {
  initial: 'Added',
  manual: 'Condition update',
  return_inspection: 'Return inspection',
  inspection: 'Inspection',
  repair: 'Back from repair',
  found: 'Found',
};

export const LOAN_PRESETS: { minutes: number; label: string }[] = [
  { minutes: 30, label: '30 min' },
  { minutes: 60, label: '1 hour' },
  { minutes: 120, label: '2 hours' },
  { minutes: 240, label: '4 hours' },
  { minutes: 480, label: '8 hours' },
  { minutes: 1440, label: '1 day' },
  { minutes: 4320, label: '3 days' },
  { minutes: 10080, label: '1 week' },
];
