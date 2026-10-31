//! Adatkezelő parancsok: Excel export, beállítások, mentés és visszaállítás, profilok.

use super::*;

/* ───────────── Excel export ───────────── */

/// Excel munkafüzet: „Tételek” és „Éves összesítő” lap, számként tárolt összegekkel.
#[tauri::command(async)]
pub fn export_xlsx(state: State<'_, AppState>, path: String) -> Result<(), String> {
    let conn = state.db.lock().unwrap();
    let txs = all_transactions(&conn, None)?;
    let years = compute_summary(&conn)?;
    drop(conn);
    write_xlsx(&txs, &years, Path::new(&path))
}

pub(crate) fn write_xlsx(txs: &[Tx], years: &[tax::YearSummary], path: &Path) -> Result<(), String> {
    use rust_xlsxwriter::{Format, Workbook, XlsxError};
    let x = |e: XlsxError| e.to_string();
    let head = Format::new().set_bold().set_background_color("#E6EFEE");
    let ft = Format::new().set_num_format("# ##0");
    let money = Format::new().set_num_format("# ##0.00");
    let rate = Format::new().set_num_format("0.0000");
    let mut wb = Workbook::new();

    let ws = wb.add_worksheet().set_name("Tételek").map_err(x)?;
    let heads = [
        "Dátum", "Típus", "Forrás", "Deviza", "Összeg", "Árfolyam", "Árfolyam forrása", "Árfolyam napja", "Forint", "Díj", "Megjegyzés",
    ];
    for (c, h) in heads.iter().enumerate() {
        ws.write_string_with_format(0, c as u16, *h, &head).map_err(x)?;
    }
    for (i, t) in txs.iter().rev().enumerate() {
        let r = i as u32 + 1;
        ws.write_string(r, 0, &t.date).map_err(x)?;
        ws.write_string(r, 1, kind_label(&t.kind, t.amount)).map_err(x)?;
        ws.write_string(r, 2, &t.source).map_err(x)?;
        ws.write_string(r, 3, &t.currency).map_err(x)?;
        ws.write_number_with_format(r, 4, t.amount, &money).map_err(x)?;
        ws.write_number_with_format(r, 5, t.rate, &rate).map_err(x)?;
        ws.write_string(r, 6, &t.rate_source).map_err(x)?;
        ws.write_string(r, 7, t.rate_date.as_deref().unwrap_or("")).map_err(x)?;
        ws.write_number_with_format(r, 8, t.huf as f64, &ft).map_err(x)?;
        ws.write_number_with_format(r, 9, t.fee, &money).map_err(x)?;
        ws.write_string(r, 10, &t.note).map_err(x)?;
    }
    for (c, w) in [12.0, 18.0, 10.0, 8.0, 14.0, 11.0, 15.0, 14.0, 14.0, 10.0, 40.0].iter().enumerate() {
        ws.set_column_width(c as u16, *w).map_err(x)?;
    }
    ws.set_freeze_panes(1, 0).map_err(x)?;
    ws.autofilter(0, 0, txs.len() as u32, heads.len() as u16 - 1).map_err(x)?;

    let ws = wb.add_worksheet().set_name("Éves összesítő").map_err(x)?;
    let heads = [
        "Év",
        "Befizetve (Ft)",
        "Kivéve (Ft)",
        "Eredmény (Ft)",
        "Adó 15% (Ft)",
        "Adókiegyenlítés (Ft)",
        "Fizetendő, negatív = visszajár (Ft)",
        "Veszteség 15%-a (Ft)",
        "Ebből fel nem használt (Ft)",
        "Megjegyzés",
    ];
    for (c, h) in heads.iter().enumerate() {
        ws.write_string_with_format(0, c as u16, *h, &head).map_err(x)?;
    }
    for (i, y) in years.iter().enumerate() {
        let r = i as u32 + 1;
        ws.write_number(r, 0, y.year as f64).map_err(x)?;
        let nums = [
            y.deposits_huf,
            y.withdrawals_huf,
            y.result_huf,
            y.tax_gross,
            y.offset_used,
            y.tax_due,
            y.loss_credit,
            y.loss_credit_remaining,
        ];
        for (c, v) in nums.iter().enumerate() {
            ws.write_number_with_format(r, c as u16 + 1, *v as f64, &ft).map_err(x)?;
        }
        ws.write_string(r, 9, year_remark(y)).map_err(x)?;
    }
    for c in 0..heads.len() as u16 {
        ws.set_column_width(c, if c == 0 { 8.0 } else { 18.0 }).map_err(x)?;
    }
    ws.set_freeze_panes(1, 0).map_err(x)?;
    wb.save(path).map_err(x)
}

/* ───────────── PDF nyilvántartás ───────────── */

/// Az év nyilvántartása PDF-ben: összesítő, veszteség-nyilvántartás, tételek.
#[tauri::command(async)]
pub fn export_pdf(state: State<'_, AppState>, path: String, year: i32) -> Result<(), String> {
    let conn = state.db.lock().unwrap();
    let mut txs = all_transactions(&conn, Some(year))?;
    txs.reverse();
    let years = compute_summary(&conn)?;
    drop(conn);
    let id = state.profile.lock().unwrap().clone();
    let profile = profile_names(&state.dir).get(&id).cloned().unwrap_or(id);
    let report = pdf::Report {
        year,
        profile: &profile,
        generated: Local::now().format("%Y-%m-%d %H:%M").to_string(),
        years: &years,
        txs: &txs,
    };
    pdf::write(&report, Path::new(&path))
}

/// Szöveges fájl mentése UTF-8 BOM-mal (pl. a bevallási sorok CSV-je, amit a felület állít össze).
#[tauri::command(async)]
pub fn save_text(path: String, text: String) -> Result<(), String> {
    std::fs::write(&path, [&[0xEF_u8, 0xBB, 0xBF][..], text.as_bytes()].concat()).map_err(err)
}

/* ───────────── Beállítások ───────────── */

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<BTreeMap<String, String>, String> {
    state
        .db
        .lock()
        .unwrap()
        .prepare("SELECT key, value FROM settings")
        .and_then(|mut s| s.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?.collect())
        .map_err(err)
}

#[tauri::command]
pub fn set_setting(state: State<'_, AppState>, key: String, value: String) -> Result<(), String> {
    state
        .db
        .lock()
        .unwrap()
        .execute("INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)", params![key, value])
        .map_err(err)?;
    Ok(())
}

/* ───────────── Mentés és visszaállítás ───────────── */

fn backups_dir(state: &AppState) -> PathBuf {
    state.dir.join("backups")
}

#[tauri::command(async)]
pub fn backup_save(state: State<'_, AppState>, path: String, password: Option<String>) -> Result<(), String> {
    backup::write(&state.db.lock().unwrap(), Path::new(&path), password.as_deref())
}

#[tauri::command]
pub fn backup_list(state: State<'_, AppState>) -> Vec<backup::BackupFile> {
    backup::list(&backups_dir(&state), &state.profile.lock().unwrap())
}

#[tauri::command]
pub fn backup_dir(state: State<'_, AppState>) -> Result<String, String> {
    let dir = backups_dir(&state);
    std::fs::create_dir_all(&dir).map_err(err)?;
    Ok(dir.to_string_lossy().to_string())
}

#[tauri::command]
pub fn backup_is_encrypted(path: String) -> Result<bool, String> {
    backup::is_encrypted(Path::new(&path))
}

/// Visszaállítás mentésből. Előtte a jelenlegi állapotról is mentés készül a mentések mappájába.
#[tauri::command(async)]
pub fn backup_restore(state: State<'_, AppState>, path: String, password: Option<String>) -> Result<(), String> {
    let profile = state.profile.lock().unwrap().clone();
    let tmp = state.dir.join("restore.tmp.sqlite");
    backup::read_to_temp(Path::new(&path), password.as_deref(), &tmp)?;
    let mut conn = state.db.lock().unwrap();
    let dir = backups_dir(&state);
    std::fs::create_dir_all(&dir).map_err(err)?;
    let stamp = Local::now().format("%Y-%m-%d_%H%M%S");
    backup::write(&conn, &dir.join(format!("{profile}-visszaallitas-elott-{stamp}.sqlite")), None)?;
    let target = db_path(&state.dir, &profile);
    *conn = Connection::open_in_memory().map_err(err)?; // a régi fájl bezárása (Windowson enélkül nem írható felül)
    let copied = std::fs::copy(&tmp, &target);
    let _ = std::fs::remove_file(&tmp);
    *conn = open_db(&target)?;
    copied.map(|_| ()).map_err(err)
}

/* ───────────── Profilok ───────────── */

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    id: String,
    name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Profiles {
    current: String,
    profiles: Vec<Profile>,
}

/// Profilazonosító → megjelenített név (`profiles.json`); a Saját profil mindig létezik.
pub(crate) fn profile_names(dir: &Path) -> BTreeMap<String, String> {
    let mut names: BTreeMap<String, String> = std::fs::read_to_string(dir.join("profiles.json"))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    names.entry(DEFAULT_PROFILE.into()).or_insert_with(|| "Saját".into());
    names
}

fn save_profile_names(dir: &Path, names: &BTreeMap<String, String>) -> Result<(), String> {
    std::fs::write(dir.join("profiles.json"), serde_json::to_string_pretty(names).map_err(err)?).map_err(err)
}

/// Fájlnévnek alkalmas azonosító a névből ("Anna Kovács" → "anna-kovacs").
fn slug(name: &str) -> String {
    let mut out = String::new();
    for c in name.trim().to_lowercase().chars() {
        let c = match c {
            'á' => 'a',
            'é' => 'e',
            'í' => 'i',
            'ó' | 'ö' | 'ő' => 'o',
            'ú' | 'ü' | 'ű' => 'u',
            c => c,
        };
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    let out = out.trim_end_matches('-');
    if out.is_empty() { "profil".into() } else { out.into() }
}

fn profiles_of(state: &AppState) -> Profiles {
    Profiles {
        current: state.profile.lock().unwrap().clone(),
        profiles: profile_names(&state.dir).into_iter().map(|(id, name)| Profile { id, name }).collect(),
    }
}

fn activate_profile(state: &AppState, id: &str) -> Result<(), String> {
    let conn = open_db(&db_path(&state.dir, id))?;
    let _ = backup::auto(&conn, &backups_dir(state), id, Local::now().date_naive());
    *state.db.lock().unwrap() = conn;
    *state.profile.lock().unwrap() = id.to_string();
    std::fs::write(state.dir.join("current-profile"), id).map_err(err)
}

#[tauri::command]
pub fn list_profiles(state: State<'_, AppState>) -> Profiles {
    profiles_of(&state)
}

#[tauri::command]
pub fn create_profile(state: State<'_, AppState>, name: String) -> Result<Profiles, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Adj nevet a profilnak.".into());
    }
    let mut names = profile_names(&state.dir);
    let base = slug(name);
    let mut id = base.clone();
    let mut n = 2;
    while names.contains_key(&id) || db_path(&state.dir, &id).exists() {
        id = format!("{base}-{n}");
        n += 1;
    }
    names.insert(id.clone(), name.to_string());
    save_profile_names(&state.dir, &names)?;
    activate_profile(&state, &id)?;
    Ok(profiles_of(&state))
}

#[tauri::command]
pub fn switch_profile(state: State<'_, AppState>, id: String) -> Result<Profiles, String> {
    if !profile_names(&state.dir).contains_key(&id) {
        return Err("Nincs ilyen profil.".into());
    }
    activate_profile(&state, &id)?;
    Ok(profiles_of(&state))
}

#[tauri::command]
pub fn rename_profile(state: State<'_, AppState>, id: String, name: String) -> Result<Profiles, String> {
    if name.trim().is_empty() {
        return Err("Adj nevet a profilnak.".into());
    }
    let mut names = profile_names(&state.dir);
    *names.get_mut(&id).ok_or("Nincs ilyen profil.")? = name.trim().to_string();
    save_profile_names(&state.dir, &names)?;
    Ok(profiles_of(&state))
}

/// Profil törlése. Az adatbázisa előtte a mentések mappájába kerül, így visszaállítható.
#[tauri::command]
pub fn delete_profile(state: State<'_, AppState>, id: String) -> Result<Profiles, String> {
    if id == DEFAULT_PROFILE {
        return Err("A Saját profil nem törölhető.".into());
    }
    if *state.profile.lock().unwrap() == id {
        return Err("Az aktív profil nem törölhető; előbb válts másikra.".into());
    }
    let mut names = profile_names(&state.dir);
    names.remove(&id).ok_or("Nincs ilyen profil.")?;
    let path = db_path(&state.dir, &id);
    if path.exists() {
        let dir = backups_dir(&state);
        std::fs::create_dir_all(&dir).map_err(err)?;
        let stamp = Local::now().format("%Y-%m-%d_%H%M%S");
        std::fs::copy(&path, dir.join(format!("{id}-torolt-profil-{stamp}.sqlite"))).map_err(err)?;
        std::fs::remove_file(&path).map_err(err)?;
    }
    save_profile_names(&state.dir, &names)?;
    Ok(profiles_of(&state))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs() {
        assert_eq!(slug("Anna Kovács"), "anna-kovacs");
        assert_eq!(slug("  Őrült   Ügyfél! "), "orult-ugyfel");
        assert_eq!(slug("!!!"), "profil");
    }

    #[test]
    fn writes_xlsx() {
        let years = tax::summarize(&BTreeMap::from([(2026, (100_000, 250_000))]), 2026, &BTreeSet::new());
        let txs = vec![Tx {
            id: 1,
            date: "2026-03-04".into(),
            currency: "EUR".into(),
            amount: -20.0,
            rate: 390.0,
            rate_source: "MNB".into(),
            rate_date: Some("2026-03-04".into()),
            huf: -7800,
            note: "ajándékkártya".into(),
            source: "kézi".into(),
            kind: "payment".into(),
            fee: 0.0,
        }];
        let path = std::env::temp_dir().join(format!("kriptomerleg-{}.xlsx", std::process::id()));
        write_xlsx(&txs, &years, &path).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert!(bytes.starts_with(b"PK"));
        let _ = std::fs::remove_file(path);
    }
}
