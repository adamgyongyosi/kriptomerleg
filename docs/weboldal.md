# Weboldal

[← vissza a README-hez](../README.md)

A landing oldal a `site/` mappában van: egyetlen statikus `index.html` (külső függőség nélkül), a logó és az ikon. Sötét/világos módot és mobilnézetet is kezel, a letöltőgombok a legfrissebb release fájljaira mutatnak, és kiemeli a látogató rendszeréhez illőt.

- Helyi előnézet: `npx vite` (vagy `npm run dev`), majd <http://localhost:1420/site/index.html>
- Közzététel: a `.github/workflows/pages.yml` minden `site/` változásnál feltölti a GitHub Pages-re.
- **Egyszeri beállítás:** Settings → Pages → Source: *GitHub Actions*. Ingyenes GitHub-fiókkal a Pages (és a mindenki által letölthető release) csak **publikus** repóból működik.
- A repó neve egy helyen szerepel a lap alján lévő szkriptben (`REPO`), átnevezésnél csak ezt kell módosítani.
- Felépítés: hero valódi képernyőképpel → támogatott források → adatvédelmi sáv → **Bemutató** (öt kép–szöveg blokk: import, ellenőrzés, adótervező, évek és adó, bevallási segéd, plusz három további kép) → funkciókártyák → lépések → összevetés → letöltés → GYIK → záró felhívás. Mobilon a menü vízszintesen görgethető sor a fejléc alatt.
- A funkciókártyák (9 db), a „Minden funkció részletesen” lenyíló lista (`#minden-funkcio`, 6 csoport: rögzítés, adószámítás, bevallás, import, export és mentés, az app), az adatvédelmi sáv, az összevetés és a GYIK az app funkcióit követi; új funkciónál ezeket, a JSON-LD `featureList`-et és a `site/llms.txt`-t is frissítsd.
- **Animációk** (külső könyvtár nélkül, csak `transform`/`opacity`): lassan mozgó színfoltok és színátmenetes szöveg a hero-ban, a hero kép 3D-s beúszása és egérrel követett dőlése (csak egérrel, széles kijelzőn), a források végtelenített futószalagon, görgetésre beúszó blokkok (`IntersectionObserver`), újabb böngészőkben görgetéshez kötött ráközelítés a képeken és haladásjelző csík felül (`animation-timeline`), kurzort követő fény a kártyákon, fénycsík a fő gombokon. Ha a rendszerben be van kapcsolva a „csökkentett mozgás” (`prefers-reduced-motion`), minden animáció kikapcsol; JavaScript nélkül minden tartalom látszik.
- **Képernyőképek** (`site/screens/`): a valódi felület, bemutató adatokkal; nézetenként sötét és világos téma (a látogató beállítása szerint jelenik meg, `<picture>` + `prefers-color-scheme`), 1× és 2× felbontású WebP (`srcset`), lustán töltve. Kattintásra (vagy Enterre) nagyítóban nyílnak meg, nyilakkal lapozhatók, Esc zárja. A nagyító gombjainak ikonjai SVG-k (nem szövegkarakterek), így minden betűtípussal pontosan középen állnak.
- A képek újragenerálása, ha a felület változik:
  1. `cargo test --manifest-path src-tauri/Cargo.toml dump_demo -- --ignored` – a bemutató tételekből a valódi adószámítással elkészíti a `brand/app-screens.json`-t (`src-tauri/src/demo_dump.rs`);
  2. `npm run dev`, majd a <http://localhost:1420/brand/app-screens.html> oldal a valódi appot mutatja ezekkel az adatokkal (a `brand/app-screens.ts` a Tauri hívásokat a JSON-ből válaszolja meg);
  3. `npm i --no-save puppeteer-core && node brand/app-screens.mjs` – végigkattintja a nézeteket a Windowsra telepített Edge-dzsel, és a `site/screens/` mappába menti a képeket (egy nézet: `node brand/app-screens.mjs tervezo`).

## Keresőoptimalizálás (SEO, GEO, AEO)

- **Cím és leírás:** a `<title>` és a meta description a fő keresett kifejezésekkel („kriptó adó kalkulátor”, „SZJA”, „NAV bevallás”); a H1 felső sora ugyanezt tartalmazza, a szlogen megmarad.
- **Kanonikus URL és megosztás:** `canonical`, Open Graph (`og:url`, `og:locale` = `hu_HU`, 1200×630-as `og-image.png`) és Twitter-kártya. A megosztási kép forrása `brand/og-card.html`; újragenerálás Edge-dzsel a fájl elején leírt paranccsal.
- **Strukturált adat (JSON-LD):** `WebSite`, `SoftwareApplication` (ingyenes, Windows/Linux, `FinanceApplication`, letöltési link) és `FAQPage`. A `FAQPage` szövege pontosan a látható GYIK; ha a GYIK változik, a JSON-LD-t is frissíteni kell (a Google eltérés esetén figyelmen kívül hagyja).
- **AI-válaszmotorok (GEO/AEO):** a GYIK a valódi keresési kérdésekre (mennyi a kriptóadó, adóköteles-e a kripto–kripto csere, mikor kell bevallani, adókiegyenlítés, kisértékű ügylet) ad rövid, tényszerű, törvényhivatkozásos választ. A `site/llms.txt` gépi olvasásra szánt összefoglaló a fő tényekkel és linkekkel. Az oldalon szerepel a frissítés dátuma (`<time>`), a JSON-LD-ben `dateModified`.
- **Sitemap:** `site/sitemap.xml`. Projektoldalon (`/kriptomerleg/`) saját `robots.txt` nem lehet (az csak a domain gyökerében működik), ezért a sitemapet kézzel kell beküldeni:
  1. [Google Search Console](https://search.google.com/search-console) → *Tulajdon hozzáadása* → *URL-előtag*: `https://adamgyongyosi.github.io/kriptomerleg/` → ellenőrzés *HTML-címkével*: a kapott `<meta name="google-site-verification" …>` sort tedd a `site/index.html` `<head>`-jébe, pushold, majd *Ellenőrzés*.
  2. *Webhelytérképek* → `sitemap.xml` → *Beküldés*. Az *URL-vizsgálat* → *Indexelés kérése* gyorsítja az első indexelést.
  3. [Bing Webmaster Tools](https://www.bing.com/webmasters) → *Importálás a Google Search Console-ból* (a Bing adja a ChatGPT keresés találatainak egy részét is).
- Ellenőrzés: [Rich Results Test](https://search.google.com/test/rich-results) és [Schema Markup Validator](https://validator.schema.org/) az oldal URL-jével.

Arculat (logó, színek, hangnem): [`brand/README.md`](../brand/README.md).
