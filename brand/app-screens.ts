// Bemutató háttér a képernyőképekhez: a Tauri parancsokat a brand/app-screens.json adataival válaszolja meg.
import { mockIPC } from "@tauri-apps/api/mocks";
import demo from "./app-screens.json";

type Args = Record<string, unknown>;
type Year = (typeof demo.summary)[number];

const preview = {
  source: "binance",
  format: "Binance API",
  origin: "Binance API · lekérve 2026-11-02 10:42 · 2026-01-01 óta",
  existing: 0,
  ignored: 37,
  warnings: [],
  manualInYears: [],
  rows: [
    {
      key: "a",
      date: "2026-09-15",
      currency: "EUR",
      kind: "buy",
      amount: 600,
      fee: 0.6,
      count: 1,
      duplicate: "Revolut tétel ugyanezen a napon: 600,00 EUR",
      details: [],
    },
    { key: "b", date: "2026-10-28", currency: "EUR", kind: "buy", amount: 250, fee: 0.25, count: 1, details: [] },
    { key: "c", date: "2026-10-30", currency: "EUR", kind: "sell", amount: 820, fee: 0.82, count: 2, details: [] },
    { key: "d", date: "2026-10-31", currency: "EUR", kind: "buy", amount: 400, fee: 0.4, count: 1, details: [] },
  ],
};

function previewTax(tx: { year: number; huf: number; excludeId: number | null }): Year | null {
  const y = demo.summary.find((s) => s.year === tx.year);
  if (!y) return null;
  const resultHuf = y.resultHuf - tx.huf;
  const taxGross = Math.max(0, Math.round(resultHuf * 0.15));
  return { ...y, resultHuf, taxGross, taxDue: taxGross, taxFreeHeadroom: Math.max(0, Math.round(y.offsetCap / 0.15) - resultHuf) };
}

mockIPC((cmd, payload) => {
  const a = (payload ?? {}) as Args;
  switch (cmd) {
    case "summary":
      return demo.summary;
    case "list_transactions":
      return demo.transactions.filter((t) => a.year == null || t.date.startsWith(String(a.year))).reverse();
    case "get_settings":
      return {};
    case "list_profiles":
      return { current: "default", profiles: [{ id: "default", name: "Saját" }, { id: "p2", name: "Család" }] };
    case "get_mnb_rate":
      return { rate: 366.95, rateDate: "2026-10-30" };
    case "preview_tax":
      return previewTax(a.tx as { year: number; huf: number; excludeId: number | null });
    case "find_issues":
      return demo.issues.slice(0, 3);
    case "plan_withdrawal":
      return demo.plan;
    case "transaction_details":
      return demo.details.filter((d) => d.id === a.id);
    case "declaration_impact":
      return [];
    case "preview_api":
    case "preview_import":
      return preview;
    case "backup_list":
      return [
        { path: "a", name: "kriptomerleg-2026-11-01.kmbackup", date: "2026-11-01", size: 48213 },
        { path: "b", name: "kriptomerleg-2026-10-25.kmbackup", date: "2026-10-25", size: 47104 },
      ];
    case "backup_dir":
      return "C:\\Users\\Te\\AppData\\Roaming\\hu.kriptoado.app\\mentesek";
    case "plugin:app|version":
      return "0.3.18";
    case "plugin:notification|is_permission_granted":
      return true;
    default:
      return null;
  }
});

await import("../src/main.tsx");
