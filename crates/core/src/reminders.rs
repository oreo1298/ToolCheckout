//! When to send desktop notifications about overdue tools and the end of the
//! day. Pure scheduling logic; the desktop shell delivers them.
//!
//! * A loan that becomes overdue triggers one notification (batched with any
//!   others that went overdue around the same time).
//! * While anything stays overdue, a summary repeats every
//!   `overdue_reminder_minutes`.
//! * Never two overdue notifications within `MIN_GAP`, so a backlog can't spam.
//! * At closing time, once per day, a summary of everything still out.

use std::collections::HashSet;

use chrono::{Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone};
use serde::Serialize;

use crate::clock::MINUTE;
use crate::error::Result;
use crate::loans::{self, LoanRow};
use crate::settings::{Settings, parse_hhmm};

pub const MIN_GAP: i64 = 5 * MINUTE;
const MAX_LINES: usize = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NoticeKind {
    NewlyOverdue,
    StillOverdue,
    EndOfDay,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Notice {
    pub kind: NoticeKind,
    pub title: String,
    pub body: String,
}

#[derive(Debug, Default)]
pub struct ReminderState {
    notified: HashSet<i64>,
    last_overdue_notice: Option<i64>,
    last_end_of_day: Option<NaiveDate>,
}

fn local_time(ms: i64) -> String {
    Local
        .timestamp_millis_opt(ms)
        .single()
        .map(|d| d.format("%-I:%M %p").to_string())
        .unwrap_or_default()
}

fn lines(loans: &[&LoanRow], with_overdue_mark: bool) -> String {
    let mut out: Vec<String> = loans
        .iter()
        .take(MAX_LINES)
        .map(|l| {
            let mark = if with_overdue_mark && l.overdue {
                " (overdue)"
            } else {
                ""
            };
            format!(
                "• {}: {} {}{mark}",
                l.borrower_name, l.tool_name, l.asset_tag
            )
        })
        .collect();
    if loans.len() > MAX_LINES {
        out.push(format!("…and {} more", loans.len() - MAX_LINES));
    }
    out.join("\n")
}

fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// Decides which notifications are due now. `out` are the loans still out.
pub fn due(
    st: &mut ReminderState,
    now: i64,
    local_now: NaiveDateTime,
    out: &[LoanRow],
    settings: &Settings,
) -> Vec<Notice> {
    let mut notices = Vec::new();
    let overdue: Vec<&LoanRow> = out.iter().filter(|l| l.overdue).collect();
    st.notified.retain(|id| overdue.iter().any(|l| l.id == *id));

    if settings.overdue_notifications && !overdue.is_empty() {
        let newly: Vec<&LoanRow> = overdue
            .iter()
            .copied()
            .filter(|l| !st.notified.contains(&l.id))
            .collect();
        let since_last = st.last_overdue_notice.map(|t| now - t);
        let gap_ok = since_last.is_none_or(|d| d >= MIN_GAP);
        if !newly.is_empty() && gap_ok {
            notices.push(if let [one] = newly.as_slice() {
                Notice {
                    kind: NoticeKind::NewlyOverdue,
                    title: format!("Overdue: {} {}", one.tool_name, one.asset_tag),
                    body: format!("{} — was due {}", one.borrower_name, local_time(one.due_at)),
                }
            } else {
                Notice {
                    kind: NoticeKind::NewlyOverdue,
                    title: format!(
                        "{} now overdue",
                        plural(newly.len(), "tool is", "tools are")
                    ),
                    body: lines(&newly, false),
                }
            });
        } else if newly.is_empty()
            && since_last.is_some_and(|d| d >= settings.overdue_reminder_minutes * MINUTE)
        {
            notices.push(Notice {
                kind: NoticeKind::StillOverdue,
                title: format!("{} still overdue", plural(overdue.len(), "tool", "tools")),
                body: lines(&overdue, false),
            });
        }
        if !notices.is_empty() {
            st.last_overdue_notice = Some(now);
            st.notified.extend(overdue.iter().map(|l| l.id));
        }
    }

    if settings.end_of_day_reminder
        && let Some((h, m)) = parse_hhmm(&settings.closing_time)
        && let Some(closing) = NaiveTime::from_hms_opt(h, m, 0)
        && local_now.time() >= closing
        && st.last_end_of_day != Some(local_now.date())
    {
        st.last_end_of_day = Some(local_now.date());
        if !out.is_empty() {
            let all: Vec<&LoanRow> = out.iter().collect();
            notices.push(Notice {
                kind: NoticeKind::EndOfDay,
                title: format!(
                    "End of day: {} still out",
                    plural(out.len(), "tool", "tools")
                ),
                body: lines(&all, true),
            });
        }
    }
    notices
}

impl crate::Service {
    /// Notifications due now (call periodically from the shell).
    pub fn due_notices(&self, st: &mut ReminderState) -> Result<Vec<Notice>> {
        let now = self.now();
        let out = loans::open_loans(&self.conn, now)?;
        let local_now = Local
            .timestamp_millis_opt(now)
            .single()
            .map(|d| d.naive_local())
            .unwrap_or_default();
        Ok(due(st, now, local_now, &out, &self.settings))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{CheckoutState, Condition, ToolStatus};

    fn loan(id: i64, overdue: bool) -> LoanRow {
        LoanRow {
            id,
            batch_id: "b".into(),
            tool_id: id,
            asset_tag: format!("T-{id:04}"),
            tool_name: format!("Tool {id}"),
            tool_photo_id: None,
            tool_status: ToolStatus::CheckedOut,
            borrower_id: 1,
            borrower_name: "Sam".into(),
            checked_out_at: 0,
            due_at: 1,
            checkout_condition: Condition::Good,
            authorized_by: None,
            state: CheckoutState::Out,
            returned_at: None,
            dropoff_note: String::new(),
            inspected_at: None,
            inspected_by: None,
            return_condition: None,
            return_notes: String::new(),
            return_photo_id: None,
            overdue,
            late_ms: overdue.then_some(1),
        }
    }

    fn at(h: u32, m: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 3, 2)
            .unwrap()
            .and_hms_opt(h, m, 0)
            .unwrap()
    }

    #[test]
    fn notifies_once_when_a_loan_goes_overdue_then_repeats_on_interval() {
        let s = Settings::default(); // reminders every 60 min
        let mut st = ReminderState::default();
        let t0 = 1_000_000_000;
        assert!(due(&mut st, t0, at(10, 0), &[loan(1, false)], &s).is_empty());
        let n = due(&mut st, t0 + MINUTE, at(10, 1), &[loan(1, true)], &s);
        assert_eq!(n.len(), 1);
        assert_eq!(n[0].kind, NoticeKind::NewlyOverdue);
        assert!(n[0].title.contains("Tool 1"));
        // Same overdue loan: quiet until the reminder interval passes.
        assert!(due(&mut st, t0 + 30 * MINUTE, at(10, 30), &[loan(1, true)], &s).is_empty());
        let n = due(&mut st, t0 + 61 * MINUTE, at(11, 1), &[loan(1, true)], &s);
        assert_eq!(n[0].kind, NoticeKind::StillOverdue);
    }

    #[test]
    fn batches_and_throttles_new_overdues() {
        let s = Settings::default();
        let mut st = ReminderState::default();
        let t0 = 5_000_000_000;
        let n = due(&mut st, t0, at(9, 0), &[loan(1, true), loan(2, true)], &s);
        assert_eq!(n[0].title, "2 tools are now overdue");
        // Another goes overdue a minute later: held back by the minimum gap…
        let three = [loan(1, true), loan(2, true), loan(3, true)];
        assert!(due(&mut st, t0 + MINUTE, at(9, 1), &three, &s).is_empty());
        // …and delivered once the gap has passed.
        let n = due(&mut st, t0 + MIN_GAP, at(9, 5), &three, &s);
        assert_eq!(n.len(), 1);
        assert!(n[0].title.starts_with("Overdue: Tool 3"));
    }

    #[test]
    fn respects_the_setting() {
        let s = Settings {
            overdue_notifications: false,
            end_of_day_reminder: false,
            ..Settings::default()
        };
        let mut st = ReminderState::default();
        assert!(due(&mut st, 0, at(23, 0), &[loan(1, true)], &s).is_empty());
    }

    #[test]
    fn end_of_day_once_per_day_and_only_if_something_is_out() {
        let s = Settings {
            overdue_notifications: false,
            ..Settings::default()
        }; // closing 17:30
        let mut st = ReminderState::default();
        assert!(due(&mut st, 0, at(17, 29), &[loan(1, false)], &s).is_empty());
        let n = due(&mut st, 0, at(17, 30), &[loan(1, false), loan(2, true)], &s);
        assert_eq!(n[0].kind, NoticeKind::EndOfDay);
        assert_eq!(n[0].title, "End of day: 2 tools still out");
        assert!(n[0].body.contains("(overdue)"));
        assert!(due(&mut st, 0, at(18, 0), &[loan(1, false)], &s).is_empty());
        // Nothing out at closing → nothing sent, and not again that day.
        let mut st = ReminderState::default();
        assert!(due(&mut st, 0, at(17, 45), &[], &s).is_empty());
        assert!(due(&mut st, 0, at(17, 50), &[loan(1, false)], &s).is_empty());
    }

    #[test]
    fn long_lists_are_truncated() {
        let s = Settings::default();
        let many: Vec<_> = (1..=8).map(|i| loan(i, true)).collect();
        let n = due(&mut ReminderState::default(), 0, at(9, 0), &many, &s);
        assert!(n[0].body.ends_with("…and 3 more"));
    }
}
