import { describe, expect, it } from "vitest";
import { deadlineReminder, emptyYear, fmtDate, formatAmountInput, parseAmount, YearSummary } from "./api";

const NBSP = " ";

describe("összeg mező", () => {
  it("ezres tagolást ad gépelés közben", () => {
    expect(formatAmountInput("1000")).toBe(`1${NBSP}000`);
    expect(formatAmountInput("1234567,5")).toBe(`1${NBSP}234${NBSP}567,5`);
    expect(formatAmountInput("0012")).toBe("12");
  });

  it("beillesztett pontos formátumokat is ért", () => {
    expect(formatAmountInput("1.000.000")).toBe(`1${NBSP}000${NBSP}000`);
    expect(formatAmountInput("1.000,50")).toBe(`1${NBSP}000,50`);
    expect(formatAmountInput("12.5")).toBe("12,5");
  });

  it("visszaolvassa a számot", () => {
    expect(parseAmount(`1${NBSP}000,50`)).toBe(1000.5);
    expect(parseAmount("-250")).toBe(-250);
    expect(parseAmount("1.000,5")).toBe(1000.5);
    expect(parseAmount("abc")).toBeNull();
  });
});

describe("dátum", () => {
  it("magyar rövid formátum", () => {
    expect(fmtDate("2026-10-26")).toBe("2026. okt. 26.");
  });
});

describe("határidő-emlékeztető", () => {
  const year = (y: number, patch: Partial<YearSummary>): YearSummary => ({ ...emptyYear(y), ...patch });

  it("csak március 15. és május 20. között szól", () => {
    const years = [year(2025, { resultHuf: 100_000, taxGross: 15_000, taxDue: 15_000 })];
    expect(deadlineReminder(years, new Date(2026, 2, 14))).toBeNull();
    expect(deadlineReminder(years, new Date(2026, 4, 21))).toBeNull();
    const r = deadlineReminder(years, new Date(2026, 3, 1));
    expect(r?.stage).toBe("marcius");
    expect(r?.text).toMatch(/15\s000 Ft/);
    expect(deadlineReminder(years, new Date(2026, 4, 12))?.stage).toBe("majus");
  });

  it("veszteségnél a bevallásra figyelmeztet", () => {
    const r = deadlineReminder([year(2025, { resultHuf: -50_000 })], new Date(2026, 3, 1));
    expect(r?.text).toContain("164. sor");
  });

  it("üres évnél nem szól", () => {
    expect(deadlineReminder([year(2025, {})], new Date(2026, 3, 1))).toBeNull();
  });
});
