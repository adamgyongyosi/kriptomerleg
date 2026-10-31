import { useCallback, useEffect, useState } from "react";
import { useDialogs } from "./Dialogs";
import { getVersion } from "@tauri-apps/api/app";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { Update } from "@tauri-apps/plugin-updater";
import { findUpdate, RELEASES_URL } from "./updates";
import { openPath } from "@tauri-apps/plugin-opener";
import { api, BackupFile, fmtDate, pickOpen, pickSave, Profiles, today } from "./api";
import { IconDownload, IconUpload } from "./Icons";

interface Props {
  /** A kiválasztott adóév (a PDF nyilvántartáshoz). */
  year: number;
  profiles: Profiles | null;
  settings: Record<string, string>;
  /** Adatváltozás (visszaállítás, profilváltás) után mindent újratölt. */
  onChanged: () => Promise<void>;
  onProfiles: (p: Profiles) => void;
  onSetting: (key: string, value: string) => void;
  notify: (text: string, error?: boolean) => void;
  /** Talált frissítés átadása (a fejlécben jelenik meg). */
  onUpdate: (u: Update | null) => void;
}

const kb = (n: number) => `${Math.max(1, Math.round(n / 1024)).toLocaleString("hu-HU")} KB`;

/** Adatok: export, mentés és visszaállítás, profilok, emlékeztetők. */
export function DataPage({ year, profiles, settings, onChanged, onProfiles, onSetting, notify, onUpdate }: Props) {
  const [backups, setBackups] = useState<BackupFile[]>([]);
  const [password, setPassword] = useState("");
  const [restorePassword, setRestorePassword] = useState("");
  const [pendingRestore, setPendingRestore] = useState<string | null>(null);
  const [newProfile, setNewProfile] = useState("");
  const [busy, setBusy] = useState(false);
  const dialogs = useDialogs();
  const [version, setVersion] = useState("");
  useEffect(() => {
    getVersion().then(setVersion).catch(() => setVersion(""));
  }, []);

  const loadBackups = useCallback(() => {
    api.backupList().then(setBackups).catch(() => setBackups([]));
  }, []);
  useEffect(loadBackups, [loadBackups, profiles?.current]);

  async function run(action: () => Promise<void>) {
    setBusy(true);
    try {
      await action();
    } catch (e) {
      notify(String(e), true);
    } finally {
      setBusy(false);
    }
  }

  const exportTo = (kind: "csv" | "xlsx") =>
    run(async () => {
      const path = await pickSave({
        defaultPath: `kriptomerleg-${today()}.${kind}`,
        filters: [{ name: kind === "csv" ? "CSV" : "Excel", extensions: [kind] }],
      });
      if (!path) return;
      await (kind === "csv" ? api.exportCsv(path) : api.exportXlsx(path));
      notify("Exportálva.");
    });

  const exportPdf = () =>
    run(async () => {
      const path = await pickSave({
        defaultPath: `kriptoeszkoz-nyilvantartas-${year}.pdf`,
        filters: [{ name: "PDF", extensions: ["pdf"] }],
      });
      if (!path) return;
      await api.exportPdf(path, year);
      notify("A nyilvántartás elkészült.");
    });

  const backupNow = () =>
    run(async () => {
      const encrypted = password.length > 0;
      const path = await pickSave({
        defaultPath: `kriptomerleg-mentes-${today()}.${encrypted ? "kmbackup" : "sqlite"}`,
        filters: [{ name: encrypted ? "Titkosított mentés" : "Mentés (SQLite)", extensions: [encrypted ? "kmbackup" : "sqlite"] }],
      });
      if (!path) return;
      await api.backupSave(path, encrypted ? password : null);
      setPassword("");
      notify(encrypted ? "Titkosított mentés elkészült." : "Mentés elkészült.");
    });

  async function restore(path: string, pw: string | null) {
    const ok = await dialogs.confirm({
      title: "Visszaállítod a mentést?",
      message:
        "A jelenlegi profil adatai lecserélődnek a mentés tartalmára. Előtte a mostani állapotról automatikusan mentés készül a mentések mappájába.",
      okLabel: "Visszaállítás",
      danger: true,
    });
    if (!ok) return;
    await run(async () => {
      await api.backupRestore(path, pw);
      setPendingRestore(null);
      setRestorePassword("");
      await onChanged();
      loadBackups();
      notify("Visszaállítva.");
    });
  }

  const restoreFromFile = () =>
    run(async () => {
      const path = await pickOpen({ filters: [{ name: "Kriptomérleg mentés", extensions: ["sqlite", "kmbackup"] }] });
      if (!path) return;
      if (await api.backupIsEncrypted(path)) setPendingRestore(path);
      else await restore(path, null);
    });

  const profileAction = (action: () => Promise<Profiles>, msg: string) =>
    run(async () => {
      onProfiles(await action());
      await onChanged();
      loadBackups();
      notify(msg);
    });

  const remindersOn = settings.reminders !== "off";

  return (
    <div className="page data-page">
      <header className="page-head">
        <h1>Adatok és mentés</h1>
      </header>

      <section className="panel">
        <h2>Export</h2>
        <p className="muted">
          A PDF a törvény szerinti éves nyilvántartás (összesítő, veszteség-nyilvántartás, tételek MNB árfolyammal); az
          elévülés miatt 5 évig őrizd meg. Az Excel és a CSV minden év összes tételét tartalmazza.
        </p>
        <div className="button-row">
          <button className="btn primary" onClick={exportPdf} disabled={busy}>
            <IconDownload /> {year}. évi nyilvántartás (PDF)
          </button>
          <button className="btn" onClick={() => exportTo("xlsx")} disabled={busy}>
            <IconDownload /> Excel (.xlsx)
          </button>
          <button className="btn" onClick={() => exportTo("csv")} disabled={busy}>
            <IconDownload /> CSV
          </button>
        </div>
      </section>

      <section className="panel">
        <h2>Mentés és visszaállítás</h2>
        <p className="muted">
          Az app hetente magától is ment (az utolsó 8 megmarad). Kézi mentést érdemes külső helyre (pendrive, felhő) tenni.
          Jelszóval a mentés titkosított; a jelszót nem tároljuk, elfelejtve a mentés nem nyitható meg.
        </p>
        <div className="backup-row">
          <label className="field">
            <span className="label">
              Jelszó <span className="muted">(nem kötelező)</span>
            </span>
            <input
              id="backup-password"
              type="password"
              autoComplete="new-password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              placeholder="titkosítás nélkül"
            />
          </label>
          <button className="btn primary" onClick={backupNow} disabled={busy}>
            <IconDownload /> Mentés fájlba…
          </button>
          <button className="btn" onClick={restoreFromFile} disabled={busy}>
            <IconUpload /> Visszaállítás fájlból…
          </button>
        </div>

        {pendingRestore && (
          <div className="notice warn restore-password">
            <span>A kiválasztott mentés titkosított. Add meg a jelszavát:</span>
            <input
              id="restore-password"
              type="password"
              autoFocus
              value={restorePassword}
              onChange={(e) => setRestorePassword(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && restorePassword && restore(pendingRestore, restorePassword)}
            />
            <button className="btn primary" disabled={!restorePassword || busy} onClick={() => restore(pendingRestore, restorePassword)}>
              Visszaállítás
            </button>
            <button className="btn ghost" onClick={() => setPendingRestore(null)}>
              Mégse
            </button>
          </div>
        )}

        <h3 className="sub">Automatikus mentések</h3>
        {backups.length === 0 ? (
          <p className="empty">Még nincs automatikus mentés. Az első akkor készül, amikor már van rögzített tétel.</p>
        ) : (
          <ul className="backup-list">
            {backups.map((b) => (
              <li key={b.path}>
                <span>
                  <strong>{fmtDate(b.date)}</strong> <span className="muted">· {kb(b.size)}</span>
                </span>
                <button className="link" onClick={() => restore(b.path, null)} disabled={busy}>
                  Visszaállítás
                </button>
              </li>
            ))}
          </ul>
        )}
        <button className="link" onClick={() => run(async () => void (await openPath(await api.backupDir())))}>
          Mentések mappájának megnyitása ↗
        </button>
      </section>

      <section className="panel">
        <h2>Profilok</h2>
        <p className="muted">
          Külön nyilvántartás például házastársnak vagy családtagnak. Minden profilnak saját adatbázisa és mentései vannak.
        </p>
        <ul className="profile-list">
          {profiles?.profiles.map((p) => (
            <li key={p.id} className={p.id === profiles.current ? "active" : ""}>
              <strong>{p.name}</strong>
              {p.id === profiles.current ? (
                <span className="chip in">aktív</span>
              ) : (
                <button className="link" disabled={busy} onClick={() => profileAction(() => api.switchProfile(p.id), `Profil: ${p.name}`)}>
                  Váltás
                </button>
              )}
              <span className="spacer" />
              <button
                className="link"
                disabled={busy}
                onClick={async () => {
                  const name = await dialogs.prompt({ title: "Profil átnevezése", defaultValue: p.name, okLabel: "Mentés" });
                  if (name && name.trim() !== p.name) await profileAction(() => api.renameProfile(p.id, name), "Átnevezve.");
                }}
              >
                Átnevezés
              </button>
              {p.id !== "sajat" && p.id !== profiles.current && (
                <button
                  className="link danger"
                  disabled={busy}
                  onClick={async () => {
                    const ok = await dialogs.confirm({
                      title: "Profil törlése?",
                      message: `A(z) „${p.name}” profil törlődik. Az adatbázisa a mentések mappájába kerül, onnan visszaállítható.`,
                      okLabel: "Törlés",
                      danger: true,
                    });
                    if (ok) await profileAction(() => api.deleteProfile(p.id), "Profil törölve.");
                  }}
                >
                  Törlés
                </button>
              )}
            </li>
          ))}
        </ul>
        <form
          className="backup-row"
          onSubmit={(e) => {
            e.preventDefault();
            if (!newProfile.trim()) return;
            profileAction(() => api.createProfile(newProfile), `Új profil: ${newProfile.trim()}`).then(() => setNewProfile(""));
          }}
        >
          <label className="field">
            <span className="label">Új profil neve</span>
            <input id="new-profile" value={newProfile} onChange={(e) => setNewProfile(e.target.value)} placeholder="pl. Anna" />
          </label>
          <button className="btn" type="submit" disabled={busy || !newProfile.trim()}>
            Profil létrehozása
          </button>
        </form>
      </section>

      <section className="panel">
        <h2>Frissítések</h2>
        <p className="muted">
          Telepített verzió: <strong>{version || "–"}</strong>. Az app induláskor megnézi, van-e újabb kiadás; ha van, a
          fejlécben felajánlja a letöltést és az újraindítást. Windowson és AppImage-nél automatikus, .deb csomagnál és NixOS-en
          a csomagkezelővel frissíts.
        </p>
        <label className="toggle-row">
          <input
            id="auto-update"
            type="checkbox"
            className="declare"
            checked={settings.frissites !== "off"}
            onChange={(e) => onSetting("frissites", e.target.checked ? "on" : "off")}
          />
          <span>Frissítés keresése induláskor</span>
        </label>
        <div className="button-row">
          <button
            className="btn"
            disabled={busy}
            onClick={() =>
              run(async () => {
                const u = await findUpdate(false);
                if (u) onUpdate(u);
                else notify("A legfrissebb verziót használod.");
              })
            }
          >
            Frissítés keresése most
          </button>
          <button className="link" onClick={() => openUrl(RELEASES_URL)}>
            Kiadások oldala ↗
          </button>
        </div>
      </section>

      <section className="panel">
        <h2>Emlékeztetők</h2>
        <label className="toggle-row">
          <input
            id="reminders"
            type="checkbox"
            className="declare"
            checked={remindersOn}
            onChange={(e) => onSetting("reminders", e.target.checked ? "on" : "off")}
          />
          <span>
            Rendszerértesítés március 15. (az eSZJA tervezet megjelenése) után és május 10. körül, ha az előző évre kriptoadó,
            adókiegyenlítés vagy bevallandó veszteség van. A bevallás és a befizetés határideje május 20.
          </span>
        </label>
      </section>
    </div>
  );
}
