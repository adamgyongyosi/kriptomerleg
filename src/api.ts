import { invoke as tauriInvoke, InvokeArgs } from "@tauri-apps/api/core";
import { open as openDialog, OpenDialogOptions, save as saveDialog, SaveDialogOptions } from "@tauri-apps/plugin-dialog";

/** Böngészőben (nem az asztali appban) nincs Tauri háttér. */
export const inDesktopApp = () => typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

function invoke<T>(cmd: string, args?: InvokeArgs): Promise<T> {
  if (!inDesktopApp()) {
    return Promise.reject(
      new Error("Ez a felület csak az asztali alkalmazásban működik. Indítsd a Kriptomérleg programot (kriptomerleg.exe)."),
    );
  }
  return tauriInvoke<T>(cmd, args);
}

/** ISO devizakód; a rögzítő űrlapon EUR, USD, GBP, CHF vagy HUF, importból bármely MNB-deviza. */
export type Currency = string;

/** A gyors rögzítés devizái: [kód, jel, név]. */
export const FORM_CURRENCIES = [
  ["EUR", "€", "Euró"],
  ["USD", "$", "Amerikai dollár"],
  ["GBP", "£", "Angol font"],
  ["CHF", "CHF", "Svájci frank"],
  ["HUF", "Ft", "Forint"],
] as const;

export interface Tx {
  id: number;
  date: string;
  currency: Currency;
  /** Pozitív: befizetés, negatív: kivétel. */
  amount: number;
  rate: number;
  rateSource: "MNB" | "kézi" | "-";
  rateDate: string | null;
  huf: number;
  note: string;
  /** "kézi", "excel", "binance", "kraken", "revolut", "bybit" */
  source: string;
  /** "deposit" | "withdrawal" | "payment" | "buy" | "sell" */
  kind: string;
  fee: number;
}

export interface ImportRow {
  key: string;
  date: string;
  currency: string;
  kind: "buy" | "sell";
  amount: number;
  fee: number;
  count: number;
  /** Másik forrásból már van hasonló tétel ugyanazon a napon. */
  duplicate?: string;
}

export interface ImportPreview {
  source: string;
  format: string;
  rows: ImportRow[];
  existing: number;
  ignored: number;
  warnings: string[];
  manualInYears: { year: number; count: number }[];
}

export interface TxInput {
  date: string;
  currency: Currency;
  amount: number;
  /** null: MNB árfolyam a dátum napján. */
  rate: number | null;
  note: string;
  /** "payment": kriptóval fizetett áru/szolgáltatás (bevétel). */
  kind?: "payment";
}

export interface RateInfo {
  rate: number;
  rateDate: string;
}

export interface YearSummary {
  year: number;
  depositsHuf: number;
  withdrawalsHuf: number;
  resultHuf: number;
  /** A nyereség 15%-a. */
  taxGross: number;
  /** Adókiegyenlítési keret (tárgyévi + előző 2 év adója − már érvényesített). */
  offsetCap: number;
  /** Figyelembe vehető, még fel nem használt veszteségek 15%-a. */
  lossPool: number;
  offsetUsed: number;
  /** Negatív: visszajár. */
  taxDue: number;
  lossCredit: number;
  lossCreditRemaining: number;
  taxFreeHeadroom: number;
  /** 2022 előtti év: az eredménye a 2022-es évbe számít. */
  preRegime: boolean;
  /** 2022-ben: a 2022 előtti évek eredménye, amely itt számít. */
  carriedInHuf: number;
  /** A veszteséget bevallották (csak így vonható le később). */
  lossDeclared: boolean;
  /** Kisértékű ügyletként figyelmen kívül hagyott bevétel (bekapcsolt beállításnál). */
  smallExemptHuf: number;
  /** Ennyi bevétel lenne kisértékű ügyletként mentesíthető. */
  smallExemptAvailable: number;
}

/** Egy bevallatlan veszteséges év pótlásának hatása. */
export interface DeclarationImpact {
  year: number;
  lossHuf: number;
  /** Ennyivel kevesebb adó a későbbi években, ha a veszteséget pótlólag bevallják. */
  saving: number;
}

export const api = {
  listTransactions: (year: number | null) => invoke<Tx[]>("list_transactions", { year }),
  addTransaction: (input: TxInput) => invoke<Tx>("add_transaction", { input }),
  updateTransaction: (id: number, input: TxInput) => invoke<Tx>("update_transaction", { id, input }),
  deleteTransaction: (id: number) => invoke<void>("delete_transaction", { id }),
  getMnbRate: (date: string, currency: Currency = "EUR") => invoke<RateInfo>("get_mnb_rate", { date, currency }),
  summary: () => invoke<YearSummary[]>("summary"),
  previewTax: (tx: { year: number; huf: number; excludeId: number | null }) =>
    invoke<YearSummary | null>("preview_tax", { tx }),
  importExcel: (path: string) => invoke<{ imported: number; skipped: number }>("import_excel", { path }),
  exportCsv: (path: string) => invoke<void>("export_csv", { path }),
  previewImport: (path: string) => invoke<ImportPreview>("preview_import", { path }),
  /** Kraken főkönyv csak olvasó API kulccsal; `since`: ÉÉÉÉ-HH-NN. A kulcsot nem tároljuk. */
  previewKrakenApi: (key: string, secret: string, since: string) =>
    invoke<ImportPreview>("preview_kraken_api", { key, secret, since }),
  commitImport: (source: string, rows: ImportRow[]) =>
    invoke<{ imported: number; skipped: number }>("commit_import", { source, rows }),
  deleteBySource: (source: string, year: number | null) => invoke<number>("delete_by_source", { source, year }),
  setLossDeclared: (year: number, declared: boolean) => invoke<void>("set_loss_declared", { year, declared }),
  declarationImpact: () => invoke<DeclarationImpact[]>("declaration_impact"),
  exportXlsx: (path: string) => invoke<void>("export_xlsx", { path }),
  exportPdf: (path: string, year: number) => invoke<void>("export_pdf", { path, year }),
  saveText: (path: string, text: string) => invoke<void>("save_text", { path, text }),
  getSettings: () => invoke<Record<string, string>>("get_settings"),
  setSetting: (key: string, value: string) => invoke<void>("set_setting", { key, value }),
  backupSave: (path: string, password: string | null) => invoke<void>("backup_save", { path, password }),
  backupList: () => invoke<BackupFile[]>("backup_list"),
  backupDir: () => invoke<string>("backup_dir"),
  backupIsEncrypted: (path: string) => invoke<boolean>("backup_is_encrypted", { path }),
  backupRestore: (path: string, password: string | null) => invoke<void>("backup_restore", { path, password }),
  listProfiles: () => invoke<Profiles>("list_profiles"),
  createProfile: (name: string) => invoke<Profiles>("create_profile", { name }),
  switchProfile: (id: string) => invoke<Profiles>("switch_profile", { id }),
  renameProfile: (id: string, name: string) => invoke<Profiles>("rename_profile", { id, name }),
  deleteProfile: (id: string) => invoke<Profiles>("delete_profile", { id }),
};

export interface BackupFile {
  path: string;
  name: string;
  /** A mentés napja, ÉÉÉÉ-HH-NN. */
  date: string;
  size: number;
}

export interface Profiles {
  current: string;
  profiles: { id: string; name: string }[];
}

/**
 * Határidő-figyelmeztetés az előző adóévre március 15. (eSZJA tervezet) és május 20. (bevallás és
 * befizetés) között. `stage`: melyik emlékeztetőnél tartunk (egyszer küldjük el rendszerértesítésként).
 */
export function deadlineReminder(years: YearSummary[], now = new Date()): { year: number; stage: string; text: string } | null {
  const y = now.getFullYear();
  const start = new Date(y, 2, 15);
  const end = new Date(y, 4, 20, 23, 59);
  if (now < start || now > end) return null;
  const prev = years.find((s) => s.year === y - 1);
  if (!prev || prev.preRegime) return null;
  const stage = now >= new Date(y, 4, 10) ? "majus" : "marcius";
  let text: string;
  if (prev.taxDue > 0) text = `${y - 1}. évi kriptoadó: ${fmtFt(prev.taxDue)}. A bevallás és a befizetés határideje május 20.`;
  else if (prev.taxDue < 0)
    text = `${y - 1}: ${fmtFt(-prev.taxDue)} adókiegyenlítés jár. Május 20-ig a bevallásban (06-os lap, C lap 75. sor) érvényesítsd.`;
  else if (prev.resultHuf < 0)
    text = `${y - 1}. évi kriptoveszteség: ${fmtFt(-prev.resultHuf)}. Május 20-ig vallsd be (164. sor „a”), különben később nem vonható le.`;
  else if (prev.resultHuf > 0) text = `${y - 1}: a kriptónyereséget május 20-ig a bevallásban is fel kell tüntetni (164. sor).`;
  else return null;
  return { year: y - 1, stage, text };
}

const huf = new Intl.NumberFormat("hu-HU", { maximumFractionDigits: 0 });
const dec = new Intl.NumberFormat("hu-HU", { minimumFractionDigits: 2, maximumFractionDigits: 2 });

export const fmtFt = (n: number) => `${huf.format(n)} Ft`;
export const SOURCE_LABEL: Record<string, string> = {
  kézi: "Kézi",
  excel: "Excel",
  binance: "Binance",
  kraken: "Kraken",
  revolut: "Revolut",
  bybit: "Bybit",
  bank: "Bank",
};

export function kindLabel(t: { kind: string; amount: number }): string {
  switch (t.kind) {
    case "buy":
      return "Vétel";
    case "sell":
      return "Eladás";
    case "payment":
      return "Fizetés kriptóval";
    default:
      return t.amount > 0 ? "Befizetés" : "Kivétel";
  }
}

/** Fizetendő adó; negatív esetén "… visszajár". */
export const fmtTax = (n: number) => (n < 0 ? `${fmtFt(-n)} visszajár` : fmtFt(n));
export const fmtRate = (n: number) => dec.format(n);
export const fmtAmount = (n: number, currency: Currency) =>
  currency === "HUF" ? fmtFt(n) : `${dec.format(n)} ${currency}`;

/** Gépelés közbeni formázás: ezres tagolás nem törő szóközzel, tizedesvessző. */
export function formatAmountInput(s: string): string {
  let t = s.replace(/[\s ]/g, "");
  // "1.000,50" és "1.000.000": a pont ezres tagolás; egyébként tizedesjel.
  if (t.includes(",") || /^\d{1,3}(\.\d{3})+$/.test(t)) t = t.replace(/\./g, "");
  else t = t.replace(".", ",");
  t = t.replace(/[^\d,]/g, "");
  const [int, ...rest] = t.split(",");
  const grouped = int.replace(/^0+(?=\d)/, "").replace(/\B(?=(\d{3})+(?!\d))/g, " ");
  return rest.length ? `${grouped || "0"},${rest.join("")}` : grouped;
}

/** "1 000,50", "-250", "1.000,5" → szám. */
export function parseAmount(s: string): number | null {
  const t = s.replace(/[\s ]/g, "").replace(/\.(?=\d{3}(\D|$))/g, "").replace(",", ".");
  if (!/^[+-]?\d+(\.\d+)?$/.test(t)) return null;
  return Number(t);
}

export const emptyYear = (year: number): YearSummary => ({
  year,
  depositsHuf: 0,
  withdrawalsHuf: 0,
  resultHuf: 0,
  taxGross: 0,
  offsetCap: 0,
  lossPool: 0,
  offsetUsed: 0,
  taxDue: 0,
  lossCredit: 0,
  lossCreditRemaining: 0,
  taxFreeHeadroom: 0,
  preRegime: year < 2022,
  carriedInHuf: 0,
  lossDeclared: true,
  smallExemptHuf: 0,
  smallExemptAvailable: 0,
});

export const MONTHS = ["jan", "febr", "márc", "ápr", "máj", "jún", "júl", "aug", "szept", "okt", "nov", "dec"];

/** "2026-10-26" → "2026. okt. 26." */
export function fmtDate(iso: string): string {
  const [y, m, d] = iso.split("-").map(Number);
  return `${y}. ${MONTHS[m - 1]}. ${d}.`;
}

export function today(): string {
  const d = new Date();
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

/**
 * Fájlválasztó (megnyitás). Linuxon előbb ellenőrizzük, hogy a GTK fájlválasztó sémája elérhető-e,
 * mert nélküle a program összeomlana (#1); ilyenkor érthető hibaüzenetet kapunk helyette.
 */
export async function pickOpen(options: Omit<OpenDialogOptions, "multiple" | "directory">): Promise<string | null> {
  await invoke<void>("file_dialog_ready");
  const r = await openDialog({ ...options, multiple: false, directory: false });
  return typeof r === "string" ? r : null;
}

/** Fájlválasztó (mentés), ugyanazzal az ellenőrzéssel. */
export async function pickSave(options: SaveDialogOptions): Promise<string | null> {
  await invoke<void>("file_dialog_ready");
  return saveDialog(options);
}
