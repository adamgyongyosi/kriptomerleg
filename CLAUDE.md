# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

Kriptomérleg: magyar kriptó-SZJA kalkulátor asztali app (Tauri 2 + React 19 + Rust, SQLite). A felület, a hibaüzenetek, a kódkommentek és a README magyarul vannak – így is maradjon.

## Parancsok

```bash
npm install
npm run tauri dev                                    # fejlesztői futtatás (Vite: 1420-as port)
npx tsc --noEmit                                     # típusellenőrzés (az `npm run build` is ezzel kezd)
npm test                                             # Vitest (jsdom), src/**/*.test.ts(x)
npx vitest run src/EntryForm.test.tsx -t "fizetés"   # egy tesztfájl / egy teszt
cargo test --manifest-path src-tauri/Cargo.toml                  # Rust tesztek
cargo test --manifest-path src-tauri/Cargo.toml small_value      # név szerinti szűrés
cd src-tauri && cargo audit                          # RustSec; kivétel indoklással: src-tauri/.cargo/audit.toml
```

Nincs linter-konfiguráció.

**Helyi build: csak a Windows hordozható exe** – `npm run tauri build -- --no-bundle`. Ha a link hibára fut a hosszú/szóközös útvonal miatt, `CARGO_TARGET_DIR=%LOCALAPPDATA%\kripto-ado-target`. A kimenetet `telepito\windows\kriptomerleg_<verzió>_windows_x64-portable.exe` néven kell elhelyezni. A telepítőt, a .deb-et és az AppImage-et a CI készíti; ezeket helyben ne buildeld.

## Architektúra

**Frontend ↔ backend:** minden művelet Tauri parancs. A `src/api.ts` az egyetlen híd: típusok (camelCase, a Rust oldalon `#[serde(rename_all = "camelCase")]`) és `invoke` wrapperek; böngészőben (nem az appban) a hívások hibával elutasítódnak. Új parancsnál három helyen kell módosítani: a függvény (`lib.rs` vagy `data.rs`), a `generate_handler!` lista a `lib.rs` végén, és a wrapper az `api.ts`-ben. A parancsok `Result<T, String>`-et adnak vissza, a hibaszöveg magyar, és a felhasználó látja.

**Hálózat csak a Rust oldalon:** a CSP (`tauri.conf.json` → `app.security.csp`) a webnézetből csak IPC-t enged. Az MNB árfolyam (`mnb.rs`, SOAP), a Kraken API (`import/kraken_api.rs`, HMAC-SHA512) és az updater a Rust oldalon fut. Fejlesztői módban (`devCsp: null`) nincs CSP, ezért egy CSP-hiba csak a buildelt appban jön elő.

**Állapot és profilok:** az `AppState { db: Arc<Mutex<Connection>>, dir, profile }` a `run()` setupjában jön létre. A profilváltás (`data.rs`) kicseréli a kapcsolatot. Az adatmappa azonosítója `hu.kriptoado.app` (a `tauri.conf.json` `identifier`), és az átnevezés ellenére sem változhat, mert különben elvesznek a felhasználói adatok. Profilonként külön SQLite fájl: alapértelmezett `kripto-ado.sqlite`, a többi `profiles/<id>.sqlite`.

**Séma és migráció:** `init_db` (`lib.rs`) `PRAGMA user_version` alapján migrál (jelenleg 2). Sémaváltozásnál új verziólépés kell, és a régi adatbázisoknak működniük kell (lásd a `migrates_old_database…` tesztet). A `transactions.huf` előjeles: pozitív = kiadás (befizetés, vétel), negatív = bevétel (kivétel, kriptós fizetés, eladás). Az adószámítás kizárólag ebből dolgozik.

**Adószámítás:** a `compute_summary_inner` (`lib.rs`) évenként összegzi a `huf`-ot SQL-lel. Ráteszi a hipotetikus tételt (élő adóhatás az űrlapon, `preview_tax`) és a kisértékű mentességet (`settings.kiserteku`), majd meghatározza a bevallatlan veszteséges éveket (`loss_declarations` + határidő-logika). Ezután a `tax::summarize` számol: 2022 előtti tételek 2022-be, 15% SZJA, adókiegyenlítés. Az eredmény sosem tárolódik. A `declaration_impact` / önellenőrzés-segéd ugyanezt futtatja módosított bevallatlan-halmazzal. Az adószabályok indoklása a README „Az adószámítás szabályai” szakaszában van; szabályváltozásnál azt is frissíteni kell.

**Import:** az `import/mod.rs` `parse_file` a fejlécek alapján választ formátumot (`detect`/`parse` modulonként). A sorrend számít: a `bank` az utolsó, mert az oszlopai általánosak. Minden parser `FiatFlow`-kat ad (csak a fiat oldal, díjjal), ezeket az `aggregate` napi tételekké vonja össze `external_id = forrás:dátum:deviza:buy|sell` kulccsal. Újraimportáláskor ez frissít, nem duplikál (`UPSERT_IMPORTED`). A más forrásból származó azonos napi tételt a `find_duplicate` jelöli. A kripto–kripto ügylet nem adóköteles, ezért nem kerül be. Az `examples/` mintafájljait tesztek olvassák be. Új formátumnál mintafájl és teszt is kell.

**Export és mentés:** a `pdf.rs` (printpdf, beágyazott DejaVu font a `src-tauri/fonts/`-ból) és a `data.rs` (xlsx, mentés, profilok) végzi. A `backup.rs` `VACUUM INTO`-val ment. Titkosítva: `KMBACKUP1` fejléc + Argon2id + ChaCha20-Poly1305. Ezeket a crate-verziókat (0.10 / 0.5) szándékosan rögzítettük.

**Frissítő:** a `src/updates.ts` a release `latest.json`-ját ellenőrzi, a pubkey a `tauri.conf.json`-ban van. Csak nagyobb verziószámra ajánl frissítést.

## Verzió és CI

- A verzió a `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, `nix/package.nix` és a `packaging/` fájljaiban szerepel. Ezeket együtt kell emelni.
- A `.github/workflows/build.yml` minden pushnál teszteket és auditot futtat, buildel Windowsra és Linuxra, és ellenőrzi a `nix build`-et. A verziót tag nélkül `<major.minor>.<run number>` alakban számolja. `main`-re pushnál aláírt release készül: a kulcs a `release` environment `TAURI_SIGNING_PRIVATE_KEY` titka. A release előbb piszkozat, utána `gh release edit --draft=false`, mert a release immutability miatt utólag nem módosítható.
- A `main` ágat és a `v*` tageket ruleset védi (CODEOWNERS: `@adamgyongyosi`).

## Weboldal (`site/`)

Statikus `index.html` (GitHub Pages, `.github/workflows/pages.yml`). Új app-funkciónál frissíteni kell a funkciókártyákat, a `#minden-funkcio` listát, a JSON-LD `featureList`-et és a `site/llms.txt`-t. A JSON-LD `FAQPage` szövegének szó szerint egyeznie kell a látható GYIK-kel. A megosztási kép (`site/og-image.png`) forrása a `brand/og-card.html`; az újragenerálás parancsa a fájl elején van.

## Konvenciók

- Minden változás ugyanabban a commitban a `README.md`-t (magyarul) is bővítse/igazítsa.
- A privát aláírókulcs csak GitHub *Secret* lehet (environment `release`), soha nem *Variable*.
- A Kraken API kulcs és a mentésjelszó nem tárolódhat és nem naplózható; SQL csak kötött paraméterekkel.
