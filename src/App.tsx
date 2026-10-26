import { ReactNode, useCallback, useEffect, useMemo, useState } from "react";
import { ask, open, save } from "@tauri-apps/plugin-dialog";
import { api, inDesktopApp, emptyYear, kindLabel, SOURCE_LABEL, fmtAmount, fmtDate, fmtFt, fmtRate, fmtTax, MONTHS, today, Tx, YearSummary } from "./api";
import { Filing } from "./Filing";
import { ImportDialog } from "./ImportDialog";
import { GroupedColumns } from "./Chart";
import { EntryForm } from "./EntryForm";
import { IconAlert, IconChart, IconCheck, IconDoc, IconDownload, IconHome, IconList, IconLogo, IconTrash, IconUpload } from "./Icons";
import "./App.css";

const CURRENT_YEAR = new Date().getFullYear();
type Page = "overview" | "transactions" | "years" | "filing";

function App() {
  const [page, setPage] = useState<Page>("overview");
  const [years, setYears] = useState<YearSummary[]>([]);
  const [year, setYear] = useState(CURRENT_YEAR);
  const [txs, setTxs] = useState<Tx[]>([]);
  const [editing, setEditing] = useState<Tx | null>(null);
  const [toast, setToast] = useState<{ text: string; error?: boolean } | null>(null);
  const [loaded, setLoaded] = useState(false);
  const [importOpen, setImportOpen] = useState(false);

  const notify = useCallback((text: string, error = false) => {
    setToast({ text, error });
    window.setTimeout(() => setToast(null), error ? 6000 : 3000);
  }, []);

  const reload = useCallback(async () => {
    const [s, t] = await Promise.all([api.summary(), api.listTransactions(year)]);
    setYears(s);
    setTxs(t);
    setLoaded(true);
  }, [year]);

  useEffect(() => {
    reload().catch((e) => notify(String(e), true));
  }, [reload, notify]);

  const current = useMemo(() => years.find((y) => y.year === year) ?? emptyYear(year), [years, year]);
  const yearOptions = useMemo(() => {
    const set = new Set(years.map((y) => y.year));
    set.add(CURRENT_YEAR);
    set.add(year);
    return [...set].sort((a, b) => b - a);
  }, [years, year]);

  async function onSaved(t: Tx) {
    notify(`${editing ? "Módosítva" : "Rögzítve"}: ${fmtAmount(t.amount, t.currency)} = ${fmtFt(t.huf)}`);
    setEditing(null);
    const y = Number(t.date.slice(0, 4));
    if (y !== year) setYear(y);
    else await reload();
  }

  async function remove(t: Tx) {
    const ok = await ask(`${fmtDate(t.date)}  ·  ${fmtAmount(t.amount, t.currency)}  (${fmtFt(t.huf)})`, {
      title: "Törlöd ezt a tételt?",
      kind: "warning",
      okLabel: "Törlés",
      cancelLabel: "Mégse",
    });
    if (!ok) return;
    try {
      await api.deleteTransaction(t.id);
      setEditing(null);
      await reload();
      notify("Tétel törölve.");
    } catch (err) {
      notify(String(err), true);
    }
  }

  async function importExcel() {
    setImportOpen(false);
    const path = await open({ multiple: false, filters: [{ name: "Excel", extensions: ["xlsx", "xlsm", "xls"] }] });
    if (!path) return;
    try {
      const r = await api.importExcel(path);
      notify(`Import kész: ${r.imported} új tétel${r.skipped ? `, ${r.skipped} már megvolt` : ""}.`);
      await reload();
    } catch (err) {
      notify(String(err), true);
    }
  }

  async function exportCsv() {
    const path = await save({ defaultPath: `kripto-ado-${today()}.csv`, filters: [{ name: "CSV", extensions: ["csv"] }] });
    if (!path) return;
    try {
      await api.exportCsv(path);
      notify("Exportálva.");
    } catch (err) {
      notify(String(err), true);
    }
  }

  const nav: { id: Page; label: string; icon: ReactNode }[] = [
    { id: "overview", label: "Áttekintés", icon: <IconHome /> },
    { id: "transactions", label: "Tételek", icon: <IconList /> },
    { id: "years", label: "Évek és adó", icon: <IconChart /> },
    { id: "filing", label: "Bevallási segéd", icon: <IconDoc /> },
  ];

  return (
    <div className="shell">
      <aside className="sidebar">
        <div className="brand">
          <IconLogo />
          <div>
            <strong>Kriptomérleg</strong>
            <span>kriptóadó, helyben</span>
          </div>
        </div>

        <nav>
          {nav.map((n) => (
            <button key={n.id} className={page === n.id ? "active" : ""} onClick={() => setPage(n.id)}>
              {n.icon}
              {n.label}
            </button>
          ))}
        </nav>

        <div className="side-section">
          <span className="side-label">Adóév</span>
          <div className="year-list">
            {yearOptions.map((y) => (
              <button key={y} className={y === year ? "active" : ""} onClick={() => setYear(y)}>
                {y}
              </button>
            ))}
          </div>
        </div>

        <div className="side-bottom">
          <button onClick={() => setImportOpen(true)}>
            <IconUpload /> Importálás
          </button>
          <button onClick={exportCsv}>
            <IconDownload /> Export CSV
          </button>
        </div>
      </aside>

      <main className="content">
        {!inDesktopApp() && (
          <div className="notice warn browser-warning">
            <IconAlert />
            <span>
              Ezt az oldalt böngészőben nyitottad meg. Az adatok mentése és az MNB árfolyam csak az asztali appban működik –
              indítsd a <strong>Kriptomérleg</strong> programot a Start menüből vagy a <code>kriptomerleg.exe</code> fájlból.
            </span>
          </div>
        )}
        {page === "overview" && (
          <Overview
            year={year}
            current={current}
            years={years}
            txs={txs}
            empty={loaded && years.length === 0}
            onImport={() => setImportOpen(true)}
            onSaved={onSaved}
            onError={(m) => notify(m, true)}
            onEdit={setEditing}
            onShowAll={() => setPage("transactions")}
          />
        )}
        {page === "transactions" && (
          <Transactions
            year={year}
            txs={txs}
            onEdit={setEditing}
            onDelete={remove}
            onDeleteSource={async (source) => {
              const label = SOURCE_LABEL[source] ?? source;
              const ok = await ask(`Az összes ${label} forrásból importált ${year}. évi tétel törlődik.`, {
                title: `${label} tételek törlése?`,
                kind: "warning",
                okLabel: "Törlés",
                cancelLabel: "Mégse",
              });
              if (!ok) return;
              try {
                const n = await api.deleteBySource(source, year);
                notify(`${n} tétel törölve.`);
                await reload();
              } catch (err) {
                notify(String(err), true);
              }
            }}
          />
        )}
        {page === "years" && <Years years={years} selected={year} onSelect={setYear} />}
        {page === "filing" && <Filing year={year} years={years} />}
      </main>

      {editing && (
        <div className="modal-backdrop" onMouseDown={(e) => e.target === e.currentTarget && setEditing(null)}>
          <div className="modal" role="dialog" aria-label="Tétel módosítása">
            <h2>Tétel módosítása</h2>
            <EntryForm
              key={editing.id}
              editing={editing}
              years={years}
              onSaved={onSaved}
              onError={(m) => notify(m, true)}
              onCancel={() => setEditing(null)}
              onDelete={() => remove(editing)}
            />
          </div>
        </div>
      )}

      {importOpen && (
        <ImportDialog
          onClose={() => setImportOpen(false)}
          onExcel={importExcel}
          onImported={async (msg) => {
            setImportOpen(false);
            notify(msg);
            await reload();
          }}
        />
      )}

      {toast && <div className={`toast ${toast.error ? "error" : ""}`}>{toast.text}</div>}
    </div>
  );
}

/* ───────────── Áttekintés ───────────── */

function Overview(props: {
  year: number;
  current: YearSummary;
  years: YearSummary[];
  txs: Tx[];
  empty: boolean;
  onImport: () => void;
  onSaved: (t: Tx) => void;
  onError: (m: string) => void;
  onEdit: (t: Tx) => void;
  onShowAll: () => void;
}) {
  const { year, current: c, txs } = props;

  const monthly = useMemo(() => {
    const deposits = Array(12).fill(0);
    const withdrawals = Array(12).fill(0);
    for (const t of txs) {
      const m = Number(t.date.slice(5, 7)) - 1;
      if (t.huf > 0) deposits[m] += t.huf;
      else withdrawals[m] -= t.huf;
    }
    return { deposits, withdrawals };
  }, [txs]);

  let status: { tone: "good" | "bad" | "neutral"; text: string };
  if (c.taxDue > 0) {
    status = {
      tone: "bad",
      text: `${fmtFt(c.resultHuf)} nyereség után${c.offsetUsed ? `, ${fmtFt(c.offsetUsed)} levonással` : ""}, ${year + 1}. május 20-ig.`,
    };
  } else if (c.taxDue < 0) {
    status = { tone: "good", text: "A veszteség miatt a korábbi évek adójából visszajár (adókiegyenlítés)." };
  } else if (c.resultHuf > 0 && c.offsetUsed > 0) {
    status = { tone: "good", text: "A nyereséget a korábbi évek vesztesége fedezi." };
  } else if (c.resultHuf < 0) {
    status = { tone: "good", text: `Idén veszteség – 15%-a (${fmtFt(c.lossCredit)}) későbbi nyereségből levonható.` };
  } else {
    status = { tone: "neutral", text: "Idén még nincs adóköteles nyereség." };
  }

  return (
    <div className="page overview">
      <div className="overview-main">
        <header className="page-head">
          <h1>{year}. év</h1>
        </header>

        {props.empty && (
          <div className="welcome">
            <div>
              <h2>Üdv! Kezdjük az adatokkal.</h2>
              <p className="muted">
                Rögzítsd jobb oldalt az első befizetést, vagy importáld a tőzsdei exportodat (Binance, Revolut,
                Kraken, Bybit), illetve a régi Excel táblázatodat.
              </p>
            </div>
            <button className="btn primary" onClick={props.onImport}>
              <IconUpload /> Importálás
            </button>
          </div>
        )}

        <section className={`hero ${status.tone}`}>
          <div className="hero-main">
            <span className="hero-label">{c.taxDue < 0 ? "Visszajáró adó" : "Fizetendő adó"} · {year}</span>
            <strong className="hero-value">{fmtFt(Math.abs(c.taxDue))}</strong>
            <span className="hero-status">
              {status.tone === "bad" ? <IconAlert /> : status.tone === "good" ? <IconCheck /> : null}
              {status.text}
            </span>
          </div>
          <div className="hero-side">
            <span className="hero-label">Adómentesen még kivehető</span>
            <strong className="hero-side-value">{fmtFt(c.taxFreeHeadroom)}</strong>
            <span className="muted small">ennyi kivételig nem keletkezik adó {year}-ben</span>
          </div>
        </section>

        <section className="stats">
          <Stat label="Befizetve" value={fmtFt(c.depositsHuf)} swatch="--series-1" />
          <Stat label="Kivéve" value={fmtFt(c.withdrawalsHuf)} swatch="--series-2" />
          <Stat
            label={c.resultHuf >= 0 ? "Nyereség" : "Veszteség"}
            value={fmtFt(Math.abs(c.resultHuf))}
            hint="kivéve − befizetve"
          />
          <Stat
            label="Fel nem használt veszteség"
            value={fmtFt(Math.max(0, c.lossPool - c.offsetUsed))}
            hint="adóértékben (15%), később levonható"
          />
        </section>

        <section className="panel">
          <div className="panel-head">
            <h2>Havonta</h2>
          </div>
          <GroupedColumns
            categories={MONTHS}
            series={[
              { label: "Befizetés", color: "--series-1", values: monthly.deposits },
              { label: "Kivétel", color: "--series-2", values: monthly.withdrawals },
            ]}
            height={200}
          />
        </section>

        <section className="panel">
          <div className="panel-head">
            <h2>Legutóbbi tételek</h2>
            {txs.length > 0 && (
              <button className="link" onClick={props.onShowAll}>
                Mind a {txs.length} tétel
              </button>
            )}
          </div>
          {txs.length === 0 ? (
            <p className="empty">Ebben az évben még nincs tétel.</p>
          ) : (
            <ul className="recent">
              {txs.slice(0, 6).map((t) => (
                <li key={t.id} onClick={() => props.onEdit(t)} title="Kattints a módosításhoz">
                  <span className={`dot ${t.amount > 0 ? "in" : "out"}`} />
                  <span className="recent-main">
                    <strong>{kindLabel(t)}</strong>
                    <span className="muted">
                      {fmtDate(t.date)}
                      {t.note ? ` · ${t.note}` : ""}
                    </span>
                  </span>
                  <span className="recent-amount">
                    <strong>{fmtFt(Math.abs(t.huf))}</strong>
                    {t.currency !== "HUF" && <span className="muted">{fmtAmount(Math.abs(t.amount), t.currency)}</span>}
                  </span>
                </li>
              ))}
            </ul>
          )}
        </section>
      </div>

      <aside className="overview-side">
        <section className="panel entry-panel">
          <h2>Új tétel</h2>
          <EntryForm years={props.years} onSaved={props.onSaved} onError={props.onError} />
        </section>
      </aside>
    </div>
  );
}

function Stat(props: { label: string; value: string; hint?: string; swatch?: string }) {
  return (
    <div className="stat">
      <span className="stat-label">
        {props.swatch && <i style={{ background: `var(${props.swatch})` }} />}
        {props.label}
      </span>
      <strong>{props.value}</strong>
      {props.hint && <span className="muted small">{props.hint}</span>}
    </div>
  );
}

/* ───────────── Tételek ───────────── */

function Transactions(props: {
  year: number;
  txs: Tx[];
  onEdit: (t: Tx) => void;
  onDelete: (t: Tx) => void;
  onDeleteSource: (source: string) => void;
}) {
  const [filter, setFilter] = useState<"all" | "in" | "out">("all");
  const [source, setSource] = useState("all");
  const [q, setQ] = useState("");
  const sources = [...new Set(props.txs.map((t) => t.source))];
  const rows = props.txs.filter(
    (t) =>
      (filter === "all" || (filter === "in" ? t.amount > 0 : t.amount < 0)) &&
      (source === "all" || t.source === source) &&
      (!q || t.note.toLowerCase().includes(q.toLowerCase()) || t.date.includes(q)),
  );
  const sumIn = rows.filter((t) => t.huf > 0).reduce((s, t) => s + t.huf, 0);
  const sumOut = rows.filter((t) => t.huf < 0).reduce((s, t) => s - t.huf, 0);

  return (
    <div className="page">
      <header className="page-head">
        <h1>Tételek · {props.year}</h1>
        <div className="toolbar">
          <div className="segmented">
            {(
              [
                ["all", "Mind"],
                ["in", "Kiadás"],
                ["out", "Bevétel"],
              ] as const
            ).map(([id, label]) => (
              <button key={id} className={filter === id ? "on" : ""} onClick={() => setFilter(id)}>
                {label}
              </button>
            ))}
          </div>
          {sources.length > 1 && (
            <select value={source} onChange={(e) => setSource(e.target.value)} aria-label="Forrás">
              <option value="all">Minden forrás</option>
              {sources.map((s) => (
                <option key={s} value={s}>
                  {SOURCE_LABEL[s] ?? s}
                </option>
              ))}
            </select>
          )}
          {source !== "all" && source !== "kézi" && (
            <button className="btn danger-ghost" onClick={() => props.onDeleteSource(source)}>
              <IconTrash /> {SOURCE_LABEL[source] ?? source} tételek törlése
            </button>
          )}
          <input className="search" placeholder="Keresés…" value={q} onChange={(e) => setQ(e.target.value)} />
        </div>
      </header>

      <section className="panel flush">
        {rows.length === 0 ? (
          <p className="empty">Nincs megjeleníthető tétel.</p>
        ) : (
          <div className="table-wrap">
            <table>
              <thead>
                <tr>
                  <th>Dátum</th>
                  <th>Típus</th>
                  <th>Forrás</th>
                  <th className="num">Összeg</th>
                  <th className="num">Árfolyam</th>
                  <th className="num">Forint</th>
                  <th>Megjegyzés</th>
                  <th />
                </tr>
              </thead>
              <tbody>
                {rows.map((t) => (
                  <tr key={t.id} className="clickable" onClick={() => props.onEdit(t)}>
                    <td>{fmtDate(t.date)}</td>
                    <td>
                      <span className={`chip ${t.amount > 0 ? "in" : "out"}`}>{kindLabel(t)}</span>
                    </td>
                    <td className="muted">{SOURCE_LABEL[t.source] ?? t.source}</td>
                    <td className="num">{fmtAmount(Math.abs(t.amount), t.currency)}</td>
                    <td
                      className="num muted"
                      title={
                        t.rateSource === "MNB" ? `MNB, közzétéve: ${t.rateDate}` : t.rateSource === "kézi" ? "Kézzel megadott árfolyam" : ""
                      }
                    >
                      {t.currency === "HUF" ? "–" : `${fmtRate(t.rate)}${t.rateSource === "kézi" ? " (kézi)" : ""}`}
                    </td>
                    <td className="num strong">{fmtFt(Math.abs(t.huf))}</td>
                    <td className="note">{t.note}</td>
                    <td className="row-actions">
                      <button
                        className="icon danger"
                        title="Törlés"
                        onClick={(e) => {
                          e.stopPropagation();
                          props.onDelete(t);
                        }}
                      >
                        <IconTrash />
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
              <tfoot>
                <tr>
                  <td colSpan={5} className="muted">
                    {rows.length} tétel
                  </td>
                  <td className="num" colSpan={3}>
                    <span className="muted">kiadás </span>
                    <strong>{fmtFt(sumIn)}</strong>
                    <span className="muted"> · bevétel </span>
                    <strong>{fmtFt(sumOut)}</strong>
                  </td>
                </tr>
              </tfoot>
            </table>
          </div>
        )}
      </section>
    </div>
  );
}

/* ───────────── Évek ───────────── */

function Years(props: { years: YearSummary[]; selected: number; onSelect: (y: number) => void }) {
  const { years } = props;
  return (
    <div className="page">
      <header className="page-head">
        <h1>Évek és adó</h1>
      </header>

      {years.length === 0 ? (
        <section className="panel">
          <p className="empty">Még nincs adat.</p>
        </section>
      ) : (
        <>
          <section className="panel">
            <GroupedColumns
              categories={years.map((y) => String(y.year))}
              series={[
                { label: "Befizetés", color: "--series-1", values: years.map((y) => y.depositsHuf) },
                { label: "Kivétel", color: "--series-2", values: years.map((y) => y.withdrawalsHuf) },
              ]}
              extra={(i) => ({ label: "Fizetendő adó", value: fmtTax(years[i].taxDue) })}
            />
          </section>

          <section className="panel flush">
            <div className="table-wrap">
              <table>
                <thead>
                  <tr>
                    <th>Év</th>
                    <th className="num">Befizetve</th>
                    <th className="num">Kivéve</th>
                    <th className="num">Eredmény</th>
                    <th className="num">Adó 15%</th>
                    <th className="num" title="Korábbi veszteség 15%-ából levonva">Levonva</th>
                    <th className="num">Fizetendő</th>
                    <th className="num" title="Az év veszteségének 15%-a: ebből mennyi maradt még felhasználatlanul">Veszteség (15%)</th>
                  </tr>
                </thead>
                <tbody>
                  {[...years].reverse().map((y) => (
                    <tr key={y.year} className={`clickable ${y.year === props.selected ? "selected" : ""}`} onClick={() => props.onSelect(y.year)}>
                      <td className="strong">{y.year}</td>
                      <td className="num">{fmtFt(y.depositsHuf)}</td>
                      <td className="num">{fmtFt(y.withdrawalsHuf)}</td>
                      <td className="num">
                        {y.resultHuf > 0 ? "+" : y.resultHuf < 0 ? "−" : ""}
                        {fmtFt(Math.abs(y.resultHuf))}
                      </td>
                      <td className="num">{fmtFt(y.taxGross)}</td>
                      <td className="num">{y.offsetUsed ? `−${fmtFt(y.offsetUsed)}` : "–"}</td>
                      <td className="num strong">{fmtTax(y.taxDue)}</td>
                      <td className="num muted">
                        {y.lossCredit ? (
                          <>
                            {fmtFt(y.lossCreditRemaining)} maradt / {fmtFt(y.lossCredit)}
                          </>
                        ) : (
                          "–"
                        )}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </section>
        </>
      )}

      <section className="explainer">
        <h2>Hogyan számol az app?</h2>
        <ul>
          <li>
            <strong>Éves eredmény</strong> = az év összes kivétele − az év összes befizetése, forintban.
          </li>
          <li>
            <strong>Nyereség</strong> után 15% SZJA fizetendő (szocho nincs). A bevallás a következő év májusában esedékes.
          </li>
          <li>
            <strong>Veszteség</strong> esetén annak 15%-a levonható (adókiegyenlítés), legfeljebb az adott és az előző két évben
            bevallott kriptoadó erejéig – így a korábban befizetett adó vissza is járhat. A 2025-ös bevallástól a veszteség
            nem évül el, de csak akkor használható, ha az adott évben bevallottad.
          </li>
          <li>
            <strong>EUR</strong> tételeknél a teljesítés napján érvényes MNB árfolyam számít; hétvégén és ünnepnap az utolsó
            közzétett.
          </li>
        </ul>
        <p className="muted small">Tájékoztató számítás, nem adótanácsadás.</p>
      </section>
    </div>
  );
}

export default App;
