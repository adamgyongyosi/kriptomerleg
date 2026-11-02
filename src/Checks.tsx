import { useEffect, useState } from "react";
import { api, fmtAmount, fmtDate, fmtFt, formatAmountInput, ImportDetail, inDesktopApp, Issue, parseAmount, Plan, SOURCE_LABEL, Tx } from "./api";
import { IconAlert, IconCheck } from "./Icons";

/* ───────────── Hiányzó vagy gyanús tételek ───────────── */

/** Ellenőrzés: hiányzó vételek, dupla tételek, hézagos vagy elavult importok. `txs` változásakor újrafut. */
export function IssuesPanel(props: { txs: Tx[]; year: number }) {
  const [issues, setIssues] = useState<Issue[] | null>(null);
  const [open, setOpen] = useState(false);
  useEffect(() => {
    if (!inDesktopApp()) return;
    api
      .findIssues()
      .then(setIssues)
      .catch(() => setIssues(null));
  }, [props.txs]);
  if (!issues) return null;
  if (issues.length === 0) {
    return (
      <div className="notice ok issues">
        <IconCheck />
        <span>Ellenőrzés: nem találtam hiányzó vagy dupla tételre utaló jelet.</span>
      </div>
    );
  }
  const warns = issues.filter((i) => i.level === "warn").length;
  // Az aktuális év jelzései előre.
  const sorted = [...issues].sort((a, b) => Number(b.year === props.year) - Number(a.year === props.year));
  return (
    <div className={`notice issues ${warns ? "warn" : ""}`}>
      <IconAlert />
      <div>
        <button className="link" onClick={() => setOpen(!open)} aria-expanded={open}>
          Ellenőrzés: {warns ? `${warns} valószínű hiba` : ""}
          {warns && issues.length > warns ? ", " : ""}
          {issues.length > warns ? `${issues.length - warns} ellenőrizendő` : ""} {open ? "▴" : "▾"}
        </button>
        {open && (
          <ul className="issue-list">
            {sorted.map((i) => (
              <li key={i.text} className={i.level}>
                {i.text}
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}

/* ───────────── Eredeti sorok (nyomkövetés) ───────────── */

/** Egy importált napi tétel mögötti eredeti sorok és a forrásfájl (név, SHA-256) vagy API-lekérés. */
export function DetailsDialog(props: { tx: Tx; onClose: () => void }) {
  const { tx } = props;
  const [details, setDetails] = useState<ImportDetail[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    api.transactionDetails(tx.id).then(setDetails, (e) => setError(String(e)));
  }, [tx.id]);
  const origins = [...new Set((details ?? []).map((d) => d.origin))];
  return (
    <div className="modal-backdrop" onMouseDown={(e) => e.target === e.currentTarget && props.onClose()}>
      <div className="modal wide" role="dialog" aria-label="Eredeti sorok">
        <div className="modal-head">
          <h2>
            {fmtDate(tx.date)} · {SOURCE_LABEL[tx.source] ?? tx.source} · {fmtAmount(Math.abs(tx.amount), tx.currency)}
          </h2>
          <button className="icon visible" onClick={props.onClose} title="Bezárás">
            ✕
          </button>
        </div>
        {error && <p className="notice warn">{error}</p>}
        {details && details.length === 0 && (
          <p className="muted">
            Ehhez a tételhez nincsenek eltárolt eredeti sorok (a nyomkövetés előtt importálták). Importáld újra a fájlt: a tétel
            frissül, és az eredeti sorok is mentődnek.
          </p>
        )}
        {details && details.length > 0 && (
          <>
            <p className="muted small">
              A napi összevont tétel {details.length} eredeti sorból áll. Ezek az Excel-exportban („Eredeti sorok” lap) és az éves
              PDF mellékletében is szerepelnek.
            </p>
            {origins.map((o) => (
              <p key={o} className="small origin">
                <strong>Forrás:</strong> {o}
              </p>
            ))}
            <div className="table-wrap raw-rows">
              <table>
                <tbody>
                  {details.map((d, i) => (
                    <tr key={i}>
                      <td className="muted num">{i + 1}.</td>
                      <td className="mono">{d.raw}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </>
        )}
      </div>
    </div>
  );
}

/* ───────────── „Mi lenne, ha” adótervező ───────────── */

/** Egy tervezett kivétel adóhatása idén vagy jövőre, a kisértékű ügylet keretével. */
export function Planner(props: { year: number; eurRate: number | null; txs: Tx[] }) {
  const [text, setText] = useState("");
  const [cur, setCur] = useState<"EUR" | "HUF">("EUR");
  const [plan, setPlan] = useState<Plan | null>(null);
  const [error, setError] = useState<string | null>(null);
  const value = parseAmount(text);
  const huf = value && value > 0 ? Math.round(cur === "HUF" ? value : props.eurRate ? value * props.eurRate : 0) : 0;

  useEffect(() => {
    if (!huf) {
      setPlan(null);
      return;
    }
    let cancelled = false;
    const t = window.setTimeout(() => {
      api.planWithdrawal(props.year, huf).then(
        (p) => !cancelled && (setPlan(p), setError(null)),
        (e) => !cancelled && setError(String(e)),
      );
    }, 300);
    return () => {
      cancelled = true;
      window.clearTimeout(t);
    };
  }, [huf, props.year, props.txs]);

  const now = plan ? plan.now.taxTotal - plan.baselineTotal : 0;
  const next = plan ? plan.next.taxTotal - plan.baselineTotal : 0;
  let verdict = "";
  if (plan) {
    if (now < next) verdict = `Idén kedvezőbb: ${fmtFt(next - now)}-tal kevesebb adó összesen.`;
    else if (next < now) verdict = `Jövőre kedvezőbb: ${fmtFt(now - next)}-tal kevesebb adó összesen.`;
    else verdict = `Az adó összege ugyanannyi; jövőre kivéve egy évvel később, ${props.year + 2}. május 20-ig kell befizetni.`;
  }
  const s = plan?.small;

  return (
    <section className="panel planner">
      <h2>Adótervező</h2>
      <p className="muted small">Mennyi adóval jár, ha most kiveszel (vagy kriptóval fizetsz) egy összeget – idén vagy jövőre?</p>
      <div className="planner-input">
        <input
          inputMode="decimal"
          placeholder="pl. 2 000"
          value={text}
          aria-label="Tervezett kivétel"
          onChange={(e) => setText(formatAmountInput(e.target.value))}
        />
        <div className="segmented">
          {(["EUR", "HUF"] as const).map((c) => (
            <button key={c} className={cur === c ? "on" : ""} onClick={() => setCur(c)}>
              {c === "EUR" ? "€" : "Ft"}
            </button>
          ))}
        </div>
      </div>
      {cur === "EUR" && value && !props.eurRate && <p className="hint">Az euró átszámításához nincs mai MNB árfolyam – válts forintra.</p>}
      {cur === "EUR" && huf > 0 && <p className="muted small">≈ {fmtFt(huf)} a mai MNB árfolyamon</p>}
      {error && <p className="notice warn">{error}</p>}
      {plan && (
        <div className="planner-result">
          <div className={`plan-row ${now <= next ? "best" : ""}`}>
            <span>Idén ({props.year})</span>
            <strong>{now > 0 ? `+${fmtFt(now)}` : now < 0 ? `−${fmtFt(-now)}` : "nincs adó"}</strong>
          </div>
          <div className={`plan-row ${next < now ? "best" : ""}`}>
            <span>Jövőre ({props.year + 1})</span>
            <strong>{next > 0 ? `+${fmtFt(next)}` : next < 0 ? `−${fmtFt(-next)}` : "nincs adó"}</strong>
          </div>
          <p className="small">{verdict}</p>
          <p className="muted small">
            A jövő évi számítás csak a már rögzített tételekkel számol; ha jövőre is veszel kriptót, az csökkenti az adót.
          </p>
        </div>
      )}
      {s && (
        <div className="planner-small">
          <strong className="small">Kisértékű ügylet {props.year}</strong>
          <p className="muted small">
            Egy ügylet bevétele legfeljebb {fmtFt(s.dayLimit)}, aznap csak ez az egy bevételt hozó ügylet, és évente összesen
            legfeljebb {fmtFt(s.yearCap)}. Idén eddig: {fmtFt(s.yearUsed)}
            {s.todayIncome ? " · ma már volt bevételt hozó ügylet" : ""}.
          </p>
          {plan && (
            <p className={`small ${s.fits ? "good" : ""}`}>
              {s.fits
                ? "Ez az összeg ma, egyetlen ügyletként kisértékű lehet – nem kell bevételként figyelembe venni."
                : "Ez az összeg nem minősülhet kisértékű ügyletnek."}
              {s.fits && !s.enabled && " (A szabály az Évek oldalon kapcsolható be.)"}
            </p>
          )}
        </div>
      )}
    </section>
  );
}
