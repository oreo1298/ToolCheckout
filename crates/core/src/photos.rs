//! Photos are stored inside the database (immutable rows), so a backup is a
//! single file and condition evidence can't silently change. The UI
//! downscales to JPEG before upload, keeping them small.

use rusqlite::{Connection, OptionalExtension, params};

use crate::error::{Result, invalid, not_found};

pub const MAX_BYTES: usize = 4 * 1024 * 1024;

pub fn mime_for(bytes: &[u8]) -> Option<&'static str> {
    match bytes {
        [0xFF, 0xD8, 0xFF, ..] => Some("image/jpeg"),
        [0x89, b'P', b'N', b'G', ..] => Some("image/png"),
        b if b.len() >= 12 && b.starts_with(b"RIFF") && &b[8..12] == b"WEBP" => Some("image/webp"),
        _ => None,
    }
}

/// Stores a photo after checking it really is a JPEG/PNG/WebP (by content,
/// not by what the caller claims).
pub fn save(conn: &Connection, now: i64, bytes: &[u8]) -> Result<i64> {
    if bytes.is_empty() {
        return invalid("The photo is empty");
    }
    if bytes.len() > MAX_BYTES {
        return invalid("The photo is too large (4 MB max)");
    }
    let Some(mime) = mime_for(bytes) else {
        return invalid("Photos must be JPEG, PNG or WebP images");
    };
    conn.execute(
        "INSERT INTO photos (mime, data, created_at) VALUES (?1, ?2, ?3)",
        params![mime, bytes, now],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get(conn: &Connection, id: i64) -> Result<(String, Vec<u8>)> {
    conn.query_row("SELECT mime, data FROM photos WHERE id = ?1", [id], |r| {
        Ok((r.get(0)?, r.get(1)?))
    })
    .optional()?
    .ok_or_else(|| not_found("Photo"))
}

pub fn exists(conn: &Connection, id: i64) -> Result<bool> {
    Ok(conn
        .query_row("SELECT 1 FROM photos WHERE id = ?1", [id], |_| Ok(()))
        .optional()?
        .is_some())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    const JPEG: &[u8] = &[0xFF, 0xD8, 0xFF, 0xE0, 0, 0x10, b'J', b'F', b'I', b'F'];

    #[test]
    fn stores_by_sniffed_type() {
        let conn = db::open_in_memory().unwrap();
        let id = save(&conn, 1, JPEG).unwrap();
        let (mime, data) = get(&conn, id).unwrap();
        assert_eq!(mime, "image/jpeg");
        assert_eq!(data, JPEG);
        assert!(save(&conn, 1, b"<svg onload=alert(1)>").is_err());
        assert!(save(&conn, 1, &[]).is_err());
    }

    #[test]
    fn photos_are_immutable() {
        let conn = db::open_in_memory().unwrap();
        let id = save(&conn, 1, JPEG).unwrap();
        assert!(
            conn.execute("UPDATE photos SET data = x'00' WHERE id = ?1", [id])
                .is_err()
        );
    }
}
