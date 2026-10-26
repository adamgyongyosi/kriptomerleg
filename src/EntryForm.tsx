import { useEffect, useRef, useState } from "react";
import { api, Currency, fmtDate, fmtFt, fmtRate, fmtTax, parseAmount, RateInfo, today, Tx, YearSummary } from "./api";
import { IconArrow } from "./Icons";

type Kind = "in" | "out";

interface Props {
  /** Szerkesztett tétel; üres = új tétel. */
  editing?: Tx;
  /** Évenkénti összesítő (a "jelenlegi" adó az előnézethez). */
  years: YearSummary[];
  onSaved: (tx: Tx) => void;
  onError: (msg: string) => void;
  onCancel?: () => void;
  onDelete?: () => void;
}

export function EntryForm({ editing, years, onSaved, onError, onCancel, onDelete }: Props) {
  const [kind, setKind] = useState<Kind>(editing && editing.amount < 0 ? "out" : "in");
  const [amountText, setAmountText] = useState(editing ? String(Math.abs(editing.amount)).replace(".", ",") : "");
  const [currency, setCurrency] = useState<Currency>(editing?.currency ?? "EUR");
  const [date, setDate] = useState(editing?.date ?? today());
  const [manualRate, setManualRate] = useState(
    editing?.rateSource === "kézi" ? String(editing.rate).replace(".", ",") : "",
  );
  const [editRate, setEditRate] = useState(editing?.rateSource === "kézi");
  const [note, setNote] = useState(editing?.note ?? "");
  const [mnb, setMnb] = useState<{ info?: RateInfo; error?: string; loading?: boolean }>({});
  const [preview, setPreview] = useState<YearSummary | null>(null);
  const [busy, setBusy] = useState(false);
  const amountRef = useRef<HTMLInputElement>(null);

  const abs = parseAmount(amountText);
  const amount = abs === null ? null : kind === "out" ? -Math.abs(abs) : Math.abs(abs);
  const manual = manualRate.trim() ? parseAmount(manualRate) : null;
  const rate = currency === "HUF" ? 1 : editRate && manual ? manual : (mnb.info?.rate ?? null);
  const huf = amount !== null && rate !== null ? Math.round(amount * rate) : null;
  const year = Number(date.slice(0, 4));
  const before = years.find((y) => y.year === year);

  // MNB árfolyam a kiválasztott napra.
  useEffect(() => {
    if (currency !== "EUR" || !/^\d{4}-\d{2}-\d{2}$/.test(date)) {
      setMnb({});
      return;
    }
    let cancelled = false;
    setMnb((m) => ({ ...m, loading: true, error: undefined }));
    api
      .getMnbRate(date)
      .then((info) => !cancelled && setMnb({ info }))
      .catch((e) => {
        if (cancelled) return;
        setMnb({ error: String(e) });
        setEditRate(true);
      });
    return () => {
      cancelled = true;
    };
  }, [date, currency]);

  // Adóhatás előnézet.
  useEffect(() => {
    if (huf === null || huf === 0 || !year) {
      setPreview(null);
      return;
    }
    const t = window.setTimeout(() => {
      api
        .previewTax({ year, huf, excludeId: editing?.id ?? null })
        .then(setPreview)
        .catch(() => setPreview(null));
    }, 200);
    return () => window.clearTimeout(t);
  }, [huf, year, editing?.id]);

  function onAmountChange(v: string) {
    // A mínusz előjel kivételt jelent.
    if (/^\s*-/.test(v)) {
      setKind("out");
      v = v.replace("-", "");
    }
    setAmountText(v);
  }

  const valid =
    amount !== null &&
    amount !== 0 &&
    (currency === "HUF" || (editRate ? manual !== null && manual > 0 : !!mnb.info));

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    if (!valid || amount === null || busy) return;
    setBusy(true);
    try {
      const input = {
        date,
        currency,
        amount,
        rate: currency === "EUR" && editRate ? manual : null,
        note,
      };
      const saved = editing ? await api.updateTransaction(editing.id, input) : await api.addTransaction(input);
      onSaved(saved);
      if (!editing) {
        // Új tételnél mindig EUR az alapértelmezett.
        setAmountText("");
        setNote("");
        setCurrency("EUR");
        setEditRate(false);
        setManualRate("");
        setPreview(null);
        amountRef.current?.focus();
      }
    } catch (err) {
      onError(String(err));
    } finally {
      setBusy(false);
    }
  }

  const beforeTax = before?.taxDue ?? 0;
  const delta = preview ? preview.taxDue - beforeTax : 0;

  return (
    <form className="entry" onSubmit={submit}>
      <div className="kind-toggle" role="radiogroup" aria-label="Típus">
        <button type="button" role="radio" aria-checked={kind === "in"} className={kind === "in" ? "on in" : ""} onClick={() => setKind("in")}>
          Befizetés
          <small>kriptóba</small>
        </button>
        <button type="button" role="radio" aria-checked={kind === "out"} className={kind === "out" ? "on out" : ""} onClick={() => setKind("out")}>
          Kivétel
          <small>vissza a bankba</small>
        </button>
      </div>

      <div className="amount-field">
        <span className="label">Összeg</span>
        <div className="amount-box" onClick={() => amountRef.current?.focus()}>
          <input
            ref={amountRef}
            inputMode="decimal"
            autoFocus
            placeholder="0"
            value={amountText}
            onChange={(e) => onAmountChange(e.target.value)}
            aria-label="Összeg"
          />
          <div className="ccy-toggle" role="radiogroup" aria-label="Deviza" onClick={(e) => e.stopPropagation()}>
            {(
              [
                ["EUR", "€", "EUR"],
                ["HUF", "Ft", "Forint"],
              ] as const
            ).map(([code, symbol, title]) => (
              <button
                key={code}
                type="button"
                role="radio"
                aria-checked={currency === code}
                title={title}
                className={currency === code ? "on" : ""}
                onClick={() => {
                  if (currency === code) return;
                  setCurrency(code as Currency);
                  setEditRate(false);
                  setManualRate("");
                  amountRef.current?.focus();
                }}
              >
                {symbol}
              </button>
            ))}
          </div>
        </div>
      </div>

      <div className="meta-row">
        <label className="field">
          <span className="label">Dátum</span>
          <input type="date" value={date} max="2100-12-31" onChange={(e) => setDate(e.target.value)} required />
        </label>
        {currency === "EUR" && (
          <div className="field">
            <span className="label">Árfolyam</span>
            {editRate ? (
              <div className="rate-edit">
                <input
                  inputMode="decimal"
                  placeholder={mnb.info ? fmtRate(mnb.info.rate) : "pl. 392,50"}
                  value={manualRate}
                  onChange={(e) => setManualRate(e.target.value)}
                  autoFocus={!mnb.error}
                />
                {mnb.info && (
                  <button
                    type="button"
                    className="link"
                    onClick={() => {
                      setEditRate(false);
                      setManualRate("");
                    }}
                  >
                    MNB
                  </button>
                )}
              </div>
            ) : (
              <button type="button" className="rate-chip" onClick={() => setEditRate(true)} title="Kattints a kézi árfolyamhoz (pl. banki bizonylat)">
                {mnb.loading ? (
                  <span className="muted">lekérés…</span>
                ) : mnb.info ? (
                  <>
                    <strong>{fmtRate(mnb.info.rate)} Ft</strong>
                    <span className="muted">
                      MNB{mnb.info.rateDate !== date ? ` · ${fmtDate(mnb.info.rateDate)}` : ""}
                    </span>
                  </>
                ) : (
                  <span className="muted">–</span>
                )}
              </button>
            )}
          </div>
        )}
      </div>
      {mnb.error && currency === "EUR" && (
        <p className="hint warn">Az MNB most nem érhető el – írd be az árfolyamot kézzel (pl. a banki bizonylatról).</p>
      )}
      {!mnb.error && mnb.info && mnb.info.rateDate !== date && !editRate && (
        <p className="hint">Ezen a napon nem volt jegyzés (hétvége/ünnep), az utolsó közzétett árfolyam érvényes.</p>
      )}

      <div className="result">
        <div className="result-row">
          <span>Forintban</span>
          <strong className="big">{huf !== null ? fmtFt(Math.abs(huf)) : "–"}</strong>
        </div>
        <div className="result-row">
          <span>Fizetendő adó {year || ""}</span>
          {preview ? (
            <span className="tax-change">
              <span className="muted">{fmtTax(beforeTax)}</span>
              <IconArrow />
              <strong>{fmtTax(preview.taxDue)}</strong>
              {delta !== 0 && <em className={delta > 0 ? "up" : "down"}>{delta > 0 ? "+" : "−"}{fmtFt(Math.abs(delta))}</em>}
            </span>
          ) : (
            <strong>{fmtTax(beforeTax)}</strong>
          )}
        </div>
        {preview && kind === "out" && delta === 0 && (
          <p className="hint good">Ez a kivétel még nem keletkeztet adót.</p>
        )}
      </div>

      <label className="field">
        <span className="label">
          Megjegyzés <span className="muted">(nem kötelező)</span>
        </span>
        <input value={note} placeholder="pl. Revolut → Binance" onChange={(e) => setNote(e.target.value)} />
      </label>

      <div className="form-actions">
        {onDelete && (
          <button type="button" className="btn danger-ghost" onClick={onDelete}>
            Törlés
          </button>
        )}
        <span className="spacer" />
        {onCancel && (
          <button type="button" className="btn ghost" onClick={onCancel}>
            Mégse
          </button>
        )}
        <button className={`btn primary ${kind}`} type="submit" disabled={!valid || busy}>
          {editing ? "Mentés" : kind === "in" ? "Befizetés rögzítése" : "Kivétel rögzítése"}
        </button>
      </div>
    </form>
  );
}
