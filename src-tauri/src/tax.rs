//! Kriptoeszköz-ügyletek SZJA számítása (Szja tv. 67/C. §).
//!
//! - Éves eredmény = tárgyévi bevétel (kivételek) − tárgyévi kiadás (befizetések).
//! - Nyereség után 15% adó.
//! - Adókiegyenlítés (67/C. § (5)–(7)): a veszteség 15%-a érvényesíthető, legfeljebb a tárgyévben
//!   és az azt megelőző két évben bevallott kriptojövedelmek adójáig, csökkentve a korábbi két évben
//!   már érvényesített adókiegyenlítéssel. Ha ez több a tárgyévi adónál, a különbözet visszajár.
//! - Korábbi év vesztesége csak akkor vehető figyelembe, ha a keletkezés évének bevallásában
//!   feltüntették. A 2025-ös bevallástól (2025. évi LXXXIII. tv.) ennek nincs időkorlátja; előtte a
//!   tárgyévi és az előző két évi veszteség számított, ami 2022-től kezdve ugyanazt adja.
//! - A 2022 előtti, korábban nem adózott ügyletek a NAV szerint a 2022-es eredménybe számítanak.

use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

pub const RATE_PERCENT: i64 = 15;
/// Az első év, amelyre a 67/C. § vonatkozik.
pub const FIRST_YEAR: i32 = 2022;

#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct YearSummary {
    pub year: i32,
    pub deposits_huf: i64,
    pub withdrawals_huf: i64,
    /// Pozitív: nyereség, negatív: veszteség.
    pub result_huf: i64,
    /// A nyereség 15%-a (ez a bevallott kriptojövedelem adója).
    pub tax_gross: i64,
    /// Adókiegyenlítési keret: a tárgyévi és az előző két évi adó, mínusz az előző két évben érvényesített kiegyenlítés.
    pub offset_cap: i64,
    /// Az évben figyelembe vehető, még fel nem használt veszteségek 15%-a (a tárgyévit is beleértve).
    pub loss_pool: i64,
    /// Érvényesített adókiegyenlítés.
    pub offset_used: i64,
    /// Fizetendő adó; negatív érték esetén visszajár.
    pub tax_due: i64,
    /// A tárgyévi veszteség 15%-a.
    pub loss_credit: i64,
    /// A tárgyévi veszteségből (15%-os értékben) a számítás végén még fel nem használt rész.
    pub loss_credit_remaining: i64,
    /// Ennyi további kivétel még nem növelné az évi fizetendő adót (és nem csökkentené a visszajárót).
    pub tax_free_headroom: i64,
    /// 2022 előtti év: az eredménye a 2022-es évbe számít, külön adója nincs.
    pub pre_regime: bool,
    /// 2022-ben: a 2022 előtti évek eredménye (kivételek − befizetések), amely itt számít.
    pub carried_in_huf: i64,
    /// A veszteséget a keletkezés évének bevallásában feltüntették (később csak így vonható le).
    pub loss_declared: bool,
}

fn pct(x: i64) -> i64 {
    // Kerekítés egész forintra.
    (x * RATE_PERCENT + 50).div_euclid(100)
}

/// `totals`: év -> (befizetések Ft, kivételek Ft), mindkettő pozitív szám.
/// `undeclared`: azok az évek, amelyek veszteségét nem vallották be, így később nem vonható le.
/// Az eredmény a legkorábbi évtől `through_year`-ig (vagy az utolsó adatos évig) tart.
pub fn summarize(totals: &BTreeMap<i32, (i64, i64)>, through_year: i32, undeclared: &BTreeSet<i32>) -> Vec<YearSummary> {
    let mut out = compute(totals, through_year, undeclared);
    for s in &mut out {
        if !s.pre_regime {
            s.tax_free_headroom = headroom(totals, through_year, undeclared, s.year, s.tax_due);
        }
    }
    out
}

fn compute(totals: &BTreeMap<i32, (i64, i64)>, through_year: i32, undeclared: &BTreeSet<i32>) -> Vec<YearSummary> {
    let Some(&first) = totals.keys().next() else { return Vec::new() };
    // 2022 előtti adat esetén a 2022-es év mindenképp kell, mert oda számít.
    let mut last = through_year.max(*totals.keys().last().unwrap());
    if first < FIRST_YEAR {
        last = last.max(FIRST_YEAR);
    }

    let mut out: Vec<YearSummary> = Vec::new();
    // Veszteség éve -> fel nem használt adóérték (a veszteség 15%-a).
    let mut credits: BTreeMap<i32, i64> = BTreeMap::new();

    for year in first..=last {
        let (deposits, withdrawals) = totals.get(&year).copied().unwrap_or((0, 0));
        if year < FIRST_YEAR {
            out.push(YearSummary {
                year,
                deposits_huf: deposits,
                withdrawals_huf: withdrawals,
                result_huf: 0,
                tax_gross: 0,
                offset_cap: 0,
                loss_pool: 0,
                offset_used: 0,
                tax_due: 0,
                loss_credit: 0,
                loss_credit_remaining: 0,
                tax_free_headroom: 0,
                pre_regime: true,
                carried_in_huf: 0,
                loss_declared: true,
            });
            continue;
        }
        let carried_in: i64 =
            if year == FIRST_YEAR { totals.range(..FIRST_YEAR).map(|(_, &(d, w))| w - d).sum() } else { 0 };
        let result = withdrawals - deposits + carried_in;
        let tax_gross = if result > 0 { pct(result) } else { 0 };
        let loss_credit = if result < 0 { pct(-result) } else { 0 };
        if loss_credit > 0 {
            credits.insert(year, loss_credit);
        }

        let prev: Vec<&YearSummary> = out.iter().filter(|s| s.year >= year - 2).collect();
        let offset_cap = (tax_gross + prev.iter().map(|s| s.tax_gross).sum::<i64>()
            - prev.iter().map(|s| s.offset_used).sum::<i64>())
        .max(0);

        let loss_pool: i64 = credits.values().sum();
        let offset_used = offset_cap.min(loss_pool);

        // A legrégebbi veszteséget használjuk fel először.
        let mut to_use = offset_used;
        for c in credits.values_mut() {
            let take = (*c).min(to_use);
            *c -= take;
            to_use -= take;
        }
        // A tárgyévi veszteség a saját bevallásában érvényesíthető, később csak ha ott feltüntették.
        let loss_declared = !undeclared.contains(&year);
        if !loss_declared {
            credits.remove(&year);
        }

        out.push(YearSummary {
            year,
            deposits_huf: deposits,
            withdrawals_huf: withdrawals,
            result_huf: result,
            tax_gross,
            offset_cap,
            loss_pool,
            offset_used,
            tax_due: tax_gross - offset_used,
            loss_credit,
            loss_credit_remaining: 0,
            tax_free_headroom: 0,
            pre_regime: false,
            carried_in_huf: carried_in,
            loss_declared,
        });
    }

    for s in &mut out {
        s.loss_credit_remaining = credits.get(&s.year).copied().unwrap_or(0);
    }
    out
}

/// A legnagyobb további kivétel az adott évben, amely nem rontja az év adóegyenlegét.
fn headroom(
    totals: &BTreeMap<i32, (i64, i64)>,
    through_year: i32,
    undeclared: &BTreeSet<i32>,
    year: i32,
    due: i64,
) -> i64 {
    let due_with = |extra: i64| {
        let mut t = totals.clone();
        t.entry(year).or_default().1 += extra;
        compute(&t, through_year, undeclared).into_iter().find(|s| s.year == year).map_or(due, |s| s.tax_due)
    };
    let ok = |x: i64| due_with(x) <= due;
    let (mut lo, mut hi) = (0_i64, 1_000_000_000_000_i64);
    if ok(hi) {
        return hi;
    }
    while hi - lo > 1 {
        let mid = lo + (hi - lo) / 2;
        if ok(mid) { lo = mid } else { hi = mid }
    }
    lo
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(rows: &[(i32, i64, i64)], through: i32) -> Vec<YearSummary> {
        run_with(rows, through, &[])
    }

    fn run_with(rows: &[(i32, i64, i64)], through: i32, undeclared: &[i32]) -> Vec<YearSummary> {
        let totals = rows.iter().map(|&(y, d, w)| (y, (d, w))).collect();
        summarize(&totals, through, &undeclared.iter().copied().collect())
    }

    fn year(s: &[YearSummary], y: i32) -> &YearSummary {
        s.iter().find(|s| s.year == y).unwrap()
    }

    #[test]
    fn gain_is_taxed_at_15_percent() {
        let s = run(&[(2026, 1_000_000, 3_000_000)], 2026);
        assert_eq!(s[0].result_huf, 2_000_000);
        assert_eq!(s[0].tax_due, 300_000);
        // Kerekítés miatt néhány forint belefér.
        assert!(s[0].tax_free_headroom < 10);
    }

    #[test]
    fn pre_2022_years_count_in_2022() {
        // A 2021-es befizetés a 2022-es eredményt csökkenti, külön veszteséges év nincs.
        let s = run(&[(2021, 1_000_000, 0), (2022, 0, 1_500_000)], 2022);
        assert!(year(&s, 2021).pre_regime);
        assert_eq!(year(&s, 2021).loss_credit, 0);
        let y22 = year(&s, 2022);
        assert_eq!(y22.carried_in_huf, -1_000_000);
        assert_eq!(y22.result_huf, 500_000);
        assert_eq!(y22.tax_due, 75_000);

        // Ha 2022-ben nincs tétel, akkor is ott jelenik meg.
        let s = run(&[(2021, 1_000_000, 0)], 2021);
        assert_eq!(year(&s, 2022).result_huf, -1_000_000);
    }

    #[test]
    fn declared_old_loss_has_no_time_limit() {
        // 2022: 1 M veszteség; 2024: 400e nyereség (60e adó, levonva); 2025: 600e nyereség.
        // 2025 kerete 0 + 60e + 90e − 60e = 90e, a maradék 600e veszteség adóértéke is 90e.
        let s = run(&[(2022, 1_000_000, 0), (2024, 0, 400_000), (2025, 0, 600_000)], 2025);
        assert_eq!(year(&s, 2024).offset_used, 60_000);
        let y25 = year(&s, 2025);
        assert_eq!(y25.offset_cap, 90_000);
        assert_eq!(y25.offset_used, 90_000);
        assert_eq!(y25.tax_due, 0);
        assert_eq!(year(&s, 2022).loss_credit_remaining, 0);
    }

    #[test]
    fn undeclared_loss_is_not_carried_forward() {
        let rows = [(2023, 2_000_000, 0), (2025, 0, 1_000_000)];
        let s = run_with(&rows, 2025, &[2023]);
        assert!(!year(&s, 2023).loss_declared);
        assert_eq!(year(&s, 2025).loss_pool, 0);
        assert_eq!(year(&s, 2025).tax_due, 150_000);

        let s = run_with(&rows, 2025, &[]);
        assert_eq!(year(&s, 2025).tax_due, 0);
    }

    #[test]
    fn undeclared_flag_does_not_block_same_year_offset() {
        // A tárgyévi veszteség a saját bevallásában visszaigényelheti az előző évek adóját.
        let s = run_with(&[(2024, 0, 1_000_000), (2025, 500_000, 0)], 2025, &[2025]);
        assert_eq!(year(&s, 2025).tax_due, -75_000);
    }

    #[test]
    fn loss_reclaims_tax_of_previous_two_years() {
        // Cikkbeli példa: 2023, 2024: 1-1 M nyereség (150e adó), 2025: 1,5 M veszteség -> 225e visszajár.
        let s = run(&[(2023, 0, 1_000_000), (2024, 0, 1_000_000), (2025, 1_500_000, 0)], 2025);
        let y25 = &s[2];
        assert_eq!(y25.offset_cap, 300_000);
        assert_eq!(y25.offset_used, 225_000);
        assert_eq!(y25.tax_due, -225_000);
    }

    #[test]
    fn cap_reduced_by_earlier_offsets() {
        let s = run(&[(2024, 0, 1_000_000), (2025, 500_000, 0), (2026, 2_000_000, 0)], 2026);
        // 2025: 75e veszteség-adóérték, keret 150e -> 75e visszajár.
        assert_eq!(s[1].tax_due, -75_000);
        // 2026: keret = 150e - 75e = 75e, a 300e-s veszteségből 75e érvényesíthető.
        assert_eq!(s[2].offset_cap, 75_000);
        assert_eq!(s[2].tax_due, -75_000);
        assert_eq!(s[2].loss_credit_remaining, 225_000);
    }

    #[test]
    fn headroom_uses_remaining_loss() {
        // 2025: 1 M veszteség (150e). 2026: 0 adat. 2026-ban 1 M kivétel adómentes.
        let s = run(&[(2025, 1_000_000, 0)], 2026);
        let h = s[1].tax_free_headroom;
        assert!((999_990..=1_000_010).contains(&h), "{h}");
    }
}
