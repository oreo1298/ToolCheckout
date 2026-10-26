-- ToolCheckout schema v1.
-- Timestamps are UTC Unix milliseconds. Money is integer cents.
-- Append-only tables are protected by triggers; the audit log is also
-- hash-chained so edits made with the triggers dropped are still detectable.

CREATE TABLE settings (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
) STRICT;

CREATE TABLE technicians (
  id         INTEGER PRIMARY KEY,
  name       TEXT    NOT NULL CHECK (length(trim(name)) > 0),
  pin_hash   TEXT    NOT NULL,
  active     INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
) STRICT;

CREATE TABLE borrowers (
  id          INTEGER PRIMARY KEY,
  name        TEXT    NOT NULL CHECK (length(trim(name)) > 0),
  employee_id TEXT    NOT NULL DEFAULT '',
  badge_code  TEXT    UNIQUE,
  pin_hash    TEXT,
  active      INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
  created_at  INTEGER NOT NULL,
  updated_at  INTEGER NOT NULL
) STRICT;

-- One row per PIN-protected subject: 'technician:<id>', 'borrower:<id>', 'recovery'.
CREATE TABLE pin_lockouts (
  subject         TEXT PRIMARY KEY,
  failed_attempts INTEGER NOT NULL DEFAULT 0,
  lockout_level   INTEGER NOT NULL DEFAULT 0,
  locked_until    INTEGER
) STRICT;

CREATE TABLE recovery_codes (
  id           INTEGER PRIMARY KEY,
  code_hash    TEXT    NOT NULL,
  created_at   INTEGER NOT NULL,
  used_at      INTEGER,
  used_by_tech INTEGER REFERENCES technicians (id),
  revoked_at   INTEGER
) STRICT;

CREATE TABLE photos (
  id         INTEGER PRIMARY KEY,
  mime       TEXT    NOT NULL CHECK (mime IN ('image/jpeg', 'image/png', 'image/webp')),
  data       BLOB    NOT NULL,
  created_at INTEGER NOT NULL
) STRICT;

CREATE TRIGGER photos_immutable BEFORE UPDATE ON photos
BEGIN
  SELECT RAISE(ABORT, 'photos are immutable');
END;

CREATE TABLE tools (
  id                     INTEGER PRIMARY KEY,
  asset_tag              TEXT    NOT NULL UNIQUE,
  name                   TEXT    NOT NULL CHECK (length(trim(name)) > 0),
  category               TEXT    NOT NULL DEFAULT '',
  description            TEXT    NOT NULL DEFAULT '',
  serial                 TEXT    NOT NULL DEFAULT '',
  model                  TEXT    NOT NULL DEFAULT '',
  photo_id               INTEGER REFERENCES photos (id),
  replacement_cost_cents INTEGER CHECK (replacement_cost_cents IS NULL OR replacement_cost_cents >= 0),
  home_location          TEXT    NOT NULL DEFAULT '',
  default_loan_minutes   INTEGER NOT NULL CHECK (default_loan_minutes > 0),
  tech_only              INTEGER NOT NULL DEFAULT 0 CHECK (tech_only IN (0, 1)),
  condition              TEXT    NOT NULL
    CHECK (condition IN ('new', 'good', 'fair', 'worn', 'damaged', 'broken')),
  condition_notes        TEXT    NOT NULL DEFAULT '',
  status                 TEXT    NOT NULL
    CHECK (status IN ('available', 'checked_out', 'awaiting_inspection', 'out_for_repair', 'lost', 'retired')),
  created_at             INTEGER NOT NULL,
  updated_at             INTEGER NOT NULL
) STRICT;

CREATE INDEX tools_status ON tools (status);

CREATE TABLE checkouts (
  id                 INTEGER PRIMARY KEY,
  batch_id           TEXT    NOT NULL,
  tool_id            INTEGER NOT NULL REFERENCES tools (id),
  borrower_id        INTEGER NOT NULL REFERENCES borrowers (id),
  checked_out_at     INTEGER NOT NULL,
  due_at             INTEGER NOT NULL,
  checkout_condition TEXT    NOT NULL
    CHECK (checkout_condition IN ('new', 'good', 'fair', 'worn', 'damaged', 'broken')),
  authorized_by_tech INTEGER REFERENCES technicians (id),
  state              TEXT    NOT NULL CHECK (state IN ('out', 'awaiting_inspection', 'closed', 'lost')),
  returned_at        INTEGER,
  dropoff_note       TEXT    NOT NULL DEFAULT '',
  inspected_at       INTEGER,
  inspected_by_tech  INTEGER REFERENCES technicians (id),
  return_condition   TEXT
    CHECK (return_condition IS NULL OR return_condition IN ('new', 'good', 'fair', 'worn', 'damaged', 'broken')),
  return_notes       TEXT    NOT NULL DEFAULT '',
  return_photo_id    INTEGER REFERENCES photos (id),
  CHECK (due_at > checked_out_at)
) STRICT;

-- A tool can only be on one open loan at a time.
CREATE UNIQUE INDEX checkouts_one_open_per_tool ON checkouts (tool_id)
  WHERE state IN ('out', 'awaiting_inspection');
CREATE INDEX checkouts_borrower ON checkouts (borrower_id, checked_out_at);
CREATE INDEX checkouts_state_due ON checkouts (state, due_at);
CREATE INDEX checkouts_tool ON checkouts (tool_id, checked_out_at);

CREATE TABLE condition_events (
  id          INTEGER PRIMARY KEY,
  tool_id     INTEGER NOT NULL REFERENCES tools (id),
  condition   TEXT    NOT NULL
    CHECK (condition IN ('new', 'good', 'fair', 'worn', 'damaged', 'broken')),
  notes       TEXT    NOT NULL DEFAULT '',
  photo_id    INTEGER REFERENCES photos (id),
  reason      TEXT    NOT NULL DEFAULT '',
  source      TEXT    NOT NULL
    CHECK (source IN ('initial', 'manual', 'return_inspection', 'inspection', 'repair', 'found')),
  checkout_id INTEGER REFERENCES checkouts (id),
  tech_id     INTEGER REFERENCES technicians (id),
  created_at  INTEGER NOT NULL
) STRICT;

CREATE INDEX condition_events_tool ON condition_events (tool_id, id);

CREATE TRIGGER condition_events_no_update BEFORE UPDATE ON condition_events
BEGIN
  SELECT RAISE(ABORT, 'condition history is append-only');
END;
CREATE TRIGGER condition_events_no_delete BEFORE DELETE ON condition_events
BEGIN
  SELECT RAISE(ABORT, 'condition history is append-only');
END;

CREATE TABLE incidents (
  id               INTEGER PRIMARY KEY,
  kind             TEXT    NOT NULL CHECK (kind IN ('damage', 'lost', 'problem_report')),
  tool_id          INTEGER NOT NULL REFERENCES tools (id),
  borrower_id      INTEGER REFERENCES borrowers (id),
  checkout_id      INTEGER REFERENCES checkouts (id),
  before_condition TEXT,
  after_condition  TEXT,
  notes            TEXT    NOT NULL DEFAULT '',
  photo_ids        TEXT    NOT NULL DEFAULT '[]',
  created_at       INTEGER NOT NULL,
  created_by_kind  TEXT    NOT NULL CHECK (created_by_kind IN ('technician', 'borrower', 'system')),
  created_by_id    INTEGER,
  status           TEXT    NOT NULL DEFAULT 'open' CHECK (status IN ('open', 'resolved')),
  outcome          TEXT,
  resolution_note  TEXT,
  resolved_at      INTEGER,
  resolved_by_tech INTEGER REFERENCES technicians (id)
) STRICT;

CREATE INDEX incidents_status ON incidents (status, created_at);
CREATE INDEX incidents_borrower ON incidents (borrower_id);
CREATE INDEX incidents_tool ON incidents (tool_id);

CREATE TABLE audit_log (
  id          INTEGER PRIMARY KEY,
  ts          INTEGER NOT NULL,
  actor_kind  TEXT    NOT NULL CHECK (actor_kind IN ('technician', 'borrower', 'system')),
  actor_id    INTEGER,
  actor_name  TEXT    NOT NULL,
  action      TEXT    NOT NULL,
  entity_kind TEXT    NOT NULL,
  entity_id   INTEGER,
  before_json TEXT,
  after_json  TEXT,
  prev_hash   TEXT    NOT NULL,
  hash        TEXT    NOT NULL UNIQUE
) STRICT;

CREATE INDEX audit_log_entity ON audit_log (entity_kind, entity_id);

CREATE TRIGGER audit_log_no_update BEFORE UPDATE ON audit_log
BEGIN
  SELECT RAISE(ABORT, 'audit log is append-only');
END;
CREATE TRIGGER audit_log_no_delete BEFORE DELETE ON audit_log
BEGIN
  SELECT RAISE(ABORT, 'audit log is append-only');
END;
