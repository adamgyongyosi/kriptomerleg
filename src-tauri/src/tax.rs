//! Kriptoeszköz-ügyletek SZJA számítása (Szja tv. 67/B. §).
//!
//! - Éves eredmény = tárgyévi bevétel (kivételek) − tárgyévi kiadás (befizetések).
//! - Nyereség után 15% adó.
//! - Adókiegyenlítés: a veszteség 15%-a érvényesíthető, legfeljebb a tárgyévben és az azt
//!   megelőző két évben bevallott kriptojövedelmek adójáig, csökkentve a korábbi két évben
//!   már érvényesített adókiegyenlítéssel. Ha ez több a tárgyévi adónál, a különbözet visszajár.
//! - A 2024-es bevallásig csak a tárgyévi és az előző két évi veszteség volt figyelembe vehető;
//!   a 2025-ös bevallástól (2025. évi LXXXIII. tv.) bármely korábbi év még fel nem használt,
//!   bevallott vesztesége.

use serde::Serialize;
use std::collections::BTreeMap;

pub const RATE_PERCENT: i64 = 15;
/// Az első bevallási év, amelyben a veszteség kora már nem számít.
pub const UNLIMITED_LOSS_FROM: i32 = 2025;

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
}

fn pct(x: i64) -> i64 {
    // Kerekítés egész forintra.
    (x * RATE_PERCENT + 50).div_euclid(100)
}

/// `totals`: év -> (befizetések Ft, kivételek Ft), mindkettő pozitív szám.
/// Az eredmény a legkorábbi évtől `through_year`-ig (vagy az utolsó adatos évig) tart.
pub fn summarize(totals: &BTreeMap<i32, (i64, i64)>, through_year: i32) -> Vec<YearSummary> {
    let mut out = compute(totals, through_year);
    for s in &mut out {
        s.tax_free_headroom = headroom(totals, through_year, s.year, s.tax_due);
    }
    out
}

fn compute(totals: &BTreeMap<i32, (i64, i64)>, through_year: i32) -> Vec<YearSummary> {
    let Some(&first) = totals.keys().next() else { return Vec::new() };
    let last = through_year.max(*totals.keys().last().unwrap());

    let mut out: Vec<YearSummary> = Vec::new();
    // Veszteség éve -> fel nem használt adóérték (a veszteség 15%-a).
    let mut credits: BTreeMap<i32, i64> = BTreeMap::new();

    for year in first..=last {
        let (deposits, withdrawals) = totals.get(&year).copied().unwrap_or((0, 0));
        let result = withdrawals - deposits;
        let tax_gross = if result > 0 { pct(result) } else { 0 };
        let loss_credit = if result < 0 { pct(-result) } else { 0 };
        if loss_credit > 0 {
            credits.insert(year, loss_credit);
        }

        let prev: Vec<&YearSummary> = out.iter().filter(|s| s.year >= year - 2).collect();
        let offset_cap = (tax_gross + prev.iter().map(|s| s.tax_gross).sum::<i64>()
            - prev.iter().map(|s| s.offset_used).sum::<i64>())
        .max(0);

        let from = if year >= UNLIMITED_LOSS_FROM { i32::MIN } else { year - 2 };
        let loss_pool: i64 = credits.range(from..=year).map(|(_, c)| c).sum();
        let offset_used = offset_cap.min(loss_pool);

        // A legrégebbi veszteséget használjuk fel először.
        let mut to_use = offset_used;
        for (_, c) in credits.range_mut(from..=year) {
            let take = (*c).min(to_use);
            *c -= take;
            to_use -= take;
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
        });
    }

    for s in &mut out {
        s.loss_credit_remaining = credits.get(&s.year).copied().unwrap_or(0);
    }
    out
}

/// A legnagyobb további kivétel az adott évben, amely nem rontja az év adóegyenlegét.
fn headroom(totals: &BTreeMap<i32, (i64, i64)>, through_year: i32, year: i32, due: i64) -> i64 {
    let due_with = |extra: i64| {
        let mut t = totals.clone();
        t.entry(year).or_default().1 += extra;
        compute(&t, through_year).into_iter().find(|s| s.year == year).map_or(due, |s| s.tax_due)
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
        let totals = rows.iter().map(|&(y, d, w)| (y, (d, w))).collect();
        summarize(&totals, through)
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
    fn old_regime_loss_expires_after_two_years() {
        // 2020-as veszteség a 2024-es bevallásban már nem használható (régi szabály).
        let s = run(&[(2020, 1_000_000, 0), (2024, 0, 1_000_000)], 2024);
        assert_eq!(s[4].loss_pool, 0);
        assert_eq!(s[4].tax_due, 150_000);
    }

    #[test]
    fn new_regime_revives_old_unused_loss() {
        // 2021-es fel nem használt veszteség a 2025-ös bevallástól újra érvényesíthető.
        let s = run(&[(2021, 1_000_000, 0), (2025, 0, 600_000)], 2025);
        let y25 = s.iter().find(|y| y.year == 2025).unwrap();
        assert_eq!(y25.loss_pool, 150_000);
        assert_eq!(y25.offset_used, 90_000);
        assert_eq!(y25.tax_due, 0);
        assert_eq!(s[0].loss_credit_remaining, 60_000);
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
