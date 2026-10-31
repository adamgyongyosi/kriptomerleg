import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { EntryForm } from "./EntryForm";
import { DialogHost, useDialogs } from "./Dialogs";
import { emptyYear, Tx } from "./api";

/** A Tauri háttér helyett: rögzíti a hívásokat, és egyszerű válaszokat ad. */
let calls: { cmd: string; args: Record<string, unknown> }[] = [];

beforeEach(() => {
  calls = [];
  mockIPC((cmd, args) => {
    calls.push({ cmd, args: (args ?? {}) as Record<string, unknown> });
    if (cmd === "get_mnb_rate") return { rate: 400, rateDate: (args as { date: string }).date };
    if (cmd === "preview_tax") return null;
    if (cmd === "add_transaction") {
      const input = (args as { input: { date: string; currency: string; amount: number; note: string; kind?: string } }).input;
      const tx: Tx = {
        id: 1,
        date: input.date,
        currency: input.currency,
        amount: input.amount,
        rate: 400,
        rateSource: "MNB",
        rateDate: input.date,
        huf: Math.round(input.amount * 400),
        note: input.note,
        source: "kézi",
        kind: input.kind ?? (input.amount > 0 ? "deposit" : "withdrawal"),
        fee: 0,
      };
      return tx;
    }
    return null;
  });
});

afterEach(() => {
  cleanup();
  clearMocks();
});

describe("rögzítő űrlap", () => {
  it("euróból forintot számol az MNB árfolyamon, és befizetést ment", async () => {
    let saved: Tx | null = null;
    render(<EntryForm years={[emptyYear(2026)]} onSaved={(t) => (saved = t)} onError={() => {}} />);
    fireEvent.change(screen.getByLabelText("Összeg"), { target: { value: "1000" } });
    await screen.findByText("400,00 Ft");
    expect(screen.getByText(/^400\s000 Ft$/)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Befizetés rögzítése" }));
    await waitFor(() => expect(saved).not.toBeNull());
    const add = calls.find((c) => c.cmd === "add_transaction")!;
    expect(add.args.input).toMatchObject({ currency: "EUR", amount: 1000, rate: null });
  });

  it("a kriptós fizetés negatív, payment típusú tételt küld", async () => {
    render(<EntryForm years={[]} onSaved={() => {}} onError={() => {}} />);
    fireEvent.click(screen.getByRole("radio", { name: /Fizetés/ }));
    expect(screen.getByText("Az áru vagy szolgáltatás értéke")).toBeTruthy();
    fireEvent.change(screen.getByLabelText("Összeg"), { target: { value: "25" } });
    await screen.findByText("400,00 Ft");
    fireEvent.click(screen.getByRole("button", { name: "Fizetés rögzítése" }));
    await waitFor(() => expect(calls.some((c) => c.cmd === "add_transaction")).toBe(true));
    const add = calls.find((c) => c.cmd === "add_transaction")!;
    expect(add.args.input).toMatchObject({ amount: -25, kind: "payment" });
  });

  it("a „…” menüből választott devizára kéri le az árfolyamot", async () => {
    render(<EntryForm years={[]} onSaved={() => {}} onError={() => {}} />);
    fireEvent.change(screen.getByLabelText("Más deviza"), { target: { value: "USD" } });
    await waitFor(() => expect(calls.some((c) => c.cmd === "get_mnb_rate" && c.args.currency === "USD")).toBe(true));
  });

  it("forintos tételnél nem kér árfolyamot", async () => {
    render(<EntryForm years={[]} onSaved={() => {}} onError={() => {}} />);
    calls = [];
    fireEvent.click(screen.getByRole("radio", { name: "Ft" }));
    fireEvent.change(screen.getByLabelText("Összeg"), { target: { value: "5000" } });
    // A magyar számformátum a négyjegyű számot nem tagolja (5000), az ötjegyűt igen (50 000).
    expect(screen.getByText("5000 Ft")).toBeTruthy();
    expect(calls.some((c) => c.cmd === "get_mnb_rate")).toBe(false);
  });
});

describe("appon belüli párbeszédablak", () => {
  function Probe({ onResult }: { onResult: (v: boolean) => void }) {
    const d = useDialogs();
    return <button onClick={async () => onResult(await d.confirm({ title: "Törlöd?", okLabel: "Törlés", danger: true }))}>kérdez</button>;
  }

  it("a megerősítés igen/nem választ ad", async () => {
    const results: boolean[] = [];
    render(
      <DialogHost>
        <Probe onResult={(v) => results.push(v)} />
      </DialogHost>,
    );
    fireEvent.click(screen.getByText("kérdez"));
    fireEvent.click(await screen.findByRole("button", { name: "Törlés" }));
    fireEvent.click(screen.getByText("kérdez"));
    fireEvent.click(await screen.findByRole("button", { name: "Mégse" }));
    await act(async () => {});
    expect(results).toEqual([true, false]);
  });
});
