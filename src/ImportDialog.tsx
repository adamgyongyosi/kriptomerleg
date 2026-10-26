import { useMemo, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { api, fmtDate, fmtRate, ImportPreview, SOURCE_LABEL } from "./api";
import { IconAlert, IconUpload } from "./Icons";

interface Props {
  onClose: () => void;
  onImported: (msg: string) => void;
  onExcel: () => void;
}

const HOWTO: { name: string; steps: string }[] = [
  { name: "Binance", steps: "Wallet → Transaction History → Export (Generate all statements), CSV" },
  { name: "Revolut", steps: "Kripto → … → Statement (kivonat), CSV vagy Excel" },
  { name: "Kraken", steps: "History → Export → Ledgers, CSV" },
  { name: "Bybit", steps: "Orders → Spot Order → Trade History → Export; ill. Convert → History → Export" },
];

const amountFmt = (n: number) => fmtRate(n);

export function ImportDialog({ onClose, onImported, onExcel }: Props) {
  const [preview, setPreview] = useState<ImportPreview | null>(null);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function choose() {
    const path = await open({
      multiple: false,
      filters: [{ name: "Tőzsdei export", extensions: ["csv", "xlsx", "xls", "txt"] }],
    });
    if (!path) return;
    setBusy(true);
    setError(null);
    try {
      const p = await api.previewImport(path);
      setPreview(p);
      setSelected(new Set(p.rows.map((r) => r.key)));
    } catch (e) {
      setError(String(e));
      setPreview(null);
    } finally {
      setBusy(false);
    }
  }

  async function commit() {
    if (!preview) return;
    setBusy(true);
    setError(null);
    try {
      const rows = preview.rows.filter((r) => selected.has(r.key));
      const r = await api.commitImport(preview.source, rows);
      onImported(
        `${SOURCE_LABEL[preview.source] ?? preview.source}: ${r.imported} új tétel${r.skipped ? `, ${r.skipped} frissítve` : ""}.`,
      );
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  // Évenkénti, devizánkénti összesítés az előnézethez.
  const summary = useMemo(() => {
    const m = new Map<string, { year: string; currency: string; buy: number; sell: number; n: number }>();
    for (const r of preview?.rows ?? []) {
      if (!selected.has(r.key)) continue;
      const k = `${r.date.slice(0, 4)}-${r.currency}`;
      const s = m.get(k) ?? { year: r.date.slice(0, 4), currency: r.currency, buy: 0, sell: 0, n: 0 };
      if (r.kind === "buy") s.buy += r.amount;
      else s.sell += r.amount;
      s.n += r.count;
      m.set(k, s);
    }
    return [...m.values()].sort((a, b) => b.year.localeCompare(a.year));
  }, [preview, selected]);

  const toggle = (key: string) =>
    setSelected((s) => {
      const n = new Set(s);
      n.has(key) ? n.delete(key) : n.add(key);
      return n;
    });

  return (
    <div className="modal-backdrop" onMouseDown={(e) => e.target === e.currentTarget && !busy && onClose()}>
      <div className="modal wide" role="dialog" aria-label="Importálás">
        <div className="modal-head">
          <h2>Importálás tőzsdéről</h2>
          <button className="icon visible" onClick={onClose} title="Bezárás" disabled={busy}>
            ✕
          </button>
        </div>

        {!preview ? (
          <div className="import-start">
            <p className="muted">
              Az app a <strong>fiat ↔ kripto ügyleteket</strong> veszi át (vétel euróért, eladás euróra, díjakkal együtt),
              naponta összevonva, az aznapi MNB árfolyammal. A tőzsdére utalás és a kripto–kripto csere (pl. USDT) nem
              adóköteles, ezeket kihagyja. Újraimportáláskor nem duplikál, hanem frissít.
            </p>
            <button className="btn primary big" onClick={choose} disabled={busy}>
              <IconUpload /> {busy ? "Beolvasás…" : "Export fájl kiválasztása (CSV / Excel)"}
            </button>
            {error && <p className="notice warn">{error}</p>}
            <div className="howto">
              <span className="side-label">Honnan töltsd le?</span>
              {HOWTO.map((h) => (
                <div key={h.name}>
                  <strong>{h.name}:</strong> <span className="muted">{h.steps}</span>
                </div>
              ))}
              <p className="muted small">A menük neve az appok frissítésével változhat.</p>
            </div>
            <div className="import-alt">
              <span className="muted">Régi, saját Excel táblázat (évenkénti lapok, dátum + forint)?</span>
              <button className="link" onClick={onExcel}>
                Excel import
              </button>
            </div>
          </div>
        ) : (
          <div className="import-preview">
            <div className="import-meta">
              <span className="chip in">{preview.format}</span>
              <span className="muted">
                {preview.rows.length} napi tétel · {preview.ignored} sor kihagyva
                {preview.existing ? ` · ${preview.existing} már importálva (frissül)` : ""}
              </span>
            </div>

            {preview.manualInYears.length > 0 && (
              <div className="notice warn">
                <IconAlert />
                <span>
                  Ezekben az években kézi vagy Excel-tételek is vannak:{" "}
                  {preview.manualInYears.map((y) => `${y.year} (${y.count} db)`).join(", ")}. Ha azok ugyanezeket a
                  vásárlásokat jelentik (pl. a tőzsdére utalt összeg), töröld őket a Tételek oldalon, különben az app duplán
                  számolja a kiadást.
                </span>
              </div>
            )}
            {preview.warnings.map((w) => (
              <div key={w} className="notice warn">
                <IconAlert />
                <span>{w}</span>
              </div>
            ))}

            <table className="summary-table">
              <thead>
                <tr>
                  <th>Év</th>
                  <th>Deviza</th>
                  <th className="num">Vétel (kiadás)</th>
                  <th className="num">Eladás (bevétel)</th>
                  <th className="num">Ügylet</th>
                </tr>
              </thead>
              <tbody>
                {summary.map((s) => (
                  <tr key={`${s.year}-${s.currency}`}>
                    <td className="strong">{s.year}</td>
                    <td>{s.currency}</td>
                    <td className="num">{amountFmt(s.buy)}</td>
                    <td className="num">{amountFmt(s.sell)}</td>
                    <td className="num muted">{s.n}</td>
                  </tr>
                ))}
              </tbody>
            </table>

            <div className="table-wrap import-rows">
              <table>
                <thead>
                  <tr>
                    <th>
                      <input
                        type="checkbox"
                        checked={selected.size === preview.rows.length}
                        onChange={(e) => setSelected(e.target.checked ? new Set(preview.rows.map((r) => r.key)) : new Set())}
                        aria-label="Mind"
                      />
                    </th>
                    <th>Nap</th>
                    <th>Típus</th>
                    <th className="num">Összeg</th>
                    <th className="num">Ebből díj</th>
                    <th className="num">Ügylet</th>
                  </tr>
                </thead>
                <tbody>
                  {[...preview.rows].reverse().map((r) => (
                    <tr key={r.key} className="clickable" onClick={() => toggle(r.key)}>
                      <td>
                        <input type="checkbox" checked={selected.has(r.key)} readOnly />
                      </td>
                      <td>{fmtDate(r.date)}</td>
                      <td>
                        <span className={`chip ${r.kind === "buy" ? "in" : "out"}`}>{r.kind === "buy" ? "Vétel" : "Eladás"}</span>
                      </td>
                      <td className="num strong">
                        {amountFmt(r.amount)} {r.currency}
                      </td>
                      <td className="num muted">{r.fee ? `${amountFmt(r.fee)} ${r.currency}` : "–"}</td>
                      <td className="num muted">{r.count}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>

            {error && <p className="notice warn">{error}</p>}

            <div className="form-actions">
              <button className="btn ghost" onClick={() => setPreview(null)} disabled={busy}>
                Másik fájl
              </button>
              <span className="spacer" />
              <button className="btn primary" onClick={commit} disabled={busy || selected.size === 0}>
                {busy ? "MNB árfolyamok lekérése…" : `${selected.size} tétel importálása`}
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
