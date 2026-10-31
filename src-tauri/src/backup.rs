//! Adatbázis-mentés és -visszaállítás, opcionális jelszavas titkosítással.
//!
//! A mentés a teljes SQLite adatbázis (`VACUUM INTO`). Jelszóval a fájl formátuma:
//! `KMBACKUP1` + 16 bájt só + 12 bájt nonce + ChaCha20-Poly1305 titkosított adatbázis; a kulcsot
//! Argon2id származtatja a jelszóból.

use chacha20poly1305::aead::rand_core::RngCore;
use chacha20poly1305::aead::{Aead, KeyInit, OsRng};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use rusqlite::Connection;
use serde::Serialize;
use std::path::{Path, PathBuf};

const MAGIC: &[u8] = b"KMBACKUP1";
const SQLITE_HEADER: &[u8] = b"SQLite format 3\0";
/// Ennyi automatikus mentést őrzünk meg profilonként.
const KEEP_AUTO: usize = 8;
/// Ilyen gyakran készül automatikus mentés (napokban).
pub const AUTO_EVERY_DAYS: i64 = 7;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupFile {
    pub path: String,
    pub name: String,
    /// A mentés napja (a fájlnévből).
    pub date: String,
    pub size: u64,
}

fn key_from(password: &str, salt: &[u8]) -> Result<Key, String> {
    let mut key = [0u8; 32];
    argon2::Argon2::default()
        .hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|e| format!("Kulcsképzési hiba: {e}"))?;
    Ok(*Key::from_slice(&key))
}

pub fn encrypt(plain: &[u8], password: &str) -> Result<Vec<u8>, String> {
    let mut salt = [0u8; 16];
    let mut nonce = [0u8; 12];
    OsRng.fill_bytes(&mut salt);
    OsRng.fill_bytes(&mut nonce);
    let cipher = ChaCha20Poly1305::new(&key_from(password, &salt)?);
    let data = cipher.encrypt(Nonce::from_slice(&nonce), plain).map_err(|_| "Titkosítási hiba.")?;
    Ok([MAGIC, &salt, &nonce, &data].concat())
}

pub fn decrypt(bytes: &[u8], password: &str) -> Result<Vec<u8>, String> {
    let rest = &bytes[MAGIC.len()..];
    if rest.len() < 28 {
        return Err("Sérült mentésfájl.".into());
    }
    let (salt, rest) = rest.split_at(16);
    let (nonce, data) = rest.split_at(12);
    let cipher = ChaCha20Poly1305::new(&key_from(password, salt)?);
    cipher
        .decrypt(Nonce::from_slice(nonce), data)
        .map_err(|_| "Hibás jelszó, vagy a mentésfájl sérült.".into())
}

pub fn is_encrypted(path: &Path) -> Result<bool, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("Nem sikerült megnyitni: {e}"))?;
    Ok(bytes.starts_with(MAGIC))
}

/// Az adatbázis pillanatképe a megadott fájlba (titkosítva, ha van jelszó).
pub fn write(conn: &Connection, dest: &Path, password: Option<&str>) -> Result<(), String> {
    let password = password.filter(|p| !p.is_empty());
    let tmp = dest.with_extension("tmp-backup");
    let _ = std::fs::remove_file(&tmp);
    conn.execute("VACUUM INTO ?1", [tmp.to_string_lossy()]).map_err(|e| format!("Mentési hiba: {e}"))?;
    let result = match password {
        None => std::fs::rename(&tmp, dest).or_else(|_| std::fs::copy(&tmp, dest).map(|_| ())),
        Some(p) => {
            let plain = std::fs::read(&tmp).map_err(|e| e.to_string())?;
            std::fs::write(dest, encrypt(&plain, p)?)
        }
    };
    let _ = std::fs::remove_file(&tmp);
    result.map_err(|e| format!("Mentési hiba: {e}"))
}

/// Mentésfájl beolvasása egy ideiglenes, ellenőrzött SQLite fájlba, amely a visszaállításhoz kész.
pub fn read_to_temp(src: &Path, password: Option<&str>, tmp: &Path) -> Result<(), String> {
    let bytes = std::fs::read(src).map_err(|e| format!("Nem sikerült megnyitni: {e}"))?;
    let plain = if bytes.starts_with(MAGIC) {
        let p = password.filter(|p| !p.is_empty()).ok_or("Ez a mentés jelszóval titkosított, add meg a jelszót.")?;
        decrypt(&bytes, p)?
    } else {
        bytes
    };
    if !plain.starts_with(SQLITE_HEADER) {
        return Err("Ez nem Kriptomérleg mentés (nem SQLite adatbázis).".into());
    }
    std::fs::write(tmp, &plain).map_err(|e| e.to_string())?;
    let conn = Connection::open(tmp).map_err(|e| e.to_string())?;
    let ok: bool = conn
        .query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'transactions')", [], |r| {
            r.get(0)
        })
        .map_err(|e| e.to_string())?;
    if !ok {
        let _ = std::fs::remove_file(tmp);
        return Err("A fájlban nincs Kriptomérleg tételtábla.".into());
    }
    Ok(())
}

/// `<profil>-ÉÉÉÉ-HH-NN.sqlite` alakú automatikus mentések, legújabb elöl.
pub fn list(dir: &Path, profile: &str) -> Vec<BackupFile> {
    let prefix = format!("{profile}-");
    let mut files: Vec<BackupFile> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            let date = name.strip_prefix(&prefix)?.strip_suffix(".sqlite")?;
            if date.len() != 10 || chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").is_err() {
                return None;
            }
            Some(BackupFile {
                path: e.path().to_string_lossy().to_string(),
                date: date.to_string(),
                size: e.metadata().map(|m| m.len()).unwrap_or(0),
                name,
            })
        })
        .collect();
    files.sort_by(|a, b| b.date.cmp(&a.date));
    files
}

/// Heti automatikus mentés, ha az utolsó régebbi `AUTO_EVERY_DAYS` napnál; a régieket törli.
pub fn auto(conn: &Connection, dir: &Path, profile: &str, today: chrono::NaiveDate) -> Result<Option<PathBuf>, String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let existing = list(dir, profile);
    let recent = existing.first().and_then(|b| chrono::NaiveDate::parse_from_str(&b.date, "%Y-%m-%d").ok());
    if recent.is_some_and(|d| (today - d).num_days() < AUTO_EVERY_DAYS) {
        return Ok(None);
    }
    let has_data: bool = conn
        .query_row("SELECT EXISTS(SELECT 1 FROM transactions)", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if !has_data {
        return Ok(None);
    }
    let dest = dir.join(format!("{profile}-{today}.sqlite"));
    write(conn, &dest, None)?;
    for old in list(dir, profile).into_iter().skip(KEEP_AUTO) {
        let _ = std::fs::remove_file(old.path);
    }
    Ok(Some(dest))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encryption_roundtrip() {
        let data = b"SQLite format 3\0 tetszoleges adat".to_vec();
        let enc = encrypt(&data, "titok").unwrap();
        assert!(enc.starts_with(MAGIC));
        assert_eq!(decrypt(&enc, "titok").unwrap(), data);
        assert!(decrypt(&enc, "rossz").is_err());
    }

    #[test]
    fn backup_restore_and_rotation() {
        let dir = std::env::temp_dir().join(format!("kriptomerleg-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let conn = Connection::open(dir.join("db.sqlite")).unwrap();
        conn.execute_batch("CREATE TABLE transactions (id INTEGER PRIMARY KEY, huf INTEGER); INSERT INTO transactions (huf) VALUES (5);")
            .unwrap();

        let enc = dir.join("mentes.kmbackup");
        write(&conn, &enc, Some("jelszo")).unwrap();
        assert!(is_encrypted(&enc).unwrap());
        let tmp = dir.join("restore.sqlite");
        assert!(read_to_temp(&enc, None, &tmp).is_err());
        read_to_temp(&enc, Some("jelszo"), &tmp).unwrap();
        let restored = Connection::open(&tmp).unwrap();
        let huf: i64 = restored.query_row("SELECT huf FROM transactions", [], |r| r.get(0)).unwrap();
        assert_eq!(huf, 5);

        let backups = dir.join("backups");
        let day = |d: u32| chrono::NaiveDate::from_ymd_opt(2026, 1, d).unwrap();
        assert!(auto(&conn, &backups, "sajat", day(1)).unwrap().is_some());
        assert!(auto(&conn, &backups, "sajat", day(5)).unwrap().is_none());
        for d in (8..=29).step_by(7) {
            auto(&conn, &backups, "sajat", day(d)).unwrap();
        }
        for i in 0..6 {
            std::fs::write(backups.join(format!("sajat-2025-0{}-01.sqlite", i + 1)), b"x").unwrap();
        }
        auto(&conn, &backups, "sajat", chrono::NaiveDate::from_ymd_opt(2026, 3, 1).unwrap()).unwrap();
        let kept = list(&backups, "sajat");
        assert_eq!(kept.len(), KEEP_AUTO);
        assert_eq!(kept[0].date, "2026-03-01");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
