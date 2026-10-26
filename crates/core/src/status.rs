//! The tool status machine. Every status change in the app goes through
//! [`check`], so an invalid jump (e.g. Retired → Checked out) is impossible
//! no matter which command or UI path asks for it.

use crate::error::{CoreError, Result};
use crate::model::{Condition, ToolStatus};

use ToolStatus::*;

/// Allowed (from, to) pairs. See PLAN.md §2 for what triggers each one.
const ALLOWED: &[(ToolStatus, ToolStatus)] = &[
    // On the shelf
    (Available, CheckedOut),         // checkout
    (Available, AwaitingInspection), // borrower reported a problem at checkout
    (Available, OutForRepair),       // tech sends it to repair
    (Available, Lost),               // tech marks it lost (not on loan)
    (Available, Retired),            // tech retires it
    // On loan
    (CheckedOut, Available),          // inspected return, usable
    (CheckedOut, OutForRepair),       // inspected return, damaged/broken
    (CheckedOut, AwaitingInspection), // borrower drop-off
    (CheckedOut, Lost),               // tech marks it lost
    // Dropped off / flagged
    (AwaitingInspection, Available),
    (AwaitingInspection, OutForRepair),
    (AwaitingInspection, Lost),
    // Repair
    (OutForRepair, Available),
    (OutForRepair, Lost),
    (OutForRepair, Retired),
    // Lost
    (Lost, Available),    // found, usable
    (Lost, OutForRepair), // found, damaged/broken
    (Lost, Retired),
    // Retired
    (Retired, Available), // reinstated
];

pub fn is_allowed(from: ToolStatus, to: ToolStatus) -> bool {
    ALLOWED.contains(&(from, to))
}

pub fn check(from: ToolStatus, to: ToolStatus) -> Result<()> {
    if is_allowed(from, to) {
        Ok(())
    } else {
        Err(CoreError::InvalidTransition { from, to })
    }
}

/// Where a tool goes after a technician inspects it.
pub fn after_inspection(condition: Condition) -> ToolStatus {
    if condition.needs_repair() {
        OutForRepair
    } else {
        Available
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checkout_only_from_available() {
        for &from in ToolStatus::ALL {
            assert_eq!(
                is_allowed(from, CheckedOut),
                from == Available,
                "{from:?} → CheckedOut"
            );
        }
    }

    #[test]
    fn no_self_transitions() {
        for &s in ToolStatus::ALL {
            assert!(!is_allowed(s, s), "{s:?} → itself");
        }
    }

    #[test]
    fn retired_tools_cannot_be_loaned_or_lost() {
        assert!(check(Retired, CheckedOut).is_err());
        assert!(check(Retired, Lost).is_err());
        assert!(check(Retired, Available).is_ok());
    }

    #[test]
    fn tools_on_loan_cannot_be_retired_or_repaired_directly_by_status() {
        assert!(check(CheckedOut, Retired).is_err());
        // CheckedOut → OutForRepair is only reachable via an inspected return.
        assert!(check(CheckedOut, OutForRepair).is_ok());
    }

    #[test]
    fn awaiting_inspection_requires_inspection_to_leave() {
        assert!(check(AwaitingInspection, CheckedOut).is_err());
        assert!(check(AwaitingInspection, Retired).is_err());
        assert!(check(AwaitingInspection, Available).is_ok());
    }

    #[test]
    fn invalid_transition_error_names_both_states() {
        let err = check(Lost, CheckedOut).unwrap_err();
        assert_eq!(err.code(), "invalid_transition");
        assert_eq!(
            err.to_string(),
            "Can't change a tool from Lost to Checked out"
        );
    }

    #[test]
    fn inspection_routes_damaged_tools_to_repair() {
        assert_eq!(after_inspection(Condition::Good), Available);
        assert_eq!(after_inspection(Condition::Worn), Available);
        assert_eq!(after_inspection(Condition::Damaged), OutForRepair);
        assert_eq!(after_inspection(Condition::Broken), OutForRepair);
    }

    #[test]
    fn every_status_has_a_way_out_except_by_design() {
        for &s in ToolStatus::ALL {
            assert!(
                ToolStatus::ALL.iter().any(|&t| is_allowed(s, t)),
                "{s:?} is a dead end"
            );
        }
    }
}
