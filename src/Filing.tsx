import { ReactNode, useEffect, useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { api, DeclarationImpact, emptyYear, fmtFt, YearSummary } from "./api";
import { IconAlert, IconCheck } from "./Icons";

const GUIDE_URL =
  "https://nav.gov.hu/pfile/file?path=%2Fnyomtatvanyok%2Fletoltesek%2Fnyomtatvanykitolto_programok%2Fnyomtatvanykitolto_programok_nav%2F25SZJA%2F25szja-kitoltesi-utmutato";
const NAV_INFO_URL = "https://nav.gov.hu/ado/szja/a-kriptougyletek-jovedelmenek-adozasa";
/** Az a bevallási év, amelynek nyomtatványához a sorszámokat ellenőriztük. */
const VERIFIED_FORM_YEAR = 2025;

/** Bevallási segéd: mit hova kell írni az SZJA bevallásban. */
export function Filing({
  year,
  years,
  onDeclare,
}: {
  year: number;
  years: YearSummary[];
  onDeclare: (year: number, declared: boolean) => void;
}) {
  const [impacts, setImpacts] = useState<DeclarationImpact[]>([]);
  useEffect(() => {
    api.declarationImpact().then(setImpacts).catch(() => setImpacts([]));
  }, [years]);
  const get = (y: number) => years.find((s) => s.year === y) ?? emptyYear(y);
  const c = get(year);
  const form = `${String(year).slice(2)}SZJA`;
  const hasOffset = c.offsetUsed > 0;
  // A 205. sor az útmutató szerint a három év adójának egyszerű összege.
  const formCap = get(year - 2).taxGross + get(year - 1).taxGross + c.taxGross;
  // A korábbi veszteséget 15%-os adóértékben tartjuk nyilván; a 207. sorba forintban kell.
  const priorLossHuf = Math.round(Math.max(0, c.lossPool - c.lossCredit) / 0.15);
  const nothing = c.depositsHuf === 0 && c.withdrawalsHuf === 0 && !hasOffset && c.carriedInHuf === 0;
  // Korábbi veszteség, amelyet nem jelöltek bevallottnak, így nem számít bele.
  const undeclared = years.filter((y) => y.year < year && y.lossCredit > 0 && !y.lossDeclared);
  const showOffset = hasOffset || c.lossPool > 0 || (c.resultHuf > 0 && undeclared.length > 0);

  return (
    <div className="page filing">
      <header className="page-head">
        <div>
          <h1>Bevallási segéd · {year}</h1>
          <p className="muted">
            {form} bevallás, beadási határidő: {year + 1}. május 20.
          </p>
        </div>
      </header>

      {year !== VERIFIED_FORM_YEAR && (
        <div className="notice warn">
          <IconAlert />
          <span>
            A sorszámokat a {VERIFIED_FORM_YEAR}-es ({String(VERIFIED_FORM_YEAR).slice(2)}SZJA) nyomtatvány alapján adjuk meg. A{" "}
            {form} nyomtatványon eltérhetnek, ezt a kitöltési útmutatóban ellenőrizd.
          </span>
        </div>
      )}

      {c.preRegime ? (
        <section className="panel">
          <p className="empty">
            2022 előtti év: a kriptoeszközök 15%-os szabálya 2022-től él. A korábban nem adózott ügyletek a NAV szerint a
            2022-es eredménybe számítanak, ezért ezeket a tételeket a 22SZJA bevallás 164. sorában kell figyelembe venni (az
            app így számol).
          </p>
        </section>
      ) : nothing ? (
        <section className="panel">
          <p className="empty">Ebben az évben nincs kriptós tétel, nincs mit bevallani.</p>
        </section>
      ) : (
        <>
          <Step n={1} title="Az év eredménye" where={`${form}-04-es lap, 164. sor`}>
            {c.resultHuf > 0 ? (
              <>
                <Row line="164." col="e" label="Jövedelem (nyereség)" value={c.resultHuf} />
                <Row line="164." col="f" label="A jövedelem adója (15%)" value={c.taxGross} />
              </>
            ) : c.resultHuf < 0 ? (
              <>
                <Row line="164." col="a" label="Veszteség" value={-c.resultHuf} />
                <p className="step-note">
                  <IconAlert /> A veszteséget mindenképp vallod be, mert csak így vonható le később adókiegyenlítésként.
                </p>
              </>
            ) : (
              <p className="muted">Az eredmény 0 Ft, ide nem kell semmit írni.</p>
            )}
            {c.smallExemptHuf > 0 && (
              <p className="muted small">
                A kisértékű ügyletek {fmtFt(c.smallExemptHuf)} bevétele nélkül számolva (67/C. § (2)).
              </p>
            )}
            {c.carriedInHuf !== 0 && (
              <p className="muted small">
                Benne vannak a 2022 előtti évek tételei is ({fmtFt(Math.abs(c.carriedInHuf))}
                {c.carriedInHuf < 0 ? " befizetés" : " kivétel"}): a NAV szerint a korábban nem adózott ügyletek 2022-be
                számítanak.
              </p>
            )}
            {c.resultHuf !== 0 && (
              <p className="muted small">
                Nem kötelező: a {form}-10-es lapon az ügyletek tételes adatait is megadhatod (szerzés ideje, összege, tranzakció-azonosító).
              </p>
            )}
          </Step>

          {showOffset && (
            <Step n={2} title="Adókiegyenlítés (veszteség levonása)" where={`${form}-06-os lap, 202–209. sor`}>
              <p className="step-intro">
                A veszteség 15%-a levonható, legfeljebb a {year - 2}–{year}. évben bevallott kriptoadó erejéig (keret). A
                202–205. sorba <strong>adó</strong>, a 206–207. sorba <strong>veszteség</strong> (forintban) kerül.
              </p>
              <h3 className="sub">Keret</h3>
              <Row line="202." col="b" label={`${year - 2}. évi kriptojövedelem adója`} value={get(year - 2).taxGross} />
              <Row line="203." col="b" label={`${year - 1}. évi kriptojövedelem adója`} value={get(year - 1).taxGross} />
              <Row line="204." col="b" label={`${year}. évi kriptojövedelem adója`} value={c.taxGross} />
              <Row line="205." col="b" label="Keret összesen (202–204.)" value={formCap} />
              <h3 className="sub">Veszteség</h3>
              <Row line="206." col="a" label={`${year}. évi veszteség`} value={c.resultHuf < 0 ? -c.resultHuf : 0} />
              <Row line="207." col="a" label="Korábbi évek még el nem számolt, bevallott vesztesége" value={priorLossHuf} />
              {undeclared.length > 0 && (
                <p className="step-note warn-text">
                  <IconAlert /> Nem számoltuk bele: {undeclared.map((y) => `${y.year} (${fmtFt(-y.resultHuf)})`).join(", ")}, mert
                  nincs bevallottként jelölve. Ha az adott évi bevallásban szerepelt, jelöld az Évek és adó oldalon; ha
                  kimaradt, előbb önellenőrzéssel pótold.
                </p>
              )}
              <Row line="208." col="c" label="Jogosító összeg: (206. + 207.) × 15%" value={c.lossPool} />
              <h3 className="sub">Eredmény</h3>
              <Row line="209." col="c" label={
                  formCap === c.offsetCap
                    ? "Adókiegyenlítés: a 205. és a 208. közül a kisebb"
                    : "Adókiegyenlítés (csökkentett kerettel, lásd lent)"
                } value={c.offsetUsed} strong />
              <p className="step-note">
                <IconArrow2 /> A 209. sor „c” oszlopát írd át a <strong>{form}-C lap 75. sorába</strong> („adókiegyenlítés
                címén befizetett adó”).
              </p>
              {formCap !== c.offsetCap && (
                <p className="step-note warn-text">
                  <IconAlert /> Az előző két évben már érvényesítettél {fmtFt(formCap - c.offsetCap)} adókiegyenlítést. Az app
                  ennyivel kisebb kerettel számol, hogy ugyanazt az adót ne igényeld vissza kétszer. Az útmutató ezt a kriptónál
                  nem részletezi, ezért ezt a pontot egyeztesd könyvelővel.
                </p>
              )}
            </Step>
          )}

          <Step n={showOffset ? 3 : 2} title="Végeredmény" where="az app számítása szerint">
            <div className={`final ${c.taxDue > 0 ? "bad" : "good"}`}>
              {c.taxDue > 0 ? <IconAlert /> : <IconCheck />}
              <span>
                {c.taxDue > 0
                  ? `Fizetendő kriptoadó: ${fmtFt(c.taxDue)}, ${year + 1}. május 20-ig.`
                  : c.taxDue < 0
                    ? `${fmtFt(-c.taxDue)} adókiegyenlítés illet meg: csökkenti a fizetendő SZJA-t, illetve visszajár.`
                    : "Nincs fizetendő kriptoadó."}
              </span>
            </div>
            {c.lossCreditRemaining > 0 && (
              <p className="muted small">
                Az idei veszteségből {fmtFt(c.lossCreditRemaining)} adóérték marad, ez a következő években felhasználható.
              </p>
            )}
          </Step>
        </>
      )}

      {impacts.length > 0 && <SelfRevision impacts={impacts} onDeclare={onDeclare} />}

      <section className="explainer">
        <h2>Jó tudni</h2>
        <ul>
          <li>Kriptóból kriptóba váltás nem adóköteles, csak a forintra vagy euróra váltás (vagy a vele való vásárlás).</li>
          <li>A 2025-ös bevallástól bármely korábbi év még fel nem használt veszteségét levonhatod, ha azt annak az évnek a bevallásában feltüntetted.</li>
          <li>Ha egy korábbi év veszteségét nem vallottad be, azt önellenőrzéssel pótolhatod (5 éven belül).</li>
          <li>Őrizd meg a bizonylatokat (tőzsdei kivonatok, banki utalások): a kiadást igazolni kell.</li>
        </ul>
        <div className="links">
          <button className="link" onClick={() => openUrl(GUIDE_URL)}>
            NAV 25SZJA kitöltési útmutató ↗
          </button>
          <button className="link" onClick={() => openUrl(NAV_INFO_URL)}>
            NAV: a kriptoügyletek adózása ↗
          </button>
        </div>
        <p className="muted small">Tájékoztató segédlet, nem adótanácsadás. Bizonytalan esetben kérdezz könyvelőt.</p>
      </section>
    </div>
  );
}

/** Önellenőrzés-segéd: mely korábbi évek vesztesége maradt ki a bevallásból, és mit érne a pótlása. */
function SelfRevision(props: { impacts: DeclarationImpact[]; onDeclare: (year: number, declared: boolean) => void }) {
  const forms = props.impacts.map((i) => `${String(i.year).slice(2)}SZJA`).join(", ");
  return (
    <section className="panel step self-revision">
      <div className="step-head">
        <span className="step-n warn">!</span>
        <div>
          <h2>Kimaradt veszteség pótlása</h2>
          <span className="muted small">önellenőrzés-segéd</span>
        </div>
      </div>
      <div className="step-body">
        <p className="step-intro">
          Ezeknek az éveknek a vesztesége nincs bevallottként jelölve, ezért az app nem vonja le. Ha a bevallásból kimaradt,
          az adott évi nyomtatványon pótolható, és utána a későbbi évek adójából levonható.
        </p>
        {props.impacts.map((i) => {
          const form = `${String(i.year).slice(2)}SZJA`;
          return (
            <div key={i.year} className="revision-year">
              <div className="revision-head">
                <strong>{i.year}</strong>
                <span className="muted">
                  {fmtFt(i.lossHuf)} veszteség · {form} · pótolható {i.year + 6}. december 31-ig
                </span>
              </div>
              <Row line="164." col="a" label={`${form}-04-es lap: ${i.year}. évi veszteség`} value={i.lossHuf} />
              {i.saving > 0 ? (
                <p className="step-note good-text">
                  <IconCheck /> Pótlás után a későbbi évek adója összesen {fmtFt(i.saving)}-tal kevesebb (vagy ennyivel több jár
                  vissza).
                </p>
              ) : (
                <p className="muted small">
                  A mostani adatok alapján a pótlás most nem csökkenti az adót, de a veszteség később, nyereséges évben levonható.
                </p>
              )}
              <button className="btn" onClick={() => props.onDeclare(i.year, true)}>
                <IconCheck /> Bevallottam / pótoltam
              </button>
            </div>
          );
        })}
        <ul className="revision-steps">
          <li>
            Az adott év nyomtatványát kell használni ({forms}), ÁNYK-ban vagy a NAV webes kitöltőjében, az eredeti bevallás
            összes adatával.
          </li>
          <li>
            A főlapon a bevallás jellege <strong>„O” (önellenőrzés)</strong> vagy <strong>„H” (helyesbítés)</strong>. Mivel a
            veszteség a fizetendő adót nem változtatja, a kitöltési útmutató szerint inkább helyesbítés; ezt a NAV-nál (1819)
            vagy könyvelőnél kérdezd meg. Adókülönbözet nélkül önellenőrzési pótlék nem keletkezik.
          </li>
          <li>Határidő: az elévülésig, vagyis a bevallási év végétől számított 5 évig.</li>
        </ul>
      </div>
    </section>
  );
}

function Step(props: { n: number; title: string; where: string; children: ReactNode }) {
  return (
    <section className="panel step">
      <div className="step-head">
        <span className="step-n">{props.n}</span>
        <div>
          <h2>{props.title}</h2>
          <span className="muted small">{props.where}</span>
        </div>
      </div>
      <div className="step-body">{props.children}</div>
    </section>
  );
}

function Row(props: { line?: string; col?: string; label: string; value: number; strong?: boolean }) {
  const [copied, setCopied] = useState(false);
  const raw = String(Math.round(props.value));
  return (
    <div className={`fill-row ${props.strong ? "strong" : ""}`}>
      {(props.line || props.col) && (
        <span className="col-badge">
          {props.line ? `${props.line} sor ` : ""}
          {props.col ? `„${props.col}”` : ""}
        </span>
      )}
      <span className="fill-label">{props.label}</span>
      <span className="fill-value">{fmtFt(props.value)}</span>
      <button
        className="copy"
        title="Szám másolása (formázás nélkül)"
        onClick={() => {
          navigator.clipboard.writeText(raw).then(() => {
            setCopied(true);
            window.setTimeout(() => setCopied(false), 1200);
          });
        }}
      >
        {copied ? "Másolva" : "Másol"}
      </button>
    </div>
  );
}

const IconArrow2 = () => (
  <svg width={16} height={16} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={1.8} strokeLinecap="round" strokeLinejoin="round">
    <path d="M5 12h14M13 6l6 6-6 6" />
  </svg>
);
