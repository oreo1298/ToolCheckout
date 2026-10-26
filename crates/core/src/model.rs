//! Domain enumerations shared by every module. Each is stored in SQLite as a
//! lowercase snake_case string and serialized to the UI the same way.

use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
use serde::{Deserialize, Serialize};

macro_rules! str_enum {
    ($(#[$m:meta])* $name:ident { $($variant:ident => $s:literal, $label:literal;)+ }) => {
        $(#[$m])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub enum $name { $($variant),+ }

        impl $name {
            pub const ALL: &'static [$name] = &[$($name::$variant),+];

            pub fn as_str(self) -> &'static str {
                match self { $($name::$variant => $s),+ }
            }

            /// Human-readable label used in CSV exports and messages.
            pub fn label(self) -> &'static str {
                match self { $($name::$variant => $label),+ }
            }

            pub fn parse(s: &str) -> Option<Self> {
                match s { $($s => Some($name::$variant),)+ _ => None }
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl ToSql for $name {
            fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
                Ok(ToSqlOutput::from(self.as_str()))
            }
        }

        impl FromSql for $name {
            fn column_result(v: ValueRef<'_>) -> FromSqlResult<Self> {
                let s = v.as_str()?;
                $name::parse(s).ok_or_else(|| FromSqlError::Other(
                    format!("invalid {} value {s:?}", stringify!($name)).into(),
                ))
            }
        }
    };
}

str_enum! {
    /// Ordered worst-last, so a higher rank on return means a downgrade.
    Condition {
        New => "new", "New";
        Good => "good", "Good";
        Fair => "fair", "Fair";
        Worn => "worn", "Worn";
        Damaged => "damaged", "Damaged";
        Broken => "broken", "Broken";
    }
}

impl Condition {
    pub fn rank(self) -> u8 {
        match self {
            Condition::New => 0,
            Condition::Good => 1,
            Condition::Fair => 2,
            Condition::Worn => 3,
            Condition::Damaged => 4,
            Condition::Broken => 5,
        }
    }

    /// True when `self` is strictly worse than `before`.
    pub fn is_downgrade_from(self, before: Condition) -> bool {
        self.rank() > before.rank()
    }

    /// Damaged and Broken tools go to repair instead of back on the shelf.
    pub fn needs_repair(self) -> bool {
        matches!(self, Condition::Damaged | Condition::Broken)
    }
}

str_enum! {
    ToolStatus {
        Available => "available", "Available";
        CheckedOut => "checked_out", "Checked out";
        AwaitingInspection => "awaiting_inspection", "Returned – awaiting inspection";
        OutForRepair => "out_for_repair", "Out for repair";
        Lost => "lost", "Lost";
        Retired => "retired", "Retired";
    }
}

str_enum! {
    CheckoutState {
        Out => "out", "Out";
        AwaitingInspection => "awaiting_inspection", "Awaiting inspection";
        Closed => "closed", "Returned";
        Lost => "lost", "Lost";
    }
}

str_enum! {
    IncidentKind {
        Damage => "damage", "Damage";
        Lost => "lost", "Lost";
        ProblemReport => "problem_report", "Problem reported";
    }
}

str_enum! {
    IncidentStatus {
        Open => "open", "Open";
        Resolved => "resolved", "Resolved";
    }
}

str_enum! {
    ConditionSource {
        Initial => "initial", "Added";
        Manual => "manual", "Condition update";
        ReturnInspection => "return_inspection", "Return inspection";
        Inspection => "inspection", "Inspection";
        Repair => "repair", "Back from repair";
        Found => "found", "Found";
    }
}

str_enum! {
    /// Incident outcomes a technician can choose when resolving.
    Outcome {
        NoFault => "no_fault", "Normal wear – no fault";
        BorrowerDamaged => "borrower_damaged", "Damaged by borrower";
        Repaired => "repaired", "Repaired";
        Replaced => "replaced", "Replaced";
        BorrowerPaid => "borrower_paid", "Borrower paid";
        Found => "found", "Found";
        WrittenOff => "written_off", "Written off";
        Other => "other", "Other";
    }
}

impl Outcome {
    /// Outcomes that clear the borrower: excluded from cost-exposure totals.
    pub fn clears_borrower(self) -> bool {
        matches!(self, Outcome::NoFault | Outcome::Found)
    }
}

/// Who performed an action. Recorded on every audit entry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Actor {
    Technician { id: i64, name: String },
    Borrower { id: i64, name: String },
    System,
}

impl Actor {
    pub fn kind(&self) -> &'static str {
        match self {
            Actor::Technician { .. } => "technician",
            Actor::Borrower { .. } => "borrower",
            Actor::System => "system",
        }
    }

    pub fn id(&self) -> Option<i64> {
        match self {
            Actor::Technician { id, .. } | Actor::Borrower { id, .. } => Some(*id),
            Actor::System => None,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Actor::Technician { name, .. } | Actor::Borrower { name, .. } => name,
            Actor::System => "System",
        }
    }

    pub fn tech_id(&self) -> Option<i64> {
        match self {
            Actor::Technician { id, .. } => Some(*id),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn condition_order_and_downgrade() {
        let order: Vec<_> = Condition::ALL.iter().map(|c| c.rank()).collect();
        assert_eq!(order, vec![0, 1, 2, 3, 4, 5]);
        assert!(Condition::Worn.is_downgrade_from(Condition::Good));
        assert!(!Condition::Good.is_downgrade_from(Condition::Good));
        assert!(!Condition::New.is_downgrade_from(Condition::Fair));
        assert!(Condition::Broken.needs_repair() && Condition::Damaged.needs_repair());
        assert!(!Condition::Worn.needs_repair());
    }

    #[test]
    fn round_trips_strings() {
        for s in ToolStatus::ALL {
            assert_eq!(ToolStatus::parse(s.as_str()), Some(*s));
        }
        assert_eq!(Condition::parse("excellent"), None);
        assert_eq!(
            serde_json::to_string(&ToolStatus::AwaitingInspection).unwrap(),
            "\"awaiting_inspection\""
        );
    }
}
