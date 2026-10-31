import { createContext, ReactNode, useCallback, useContext, useEffect, useRef, useState } from "react";

/**
 * Az appon belüli megerősítő, beviteli és értesítő ablakok. A rendszer üzenetablakai helyett
 * ezeket használjuk, mert Linuxon (GTK séma nélkül, illetve XDG portálon) a natív ablak nem
 * mindenhol érhető el.
 */
interface Request {
  kind: "confirm" | "prompt" | "alert";
  title: string;
  message?: string;
  okLabel?: string;
  cancelLabel?: string;
  danger?: boolean;
  defaultValue?: string;
  resolve: (value: boolean | string | null) => void;
}

type Options = Omit<Request, "kind" | "resolve">;

interface Dialogs {
  confirm: (o: Options) => Promise<boolean>;
  prompt: (o: Options) => Promise<string | null>;
  alert: (o: Options) => Promise<void>;
}

const Ctx = createContext<Dialogs | null>(null);

export function useDialogs(): Dialogs {
  const d = useContext(Ctx);
  if (!d) throw new Error("useDialogs: hiányzó DialogHost");
  return d;
}

export function DialogHost({ children }: { children: ReactNode }) {
  const [req, setReq] = useState<Request | null>(null);
  const [value, setValue] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);
  const okRef = useRef<HTMLButtonElement>(null);

  const open = useCallback(
    (kind: Request["kind"], o: Options) =>
      new Promise<boolean | string | null>((resolve) => {
        setValue(o.defaultValue ?? "");
        setReq({ ...o, kind, resolve });
      }),
    [],
  );

  const api: Dialogs = {
    confirm: (o) => open("confirm", o).then((v) => v === true),
    prompt: (o) => open("prompt", o).then((v) => (typeof v === "string" ? v : null)),
    alert: (o) => open("alert", o).then(() => undefined),
  };

  useEffect(() => {
    if (!req) return;
    (req.kind === "prompt" ? inputRef.current : okRef.current)?.focus();
    inputRef.current?.select();
  }, [req]);

  const close = (result: boolean | string | null) => {
    req?.resolve(result);
    setReq(null);
  };
  const ok = () => close(req?.kind === "prompt" ? value : true);
  const cancel = () => close(req?.kind === "prompt" ? null : false);

  return (
    <Ctx.Provider value={api}>
      {children}
      {req && (
        <div className="modal-backdrop" onMouseDown={(e) => e.target === e.currentTarget && cancel()}>
          <div
            className="modal dialog"
            role="alertdialog"
            aria-modal="true"
            aria-labelledby="dialog-title"
            onKeyDown={(e) => {
              if (e.key === "Escape") cancel();
            }}
          >
            <h2 id="dialog-title">{req.title}</h2>
            {req.message && <p className="dialog-message">{req.message}</p>}
            {req.kind === "prompt" && (
              <input
                id="dialog-input"
                ref={inputRef}
                value={value}
                onChange={(e) => setValue(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && ok()}
              />
            )}
            <div className="form-actions">
              <span className="spacer" />
              {req.kind !== "alert" && (
                <button className="btn ghost" onClick={cancel}>
                  {req.cancelLabel ?? "Mégse"}
                </button>
              )}
              <button ref={okRef} className={`btn ${req.danger ? "danger" : "primary"}`} onClick={ok}>
                {req.okLabel ?? "OK"}
              </button>
            </div>
          </div>
        </div>
      )}
    </Ctx.Provider>
  );
}
