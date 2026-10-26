//! Reports: per-borrower summary, per-tool summary and history, incidents.
//! All filterable by date range, borrower and tool, and exportable as CSV.

use std::collections::BTreeMap;

use chrono::{Local, TimeZone};
use serde::{Deserialize, Serialize};

use crate::error::{Result, invalid};
use crate::incidents::{self, Incident, IncidentFilter};
use crate::loans::{self, LoanFilter, LoanRow};
use crate::model::{CheckoutState, IncidentKind, IncidentStatus, ToolStatus};
use crate::{csv, tools};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ReportFilter {
    /// Inclusive start (UTC ms). Loans by checkout time, incidents by creation.
    pub from: Option<i64>,
    /// Exclusive end (UTC ms).
    pub to: Option<i64>,
    pub borrower_id: Option<i64>,
    pub tool_id: Option<i64>,
}

impl ReportFilter {
    fn loans(&self) -> LoanFilter {
        LoanFilter {
            tool_id: self.tool_id,
            borrower_id: self.borrower_id,
            from: self.from,
            to: self.to,
            limit: Some(1_000_000),
        }
    }
    fn incidents(&self) -> IncidentFilter {
        IncidentFilter {
            borrower_id: self.borrower_id,
            tool_id: self.tool_id,
            from: self.from,
            to: self.to,
            ..Default::default()
        }
    }
}

/// An incident counts against the borrower's cost exposure unless it was
/// resolved with an outcome that clears them (no fault, found, inspected).
pub fn counts_against(i: &Incident) -> bool {
    matches!(i.kind, IncidentKind::Damage | IncidentKind::Lost)
        && i.borrower_id.is_some()
        && !(i.status == IncidentStatus::Resolved && i.outcome.is_some_and(|o| o.clears_borrower()))
}

/// A loan that has come back (dropped off or inspected).
fn is_returned(l: &LoanRow) -> bool {
    l.returned_at.is_some() && l.state != CheckoutState::Lost
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BorrowerSummary {
    pub borrower_id: i64,
    pub name: String,
    pub checkouts: i64,
    pub returned: i64,
    pub late_returns: i64,
    /// Average lateness of the late returns.
    pub avg_late_ms: Option<i64>,
    pub currently_out: i64,
    pub currently_overdue: i64,
    pub damage_incidents: i64,
    pub lost_tools: i64,
    pub problems_reported: i64,
    /// Replacement cost of damaged/lost tools not cleared as "no fault".
    pub cost_exposure_cents: i64,
}

fn slot<'a>(
    map: &'a mut BTreeMap<i64, BorrowerSummary>,
    id: i64,
    name: &str,
) -> &'a mut BorrowerSummary {
    map.entry(id).or_insert_with(|| BorrowerSummary {
        borrower_id: id,
        name: name.to_string(),
        ..Default::default()
    })
}

pub fn borrower_summary(
    conn: &rusqlite::Connection,
    now: i64,
    f: &ReportFilter,
) -> Result<Vec<BorrowerSummary>> {
    let mut map: BTreeMap<i64, BorrowerSummary> = BTreeMap::new();
    let mut late_total: BTreeMap<i64, i64> = BTreeMap::new();
    for l in loans::history(conn, now, &f.loans())? {
        let s = slot(&mut map, l.borrower_id, &l.borrower_name);
        s.checkouts += 1;
        if l.state == CheckoutState::Out {
            s.currently_out += 1;
            if l.overdue {
                s.currently_overdue += 1;
            }
        }
        if is_returned(&l) {
            s.returned += 1;
            if let Some(late) = l.late_ms {
                s.late_returns += 1;
                *late_total.entry(l.borrower_id).or_default() += late;
            }
        }
    }
    for i in incidents::list(conn, &f.incidents())? {
        let (Some(id), Some(name)) = (i.borrower_id, i.borrower_name.as_deref()) else {
            continue;
        };
        let s = slot(&mut map, id, name);
        match i.kind {
            IncidentKind::Damage => s.damage_incidents += 1,
            IncidentKind::Lost => s.lost_tools += 1,
            IncidentKind::ProblemReport => s.problems_reported += 1,
        }
        if counts_against(&i) {
            s.cost_exposure_cents += i.replacement_cost_cents.unwrap_or(0);
        }
    }
    let mut rows: Vec<_> = map.into_values().collect();
    for r in &mut rows {
        if r.late_returns > 0 {
            r.avg_late_ms = Some(late_total[&r.borrower_id] / r.late_returns);
        }
    }
    rows.sort_by(|a, b| {
        b.cost_exposure_cents
            .cmp(&a.cost_exposure_cents)
            .then(b.late_returns.cmp(&a.late_returns))
            .then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(rows)
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolSummary {
    pub tool_id: i64,
    pub asset_tag: String,
    pub name: String,
    pub category: String,
    pub status: Option<ToolStatus>,
    pub condition: Option<crate::model::Condition>,
    pub replacement_cost_cents: Option<i64>,
    pub checkouts: i64,
    pub late_returns: i64,
    /// Total time on loan within the range (returned loans and loans still out).
    pub time_out_ms: i64,
    pub damage_incidents: i64,
    pub lost_incidents: i64,
    pub problem_reports: i64,
}

pub fn tool_summary(
    conn: &rusqlite::Connection,
    now: i64,
    f: &ReportFilter,
) -> Result<Vec<ToolSummary>> {
    let mut map: BTreeMap<i64, ToolSummary> = BTreeMap::new();
    for t in tools::list(
        conn,
        &tools::ToolFilter {
            include_retired: true,
            id: f.tool_id,
            limit: Some(1_000_000),
            ..Default::default()
        },
    )? {
        map.insert(
            t.tool.id,
            ToolSummary {
                tool_id: t.tool.id,
                asset_tag: t.tool.asset_tag,
                name: t.tool.name,
                category: t.tool.category,
                status: Some(t.tool.status),
                condition: Some(t.tool.condition),
                replacement_cost_cents: t.tool.replacement_cost_cents,
                ..Default::default()
            },
        );
    }
    for l in loans::history(conn, now, &f.loans())? {
        let Some(s) = map.get_mut(&l.tool_id) else {
            continue;
        };
        s.checkouts += 1;
        if is_returned(&l) && l.late_ms.is_some() {
            s.late_returns += 1;
        }
        let end = l.returned_at.unwrap_or(now);
        s.time_out_ms += (end - l.checked_out_at).max(0);
    }
    for i in incidents::list(conn, &f.incidents())? {
        let Some(s) = map.get_mut(&i.tool_id) else {
            continue;
        };
        match i.kind {
            IncidentKind::Damage => s.damage_incidents += 1,
            IncidentKind::Lost => s.lost_incidents += 1,
            IncidentKind::ProblemReport => s.problem_reports += 1,
        }
    }
    let mut rows: Vec<_> = map.into_values().collect();
    // With a borrower filter, only tools that borrower actually touched.
    if f.borrower_id.is_some() {
        rows.retain(|r| r.checkouts > 0 || r.damage_incidents > 0 || r.lost_incidents > 0);
    }
    rows.sort_by(|a, b| a.asset_tag.cmp(&b.asset_tag));
    Ok(rows)
}

pub fn loan_history(
    conn: &rusqlite::Connection,
    now: i64,
    f: &ReportFilter,
) -> Result<Vec<LoanRow>> {
    loans::history(conn, now, &f.loans())
}

pub fn incident_report(
    conn: &rusqlite::Connection,
    f: &ReportFilter,
    open_only: bool,
) -> Result<Vec<Incident>> {
    let mut filt = f.incidents();
    if open_only {
        filt.status = Some(IncidentStatus::Open);
        // Open incidents matter regardless of when they were opened.
        filt.from = None;
        filt.to = None;
    }
    incidents::list(conn, &filt)
}

// ---------------------------------------------------------------- CSV

pub fn local(ms: i64) -> String {
    Local
        .timestamp_millis_opt(ms)
        .single()
        .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_default()
}

fn opt_local(ms: Option<i64>) -> String {
    ms.map(local).unwrap_or_default()
}

fn hours(ms: i64) -> String {
    format!("{:.1}", ms as f64 / 3_600_000.0)
}

fn money(cents: Option<i64>) -> String {
    cents
        .map(|c| format!("{}.{:02}", c / 100, c % 100))
        .unwrap_or_default()
}

pub fn borrower_summary_csv(rows: &[BorrowerSummary]) -> String {
    csv::write(
        &[
            "Borrower",
            "Checkouts",
            "Returned",
            "Late returns",
            "Avg lateness (h)",
            "Currently out",
            "Currently overdue",
            "Damage incidents",
            "Lost tools",
            "Problems reported",
            "Replacement cost of damaged/lost",
        ],
        &rows
            .iter()
            .map(|r| {
                vec![
                    r.name.clone(),
                    r.checkouts.to_string(),
                    r.returned.to_string(),
                    r.late_returns.to_string(),
                    r.avg_late_ms.map(hours).unwrap_or_default(),
                    r.currently_out.to_string(),
                    r.currently_overdue.to_string(),
                    r.damage_incidents.to_string(),
                    r.lost_tools.to_string(),
                    r.problems_reported.to_string(),
                    money(Some(r.cost_exposure_cents)),
                ]
            })
            .collect::<Vec<_>>(),
    )
}

pub fn tool_summary_csv(rows: &[ToolSummary]) -> String {
    csv::write(
        &[
            "Tag",
            "Tool",
            "Category",
            "Status",
            "Condition",
            "Replacement cost",
            "Checkouts",
            "Late returns",
            "Hours out",
            "Damage incidents",
            "Lost incidents",
            "Problem reports",
        ],
        &rows
            .iter()
            .map(|r| {
                vec![
                    r.asset_tag.clone(),
                    r.name.clone(),
                    r.category.clone(),
                    r.status.map(|s| s.label().to_string()).unwrap_or_default(),
                    r.condition
                        .map(|c| c.label().to_string())
                        .unwrap_or_default(),
                    money(r.replacement_cost_cents),
                    r.checkouts.to_string(),
                    r.late_returns.to_string(),
                    hours(r.time_out_ms),
                    r.damage_incidents.to_string(),
                    r.lost_incidents.to_string(),
                    r.problem_reports.to_string(),
                ]
            })
            .collect::<Vec<_>>(),
    )
}

pub fn loans_csv(rows: &[LoanRow]) -> String {
    csv::write(
        &[
            "Checked out",
            "Tag",
            "Tool",
            "Borrower",
            "Due",
            "Returned",
            "State",
            "Hours late",
            "Condition out",
            "Condition back",
            "Return notes",
            "Inspected by",
            "Authorized by",
        ],
        &rows
            .iter()
            .map(|l| {
                vec![
                    local(l.checked_out_at),
                    l.asset_tag.clone(),
                    l.tool_name.clone(),
                    l.borrower_name.clone(),
                    local(l.due_at),
                    opt_local(l.returned_at),
                    l.state.label().to_string(),
                    l.late_ms.map(hours).unwrap_or_default(),
                    l.checkout_condition.label().to_string(),
                    l.return_condition
                        .map(|c| c.label().to_string())
                        .unwrap_or_default(),
                    l.return_notes.clone(),
                    l.inspected_by.clone().unwrap_or_default(),
                    l.authorized_by.clone().unwrap_or_default(),
                ]
            })
            .collect::<Vec<_>>(),
    )
}

pub fn incidents_csv(rows: &[Incident]) -> String {
    csv::write(
        &[
            "Opened",
            "Kind",
            "Tag",
            "Tool",
            "Borrower",
            "Condition before",
            "Condition after",
            "Notes",
            "Replacement cost",
            "Status",
            "Outcome",
            "Resolution note",
            "Resolved",
            "Resolved by",
        ],
        &rows
            .iter()
            .map(|i| {
                vec![
                    local(i.created_at),
                    i.kind.label().to_string(),
                    i.asset_tag.clone(),
                    i.tool_name.clone(),
                    i.borrower_name.clone().unwrap_or_default(),
                    i.before_condition
                        .map(|c| c.label().to_string())
                        .unwrap_or_default(),
                    i.after_condition
                        .map(|c| c.label().to_string())
                        .unwrap_or_default(),
                    i.notes.clone(),
                    money(i.replacement_cost_cents),
                    i.status.label().to_string(),
                    i.outcome.map(|o| o.label().to_string()).unwrap_or_default(),
                    i.resolution_note.clone().unwrap_or_default(),
                    opt_local(i.resolved_at),
                    i.resolved_by_name.clone().unwrap_or_default(),
                ]
            })
            .collect::<Vec<_>>(),
    )
}

// ---------------------------------------------------------------- service API

impl crate::Service {
    pub fn report_borrowers(&mut self, f: &ReportFilter) -> Result<Vec<BorrowerSummary>> {
        self.require_tech()?;
        borrower_summary(&self.conn, self.now(), f)
    }

    pub fn report_tools(&mut self, f: &ReportFilter) -> Result<Vec<ToolSummary>> {
        self.require_tech()?;
        tool_summary(&self.conn, self.now(), f)
    }

    pub fn report_loans(&mut self, f: &ReportFilter) -> Result<Vec<LoanRow>> {
        self.require_tech()?;
        loan_history(&self.conn, self.now(), f)
    }

    pub fn report_incidents(&mut self, f: &ReportFilter, open_only: bool) -> Result<Vec<Incident>> {
        self.require_tech()?;
        incident_report(&self.conn, f, open_only)
    }

    /// CSV text for a report: "borrowers", "tools", "loans", "incidents",
    /// "open_incidents".
    pub fn report_csv(&mut self, kind: &str, f: &ReportFilter) -> Result<String> {
        self.require_tech()?;
        let now = self.now();
        Ok(match kind {
            "borrowers" => borrower_summary_csv(&borrower_summary(&self.conn, now, f)?),
            "tools" => tool_summary_csv(&tool_summary(&self.conn, now, f)?),
            "loans" => loans_csv(&loan_history(&self.conn, now, f)?),
            "incidents" => incidents_csv(&incident_report(&self.conn, f, false)?),
            "open_incidents" => incidents_csv(&incident_report(&self.conn, f, true)?),
            other => return invalid(format!("Unknown report {other}")),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::borrowers::BorrowerInput;
    use crate::clock::HOUR;
    use crate::loans::CheckoutItem;
    use crate::model::{Condition, Outcome};
    use crate::service::tests::ready;
    use crate::tools::ToolInput;

    #[test]
    fn borrower_summary_counts_lateness_incidents_and_cost() {
        let (mut s, c, _r) = ready();
        let mk_tool = |s: &mut crate::Service, n: &str, cost| {
            s.create_tool(ToolInput {
                name: n.into(),
                default_loan_minutes: 60,
                condition: Some(Condition::Good),
                replacement_cost_cents: cost,
                ..Default::default()
            })
            .unwrap()
            .id
        };
        let a = mk_tool(&mut s, "A", Some(10_000));
        let b = mk_tool(&mut s, "B", Some(2_500));
        let d = mk_tool(&mut s, "D", None);
        let sam = s
            .create_borrower(
                BorrowerInput {
                    name: "Sam".into(),
                    ..Default::default()
                },
                Some("1111"),
            )
            .unwrap();
        let kim = s
            .create_borrower(
                BorrowerInput {
                    name: "Kim".into(),
                    ..Default::default()
                },
                Some("2222"),
            )
            .unwrap();
        let start = s.now();

        // Sam borrows A and B; returns A 2h late damaged, loses B.
        let t = s.authenticate_borrower(sam.id, Some("1111")).unwrap().token;
        let loans = s
            .checkout(
                &t,
                &[
                    CheckoutItem {
                        tool_id: a,
                        due_at: None,
                    },
                    CheckoutItem {
                        tool_id: b,
                        due_at: None,
                    },
                ],
            )
            .unwrap()
            .loans;
        c.advance(3 * HOUR);
        s.unlock(1, "123456").unwrap(); // tech mode timed out meanwhile
        let inc = s
            .inspect_return(loans[0].id, Condition::Damaged, "bent", None)
            .unwrap()
            .incident
            .unwrap();
        s.mark_lost(b, "gone").unwrap();
        // Kim borrows D, returns it on time, then reports a problem with it.
        let t = s.authenticate_borrower(kim.id, Some("2222")).unwrap().token;
        let l = s
            .checkout(
                &t,
                &[CheckoutItem {
                    tool_id: d,
                    due_at: None,
                }],
            )
            .unwrap()
            .loans
            .remove(0);
        c.advance(HOUR / 2);
        s.unlock(1, "123456").unwrap();
        s.inspect_return(l.id, Condition::Good, "", None).unwrap();
        let t = s.authenticate_borrower(kim.id, Some("2222")).unwrap().token;
        s.report_problem(&t, d, "loose tip", None).unwrap();

        let rows = s.report_borrowers(&ReportFilter::default()).unwrap();
        let sam_row = rows.iter().find(|r| r.name == "Sam").unwrap();
        assert_eq!(sam_row.checkouts, 2);
        assert_eq!(sam_row.returned, 1, "a lost loan is not a return");
        assert_eq!(sam_row.late_returns, 1);
        assert_eq!(sam_row.avg_late_ms, Some(2 * HOUR));
        assert_eq!((sam_row.damage_incidents, sam_row.lost_tools), (1, 1));
        assert_eq!(sam_row.cost_exposure_cents, 12_500);
        let kim_row = rows.iter().find(|r| r.name == "Kim").unwrap();
        assert_eq!(
            (
                kim_row.late_returns,
                kim_row.problems_reported,
                kim_row.cost_exposure_cents
            ),
            (0, 1, 0)
        );
        assert_eq!(rows[0].name, "Sam", "highest cost exposure first");

        // Resolving the damage as "no fault" removes it from the exposure.
        s.resolve_incident(inc.id, Outcome::NoFault, "normal wear")
            .unwrap();
        let rows = s.report_borrowers(&ReportFilter::default()).unwrap();
        assert_eq!(
            rows.iter()
                .find(|r| r.name == "Sam")
                .unwrap()
                .cost_exposure_cents,
            2_500
        );

        // Filters: by borrower, by tool, by date range.
        let only_kim = s
            .report_borrowers(&ReportFilter {
                borrower_id: Some(kim.id),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(only_kim.len(), 1);
        let tool_a = s
            .report_borrowers(&ReportFilter {
                tool_id: Some(a),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(tool_a.len(), 1);
        assert_eq!(tool_a[0].checkouts, 1);
        // From an hour in: Sam's loans (at start) drop out, his incidents (3h in) stay.
        let later = s
            .report_borrowers(&ReportFilter {
                from: Some(start + HOUR),
                ..Default::default()
            })
            .unwrap();
        let sam_later = later.iter().find(|r| r.name == "Sam").unwrap();
        assert_eq!(
            (
                sam_later.checkouts,
                sam_later.damage_incidents,
                sam_later.lost_tools
            ),
            (0, 1, 1)
        );

        // Tool summary
        let tools = s.report_tools(&ReportFilter::default()).unwrap();
        let ta = tools.iter().find(|t| t.tool_id == a).unwrap();
        assert_eq!(
            (ta.checkouts, ta.late_returns, ta.damage_incidents),
            (1, 1, 1)
        );
        assert_eq!(ta.time_out_ms, 3 * HOUR);

        // CSV
        let csv = s.report_csv("borrowers", &ReportFilter::default()).unwrap();
        assert!(csv.contains("Sam,2,1,1,2.0"));
        assert!(
            s.report_csv("open_incidents", &ReportFilter::default())
                .unwrap()
                .contains("Lost")
        );
        assert!(s.report_csv("nope", &ReportFilter::default()).is_err());
        s.lock();
        assert_eq!(
            s.report_borrowers(&ReportFilter::default())
                .unwrap_err()
                .code(),
            "tech_required"
        );
    }
}
