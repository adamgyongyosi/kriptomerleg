//! Éves kriptoeszköz-nyilvántartás PDF-ben (Szja tv. 67/C. §): összesítő, veszteség-nyilvántartás
//! és a tételek. A betűkészlet (DejaVu Sans) a programba van fordítva, mert az ékezetes
//! betűkhöz (ő, ű) beágyazott font kell.

use crate::{Tx, kind_label, tax::YearSummary};
use printpdf::{Color, IndirectFontRef, Line, Mm, PdfDocument, PdfDocumentReference, PdfLayerReference, Point, Rgb};

const REGULAR: &[u8] = include_bytes!("../fonts/DejaVuSans.ttf");
const BOLD: &[u8] = include_bytes!("../fonts/DejaVuSans-Bold.ttf");

const PAGE_W: f32 = 210.0;
const PAGE_H: f32 = 297.0;
const MARGIN: f32 = 15.0;
const PT_TO_MM: f32 = 0.352_778;

pub struct Report<'a> {
    pub year: i32,
    pub profile: &'a str,
    pub generated: String,
    pub years: &'a [YearSummary],
    /// Az év tételei, időrendben.
    pub txs: &'a [Tx],
}

struct Fonts {
    regular: IndirectFontRef,
    bold: IndirectFontRef,
    reg_face: ttf_parser::Face<'static>,
    bold_face: ttf_parser::Face<'static>,
}

impl Fonts {
    fn width(&self, text: &str, size: f32, bold: bool) -> f32 {
        let face = if bold { &self.bold_face } else { &self.reg_face };
        let units: u32 = text
            .chars()
            .map(|c| face.glyph_index(c).and_then(|g| face.glyph_hor_advance(g)).unwrap_or(600) as u32)
            .sum();
        units as f32 / face.units_per_em() as f32 * size * PT_TO_MM
    }

    /// Levágja a szöveget, hogy elférjen a megadott szélességben.
    fn fit(&self, text: &str, size: f32, max: f32) -> String {
        if self.width(text, size, false) <= max {
            return text.to_string();
        }
        let mut out = String::new();
        for c in text.chars() {
            if self.width(&format!("{out}{c}…"), size, false) > max {
                break;
            }
            out.push(c);
        }
        format!("{out}…")
    }
}

/// Lapozó író: felülről lefelé halad, szükség esetén új oldalt nyit.
struct Writer {
    doc: PdfDocumentReference,
    layer: PdfLayerReference,
    fonts: Fonts,
    y: f32,
    page: usize,
    footer: String,
}

const INK: (f32, f32, f32) = (0.07, 0.08, 0.08);
const MUTED: (f32, f32, f32) = (0.42, 0.43, 0.41);
const LINE: (f32, f32, f32) = (0.80, 0.80, 0.77);

impl Writer {
    fn color(&self, c: (f32, f32, f32)) {
        self.layer.set_fill_color(Color::Rgb(Rgb::new(c.0, c.1, c.2, None)));
    }

    fn text(&self, s: &str, size: f32, x: f32, bold: bool, color: (f32, f32, f32)) {
        self.color(color);
        let font = if bold { &self.fonts.bold } else { &self.fonts.regular };
        self.layer.use_text(s, size, Mm(x), Mm(self.y), font);
    }

    fn text_right(&self, s: &str, size: f32, right: f32, bold: bool, color: (f32, f32, f32)) {
        let w = self.fonts.width(s, size, bold);
        self.text(s, size, right - w, bold, color);
    }

    fn rule(&self, y: f32, color: (f32, f32, f32), thickness: f32) {
        self.layer.set_outline_color(Color::Rgb(Rgb::new(color.0, color.1, color.2, None)));
        self.layer.set_outline_thickness(thickness);
        self.layer.add_line(Line {
            points: vec![(Point::new(Mm(MARGIN), Mm(y)), false), (Point::new(Mm(PAGE_W - MARGIN), Mm(y)), false)],
            is_closed: false,
        });
    }

    fn draw_footer(&self) {
        self.color(MUTED);
        let s = format!("{} · {}. oldal", self.footer, self.page);
        let w = self.fonts.width(&s, 7.5, false);
        self.layer.use_text(&s, 7.5, Mm(PAGE_W - MARGIN - w), Mm(9.0), &self.fonts.regular);
    }

    /// Új oldal, ha `need` mm már nem fér el.
    fn ensure(&mut self, need: f32) -> bool {
        if self.y - need >= 18.0 {
            return false;
        }
        let (p, l) = self.doc.add_page(Mm(PAGE_W), Mm(PAGE_H), "tartalom");
        self.layer = self.doc.get_page(p).get_layer(l);
        self.page += 1;
        self.y = PAGE_H - MARGIN - 4.0;
        self.draw_footer();
        true
    }

    fn gap(&mut self, mm: f32) {
        self.y -= mm;
    }

    fn heading(&mut self, s: &str) {
        self.ensure(18.0);
        self.gap(4.0);
        self.text(s, 11.5, MARGIN, true, INK);
        self.gap(2.5);
        self.rule(self.y, INK, 0.4);
        self.gap(5.0);
    }
}

pub fn ft(n: i64) -> String {
    group(&n.unsigned_abs().to_string(), n < 0) + " Ft"
}

fn group(digits: &str, negative: bool) -> String {
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push('\u{a0}');
        }
        out.push(c);
    }
    if negative { format!("−{out}") } else { out }
}

fn money(x: f64, decimals: usize) -> String {
    let s = format!("{:.decimals$}", x.abs());
    let (int, frac) = s.split_once('.').unwrap_or((&s, ""));
    let mut out = group(int, x < 0.0);
    if !frac.is_empty() {
        out.push(',');
        out.push_str(frac);
    }
    out
}

pub fn write(report: &Report, path: &std::path::Path) -> Result<(), String> {
    let title = format!("Kriptoeszköz-nyilvántartás {}", report.year);
    let (doc, page, layer) = PdfDocument::new(&title, Mm(PAGE_W), Mm(PAGE_H), "tartalom");
    let e = |e: printpdf::Error| e.to_string();
    let fonts = Fonts {
        regular: doc.add_external_font(REGULAR).map_err(e)?,
        bold: doc.add_external_font(BOLD).map_err(e)?,
        reg_face: ttf_parser::Face::parse(REGULAR, 0).map_err(|e| e.to_string())?,
        bold_face: ttf_parser::Face::parse(BOLD, 0).map_err(|e| e.to_string())?,
    };
    let layer = doc.get_page(page).get_layer(layer);
    let mut w = Writer {
        doc,
        layer,
        fonts,
        y: PAGE_H - MARGIN - 6.0,
        page: 1,
        footer: format!("Kriptomérleg · {} · {}", report.profile, report.year),
    };
    w.draw_footer();

    // Fejléc
    w.text(&title, 17.0, MARGIN, true, INK);
    w.gap(7.0);
    w.text(&format!("{} · készült: {}", report.profile, report.generated), 9.0, MARGIN, false, MUTED);
    w.gap(6.0);
    for line in [
        "Nyilvántartás a kriptoeszközzel végrehajtott ügyletekről (Szja tv. 67/C. §). Az összegek forintban, a teljesítés",
        "napján érvényes MNB árfolyamon (kézi árfolyamnál a bizonylat szerint). Az elévülés miatt 5 évig őrizd meg.",
    ] {
        w.text(line, 8.5, MARGIN, false, MUTED);
        w.gap(4.2);
    }

    // Összesítő
    let s = report.years.iter().find(|y| y.year == report.year);
    w.heading("Éves összesítő");
    match s {
        Some(s) if s.pre_regime => {
            w.text("2022 előtti év: a NAV szerint ezek az ügyletek a 2022-es eredménybe számítanak.", 9.5, MARGIN, false, INK);
            w.gap(5.5);
        }
        Some(s) => {
            let mut rows: Vec<(&str, String)> = vec![
                ("Kiadás (befizetés, vétel)", ft(s.deposits_huf)),
                ("Bevétel (kivétel, eladás, kriptóval fizetés)", ft(s.withdrawals_huf)),
            ];
            if s.small_exempt_huf > 0 {
                rows.push(("Ebből kisértékű ügylet, nem számít (67/C. § (2))", ft(s.small_exempt_huf)));
            }
            if s.carried_in_huf != 0 {
                rows.push(("2022 előtti évek eredménye (ide számít)", ft(s.carried_in_huf)));
            }
            rows.push((if s.result_huf >= 0 { "Jövedelem (nyereség)" } else { "Veszteség" }, ft(s.result_huf.abs())));
            rows.push(("Adó (15%)", ft(s.tax_gross)));
            rows.push(("Adókiegyenlítés (korábbi veszteség)", ft(s.offset_used)));
            rows.push((if s.tax_due < 0 { "Visszajáró adó" } else { "Fizetendő adó" }, ft(s.tax_due.abs())));
            for (label, value) in rows {
                w.text(label, 9.5, MARGIN, false, INK);
                w.text_right(&value, 9.5, PAGE_W - MARGIN, false, INK);
                w.gap(2.0);
                w.rule(w.y, LINE, 0.2);
                w.gap(3.6);
            }
        }
        None => {
            w.text("Ebben az évben nincs tétel.", 9.5, MARGIN, false, INK);
            w.gap(5.5);
        }
    }

    // Veszteség-nyilvántartás
    let losses: Vec<&YearSummary> = report.years.iter().filter(|y| y.year <= report.year && y.loss_credit > 0).collect();
    if !losses.is_empty() {
        w.heading("Veszteség-nyilvántartás (67/C. § (7a))");
        let cols = [(MARGIN, "Év", false), (95.0, "Veszteség", true), (140.0, "Bevallva", true), (PAGE_W - MARGIN, "Fel nem használt", true)];
        for (x, h, right) in cols {
            if right { w.text_right(h, 8.5, x, true, MUTED) } else { w.text(h, 8.5, x, true, MUTED) }
        }
        w.gap(5.0);
        for l in losses {
            let unused = (l.loss_credit_remaining as f64 / 0.15).round() as i64;
            w.text(&l.year.to_string(), 9.5, MARGIN, false, INK);
            w.text_right(&ft(-l.result_huf), 9.5, 95.0, false, INK);
            w.text_right(if l.loss_declared { "igen" } else { "nem" }, 9.5, 140.0, false, INK);
            w.text_right(&if l.loss_declared { ft(unused) } else { "nem vonható le".into() }, 9.5, PAGE_W - MARGIN, false, INK);
            w.gap(5.2);
        }
    }

    // Tételek
    w.heading(&format!("Tételek ({} db)", report.txs.len()));
    // Oszlopok: (bal szél / jobb szél, fejléc, jobbra igazított)
    let header = |w: &mut Writer| {
        let heads = [
            (MARGIN, "Dátum", false),
            (36.0, "Típus", false),
            (64.0, "Forrás", false),
            (111.0, "Összeg", true),
            (133.0, "Árfolyam", true),
            (158.0, "Forint", true),
            (161.0, "Megjegyzés", false),
        ];
        for (x, h, right) in heads {
            if right { w.text_right(h, 7.8, x, true, MUTED) } else { w.text(h, 7.8, x, true, MUTED) }
        }
        w.gap(1.8);
        w.rule(w.y, LINE, 0.3);
        w.gap(3.8);
    };
    header(&mut w);
    if report.txs.is_empty() {
        w.text("Nincs tétel.", 9.0, MARGIN, false, MUTED);
    }
    for t in report.txs {
        if w.ensure(6.0) {
            header(&mut w);
        }
        let size = 8.0;
        w.text(&t.date, size, MARGIN, false, INK);
        w.text(kind_label(&t.kind, t.amount), size, 36.0, false, INK);
        w.text(&w.fonts.fit(&crate::capitalize(&t.source), size, 22.0), size, 64.0, false, INK);
        w.text_right(&format!("{} {}", money(t.amount.abs(), 2), t.currency), size, 111.0, false, INK);
        let rate = if t.currency == "HUF" { "–".to_string() } else { format!("{}{}", money(t.rate, 2), if t.rate_source == "kézi" { " k" } else { "" }) };
        w.text_right(&rate, size, 133.0, false, INK);
        w.text_right(&ft(t.huf.abs()), size, 158.0, true, INK);
        w.text(&w.fonts.fit(&t.note, 7.2, PAGE_W - MARGIN - 161.0), 7.2, 161.0, false, MUTED);
        w.gap(4.6);
    }
    w.gap(2.0);
    w.ensure(10.0);
    w.text(
        "Árfolyam: MNB hivatalos középárfolyam a teljesítés napján (hétvégén az utolsó közzétett); „k” = kézzel megadott.",
        7.2,
        MARGIN,
        false,
        MUTED,
    );

    let file = std::fs::File::create(path).map_err(|e| format!("Nem sikerült menteni: {e}"))?;
    w.doc.save(&mut std::io::BufWriter::new(file)).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};

    #[test]
    fn formats_numbers() {
        assert_eq!(ft(1_234_567), "1\u{a0}234\u{a0}567 Ft");
        assert_eq!(ft(-500), "−500 Ft");
        assert_eq!(money(1234.5, 2), "1\u{a0}234,50");
    }

    #[test]
    fn writes_multipage_pdf() {
        let years = crate::tax::summarize(&BTreeMap::from([(2025, (900_000, 100_000)), (2026, (100_000, 400_000))]), 2026, &BTreeSet::new());
        let txs: Vec<Tx> = (0..120)
            .map(|i| Tx {
                id: i,
                date: format!("2026-{:02}-{:02}", i % 12 + 1, i % 28 + 1),
                currency: "EUR".into(),
                amount: if i % 3 == 0 { -150.25 } else { 1000.0 },
                rate: 391.37,
                rate_source: "MNB".into(),
                rate_date: None,
                huf: if i % 3 == 0 { -58_806 } else { 391_370 },
                note: "Binance · 3 vétel · díj 1,20 EUR – nagyon hosszú megjegyzés, ami nem fér ki".into(),
                source: "binance".into(),
                kind: if i % 3 == 0 { "sell".into() } else { "buy".into() },
                fee: 0.0,
            })
            .collect();
        let report = Report { year: 2026, profile: "Saját", generated: "2026-10-31".into(), years: &years, txs: &txs };
        let path = std::env::temp_dir().join(format!("kriptomerleg-{}.pdf", std::process::id()));
        write(&report, &path).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert!(bytes.starts_with(b"%PDF"));
        // Szemrevételezéshez: KRIPTOMERLEG_PDF_OUT=minta.pdf cargo test writes_multipage_pdf
        if let Ok(out) = std::env::var("KRIPTOMERLEG_PDF_OUT") {
            std::fs::copy(&path, out).unwrap();
        }
        let _ = std::fs::remove_file(path);
    }
}
