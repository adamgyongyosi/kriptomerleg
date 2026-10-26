import { useEffect, useRef, useState } from "react";
import { fmtFt } from "./api";

export interface Series {
  label: string;
  /** CSS változó neve, pl. "--series-1". */
  color: string;
  values: number[];
}

interface Props {
  categories: string[];
  series: Series[];
  /** Plusz sor a tooltipben (pl. nettó eredmény). */
  extra?: (index: number) => { label: string; value: string } | null;
  height?: number;
}

const compact = new Intl.NumberFormat("hu-HU", { notation: "compact", maximumFractionDigits: 1 });

function niceStep(max: number, ticks: number) {
  const raw = max / ticks;
  const mag = 10 ** Math.floor(Math.log10(raw));
  const norm = raw / mag;
  return (norm <= 1 ? 1 : norm <= 2 ? 2 : norm <= 2.5 ? 2.5 : norm <= 5 ? 5 : 10) * mag;
}

/** Oszlop lekerekített (4px) tetővel, szögletes alappal. */
function columnPath(x: number, y: number, w: number, h: number) {
  const r = Math.min(4, w / 2, h);
  return `M${x},${y + h}V${y + r}Q${x},${y} ${x + r},${y}H${x + w - r}Q${x + w},${y} ${x + w},${y + r}V${y + h}Z`;
}

export function GroupedColumns({ categories, series, extra, height = 240 }: Props) {
  const wrapRef = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(600);
  const [hover, setHover] = useState<number | null>(null);

  useEffect(() => {
    const el = wrapRef.current;
    if (!el) return;
    const ro = new ResizeObserver(([e]) => setWidth(Math.max(280, e.contentRect.width)));
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  const m = { top: 12, right: 8, bottom: 26, left: 52 };
  const plotW = width - m.left - m.right;
  const plotH = height - m.top - m.bottom;

  const max = Math.max(1, ...series.flatMap((s) => s.values));
  const step = niceStep(max, 4);
  const top = Math.ceil(max / step) * step;
  const ticks = Array.from({ length: Math.round(top / step) + 1 }, (_, i) => i * step);
  const y = (v: number) => m.top + plotH - (v / top) * plotH;

  const band = plotW / categories.length;
  const gap = 2;
  const barW = Math.max(3, Math.min(24, (band * 0.7 - gap * (series.length - 1)) / series.length));
  const groupW = barW * series.length + gap * (series.length - 1);

  const tipLeft = hover === null ? 0 : m.left + band * hover + band / 2;
  const flip = tipLeft > width * 0.65;

  return (
    <div className="chart" ref={wrapRef}>
      <div className="legend">
        {series.map((s) => (
          <span key={s.label}>
            <i style={{ background: `var(${s.color})` }} />
            {s.label}
          </span>
        ))}
      </div>
      <svg width={width} height={height} role="img" onMouseLeave={() => setHover(null)}>
        {ticks.map((t) => (
          <g key={t}>
            <line className={t === 0 ? "baseline" : "grid"} x1={m.left} x2={width - m.right} y1={y(t)} y2={y(t)} />
            <text className="tick" x={m.left - 8} y={y(t)} dy="0.32em" textAnchor="end">
              {compact.format(t)}
            </text>
          </g>
        ))}
        {categories.map((c, i) => {
          const x0 = m.left + band * i + (band - groupW) / 2;
          return (
            <g key={c}>
              {hover === i && <rect className="hover-band" x={m.left + band * i} y={m.top} width={band} height={plotH} />}
              {series.map((s, j) => {
                const v = s.values[i];
                if (!v) return null;
                const h = Math.max(1, (v / top) * plotH);
                return (
                  <path
                    key={s.label}
                    d={columnPath(x0 + j * (barW + gap), m.top + plotH - h, barW, h)}
                    fill={`var(${s.color})`}
                  />
                );
              })}
              <text className="tick" x={m.left + band * i + band / 2} y={height - 8} textAnchor="middle">
                {c}
              </text>
              <rect
                x={m.left + band * i}
                y={m.top}
                width={band}
                height={plotH + m.bottom}
                fill="transparent"
                onMouseEnter={() => setHover(i)}
              />
            </g>
          );
        })}
      </svg>
      {hover !== null && (
        <div
          className="tooltip"
          style={{ left: tipLeft, top: m.top, transform: flip ? "translateX(calc(-100% - 12px))" : "translateX(12px)" }}
        >
          <div className="tooltip-title">{categories[hover]}</div>
          {series.map((s) => (
            <div key={s.label} className="tooltip-row">
              <i style={{ background: `var(${s.color})` }} />
              <span>{s.label}</span>
              <strong>{fmtFt(s.values[hover])}</strong>
            </div>
          ))}
          {(() => {
            const e = extra?.(hover);
            return e ? (
              <div className="tooltip-row extra">
                <span>{e.label}</span>
                <strong>{e.value}</strong>
              </div>
            ) : null;
          })()}
        </div>
      )}
    </div>
  );
}
