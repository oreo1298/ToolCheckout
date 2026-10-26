//! Demo data: a separate database with sample tools, people and two weeks of
//! history, generated through the normal business logic (so the audit log is
//! real and verifies). It never touches the shop's real database.

use std::path::Path;
use std::sync::Arc;

use crate::Service;
use crate::borrowers::BorrowerInput;
use crate::clock::{DAY, HOUR, MINUTE, ManualClock};
use crate::error::Result;
use crate::loans::CheckoutItem;
use crate::model::{Condition, Outcome};
use crate::settings;
use crate::tools::ToolInput;

pub const TECH_NAME: &str = "Alex (demo tech)";
pub const TECH_PIN: &str = "123456";
pub const BORROWER_PIN: &str = "1234";

struct Demo {
    s: Service,
    c: Arc<ManualClock>,
    tech: i64,
}

impl Demo {
    fn at(&self, ms: i64) {
        self.c.set(ms);
    }
    fn unlock(&mut self) -> Result<()> {
        self.s.unlock(self.tech, TECH_PIN).map(|_| ())
    }
    fn token(&mut self, borrower: i64) -> Result<String> {
        Ok(self
            .s
            .authenticate_borrower(borrower, Some(BORROWER_PIN))?
            .token)
    }
    fn lend(&mut self, borrower: i64, tools: &[i64], minutes: i64) -> Result<Vec<i64>> {
        let t = self.token(borrower)?;
        let due = self.s.now() + minutes * MINUTE;
        let items: Vec<_> = tools
            .iter()
            .map(|&id| CheckoutItem {
                tool_id: id,
                due_at: Some(due),
            })
            .collect();
        // Tech-only tools need a technician to authorize.
        if tools.iter().any(|&id| {
            crate::tools::get(self.s.conn(), id)
                .map(|t| t.tech_only)
                .unwrap_or(false)
        }) {
            self.unlock()?;
        }
        Ok(self
            .s
            .checkout(&t, &items)?
            .loans
            .into_iter()
            .map(|l| l.id)
            .collect())
    }
    fn inspect(&mut self, loan: i64, c: Condition, notes: &str) -> Result<Option<i64>> {
        self.unlock()?;
        Ok(self
            .s
            .inspect_return(loan, c, notes, None)?
            .incident
            .map(|i| i.id))
    }
}

/// (name, category, home location, replacement cost in cents, loan minutes,
/// condition, tech only)
type ToolSpec = (
    &'static str,
    &'static str,
    &'static str,
    i64,
    i64,
    Condition,
    bool,
);

#[rustfmt::skip]
const TOOLS: [ToolSpec; 15] = {
    use Condition::*;
    [
        ("iFixit Pro Tech Toolkit", "Kits", "Bench 1, top drawer", 7499, 240, Good, false),
        ("Pentalobe P5 driver", "Drivers", "Bench 1, driver rack", 899, 240, Good, false),
        ("Tri-point Y000 driver", "Drivers", "Bench 1, driver rack", 899, 240, New, false),
        ("Spudger set (nylon)", "Opening", "Bench 2, cup", 1299, 240, Fair, false),
        ("Opening picks (6)", "Opening", "Bench 2, cup", 599, 480, Worn, false),
        ("Precision tweezers ESD", "Handling", "Bench 2, top drawer", 1999, 240, Good, false),
        ("Heat gun (Wagner)", "Heat", "Shelf A", 4599, 480, Good, false),
        ("Silicone repair mat", "Handling", "Shelf A", 2499, 1440, Good, false),
        ("Fluke 117 multimeter", "Meters", "Bench 2, top drawer", 22999, 240, Good, false),
        ("USB-C power meter", "Meters", "Bench 2, top drawer", 3499, 240, New, false),
        ("Hakko FX-951 soldering station", "Soldering", "Bench 3", 34999, 480, Good, true),
        ("Hot air rework station", "Soldering", "Bench 3", 18999, 240, Good, true),
        ("Torque screwdriver set", "Drivers", "Bench 1, lower drawer", 8999, 240, Good, false),
        ("Metal spudger (old)", "Opening", "Junk bin", 499, 240, Worn, false),
        ("Suction handle", "Opening", "Bench 2, cup", 799, 240, Good, false),
    ]
};

/// Creates a fresh demo database at `path`, with "now" = `now_ms`.
pub fn create(path: &Path, now_ms: i64) -> Result<()> {
    for ext in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{}{ext}", path.display()));
    }
    let start = now_ms - 14 * DAY;
    let c = Arc::new(ManualClock::new(start));
    let mut s = Service::open(path, c.clone())?;
    let setup = s.setup("Demo Repair Co.", TECH_NAME, TECH_PIN)?;
    settings::set_sys(s.conn(), "demo", "1")?;
    let mut d = Demo {
        s,
        c,
        tech: setup.technician.id,
    };
    use Condition::*;

    // Tools
    let s = &mut d.s;
    let mut ids = Vec::new();
    for (name, cat, loc, cost, mins, cond, tech_only) in TOOLS {
        let input = ToolInput {
            name: name.into(),
            category: cat.into(),
            home_location: loc.into(),
            replacement_cost_cents: Some(cost),
            default_loan_minutes: mins,
            condition: Some(cond),
            tech_only,
            ..Default::default()
        };
        ids.push(s.create_tool(input)?.id);
    }
    let [
        kit,
        p5,
        tri,
        spud,
        picks,
        tweez,
        heat,
        mat,
        fluke,
        usbm,
        iron,
        _hot_air,
        torque,
        old,
        suction,
    ] = ids[..]
    else {
        unreachable!("one id per tool")
    };

    // People
    let person = |s: &mut Service, name: &str, emp: &str, badge: &str| -> Result<i64> {
        Ok(s.create_borrower(
            BorrowerInput {
                name: name.into(),
                employee_id: emp.into(),
                badge_code: Some(badge.into()),
            },
            Some(BORROWER_PIN),
        )?
        .id)
    };
    let jordan = person(s, "Jordan Lee", "E1001", "DEMO-01")?;
    let priya = person(s, "Priya Shah", "E1002", "DEMO-02")?;
    let marcus = person(s, "Marcus Brown", "E1003", "DEMO-03")?;
    let sofia = person(s, "Sofia García", "E1004", "DEMO-04")?;
    let ken = person(s, "Ken Watanabe", "E1005", "DEMO-05")?;
    let ava = person(s, "Ava Johnson", "E1006", "DEMO-06")?;
    let _ =
        d.s.retire_tool(old, "Tip snapped off; replaced by nylon set")?;
    d.s.lock();

    // Two weeks of ordinary traffic: mostly on time, a few late.
    let day = |n: i64, h: i64| start + n * DAY + h * HOUR;
    let routine: &[(i64, i64, &[i64], i64, i64)] = &[
        // (day, borrower, tools, loan minutes, return after minutes)
        (0, jordan, &[p5, spud], 240, 90),
        (0, priya, &[kit], 240, 200),
        (1, marcus, &[fluke], 240, 330),
        (1, ken, &[heat, mat], 480, 300),
        (2, sofia, &[tweez], 240, 60),
        (3, jordan, &[kit, suction], 240, 180),
        (3, ava, &[usbm], 240, 150),
        (4, marcus, &[p5], 240, 500),
        (5, priya, &[tri, spud], 240, 120),
        (6, ken, &[fluke], 240, 100),
        (7, sofia, &[heat], 480, 420),
        (8, jordan, &[tweez, picks], 240, 230),
        (9, ava, &[torque], 240, 200),
        (10, marcus, &[kit], 240, 260),
        (11, priya, &[usbm, p5], 240, 90),
    ];
    for &(n, who, tools, mins, back) in routine {
        d.at(day(n, 9) + (who % 3) * HOUR);
        let loans = d.lend(who, tools, mins)?;
        d.c.advance(back * MINUTE);
        for l in loans {
            // Ordinary returns come back as they left.
            let cond = crate::loans::get_loan(d.s.conn(), d.s.now(), l)?.checkout_condition;
            d.inspect(l, cond, "")?;
        }
        d.s.lock();
    }

    // Day 12: Jordan returns the spudgers worn — a damage incident, judged normal wear.
    d.at(day(12, 10));
    let l = d.lend(jordan, &[spud], 240)?;
    d.c.advance(2 * HOUR);
    if let Some(inc) = d.inspect(l[0], Worn, "Tips frayed")? {
        d.s.resolve_incident(inc, Outcome::NoFault, "Nylon spudgers wear out; normal use")?;
    }
    // Day 12: the soldering station (tech-only) comes back broken from Marcus.
    d.at(day(12, 14));
    let l = d.lend(marcus, &[iron], 240)?;
    d.c.advance(3 * HOUR);
    d.inspect(l[0], Broken, "Handpiece cable cut; tip melted into holder")?;
    // Day 13: Ava loses the torque set.
    d.at(day(13, 9));
    d.lend(ava, &[torque], 240)?;
    d.at(day(13, 17));
    d.unlock()?;
    d.s.mark_lost(
        torque,
        "Ava thinks it went out with a customer's laptop bag",
    )?;
    d.s.lock();

    // Today
    let now = now_ms;
    d.at(now - 5 * HOUR);
    d.lend(marcus, &[fluke], 180)?; // overdue by 2 h
    d.at(now - 3 * HOUR);
    let sofia_loans = d.lend(sofia, &[tweez, suction], 240)?;
    d.at(now - 2 * HOUR);
    d.lend(priya, &[kit], 240)?; // due in 2 h
    d.at(now - HOUR);
    d.lend(ken, &[heat], 480)?; // due this evening
    d.at(now - 40 * MINUTE);
    let t = d.token(sofia)?;
    d.s.drop_off(&t, &sofia_loans, "Left on the inspection shelf")?;
    d.at(now - 20 * MINUTE);
    let t = d.token(jordan)?;
    d.s.report_problem(&t, picks, "Two picks are bent — didn't take them", None)?;
    d.at(now - 10 * MINUTE);
    let t = d.token(ava)?;
    d.s.checkout(
        &t,
        &[CheckoutItem {
            tool_id: tri,
            due_at: Some(now + 3 * HOUR),
        }],
    )?;
    d.s.lock();
    Ok(())
}

pub fn is_demo(conn: &rusqlite::Connection) -> Result<bool> {
    Ok(settings::get_sys(conn, "demo")?.as_deref() == Some("1"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SystemClock;

    #[test]
    fn demo_database_is_complete_and_its_audit_log_verifies() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("demo.db");
        let now = 1_790_000_000_000;
        create(&path, now).unwrap();
        let clock = Arc::new(ManualClock::new(now));
        let mut s = Service::open(&path, clock).unwrap();
        assert!(s.is_demo().unwrap());
        let home = s.home().unwrap();
        assert_eq!(home.overdue.len(), 1, "Marcus's multimeter");
        assert_eq!(home.out.len(), 3);
        assert_eq!(home.awaiting.len(), 3, "two drop-offs and a problem report");
        assert!(home.open_incidents >= 3);
        s.unlock(1, TECH_PIN).unwrap();
        assert!(s.verify_audit().unwrap().ok);
        let report = s.report_borrowers(&Default::default()).unwrap();
        assert!(report.iter().any(|r| r.late_returns > 0));
        // Re-creating wipes and rebuilds.
        drop(s);
        create(&path, now).unwrap();
        let s = Service::open(&path, Arc::new(SystemClock)).unwrap();
        assert!(is_demo(s.conn()).unwrap());
    }
}
