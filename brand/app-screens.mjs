// A weboldal képernyőképei (site/screens/*.webp) a valódi felületről, bemutató adatokkal.
// Használat (a repó gyökeréből, futó `npm run dev` mellett):
//   npm i --no-save puppeteer-core && node brand/app-screens.mjs [nézet]
// A Windowsra telepített Edge-et használja (EDGE környezeti változóval felülírható).
// Nézetenként sötét és világos téma, 2× (….webp) és 1× (…-sm.webp) felbontásban.
import puppeteer from "puppeteer-core";
import fs from "node:fs";

const OUT = "site/screens";
const URL = "http://localhost:1420/brand/app-screens.html";
const only = process.argv[2];
fs.mkdirSync(OUT, { recursive: true });

const browser = await puppeteer.launch({
  executablePath: process.env.EDGE ?? "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe",
  headless: true,
  args: ["--hide-scrollbars", "--lang=hu-HU"],
});
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function clickText(page, text, sel = "button") {
  const ok = await page.evaluate(
    (text, sel) => {
      const el = [...document.querySelectorAll(sel)].find((b) => b.textContent.trim().startsWith(text));
      if (!el) return false;
      el.click();
      return true;
    },
    text,
    sel,
  );
  if (!ok) throw new Error(`nincs: ${text}`);
  await sleep(400);
}

const scenes = {
  attekintes: async () => {},
  tetelek: async (p) => {
    await clickText(p, "Tételek");
    await clickText(p, "Ellenőrzés");
  },
  nyomkovetes: async (p) => {
    await clickText(p, "Tételek");
    await p.evaluate(() => document.querySelectorAll('button[title^="Eredeti sorok"]')[2].click());
    await sleep(400);
  },
  api: async (p) => {
    await clickText(p, "Importálás");
    await clickText(p, "Lekérés közvetlenül");
    await clickText(p, "Binance");
    await p.type("#api-key", "demo-csak-olvaso-kulcs-7f3a9c41e8b2");
    await p.type("#api-secret", "demo-titok-0000000000000000");
    await p.evaluate(() => {
      const i = document.querySelector("#api-since");
      Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value").set.call(i, "2026-01-01");
      i.dispatchEvent(new Event("input", { bubbles: true }));
    });
    await sleep(300);
  },
  elonezet: async (p) => {
    await scenes.api(p);
    await clickText(p, "Lekérés (");
    await sleep(500);
  },
  evek: async (p) => clickText(p, "Évek és adó"),
  bevallas: async (p) => clickText(p, "Bevallási segéd"),
  tervezo: async (p) => {
    await p.type('input[aria-label="Tervezett kivétel"]', "1500");
    await sleep(900);
    await p.evaluate(() => document.querySelector('input[aria-label="Tervezett kivétel"]').scrollIntoView({ block: "center" }));
    await sleep(300);
  },
  adatok: async (p) => clickText(p, "Adatok és mentés"),
};

for (const [scale, suffix] of [[2, ""], [1, "-sm"]]) for (const theme of ["dark", "light"]) {
  for (const [name, run] of Object.entries(scenes)) {
    if (only && name !== only) continue;
    const page = await browser.newPage();
    await page.setViewport({ width: 1180, height: 760, deviceScaleFactor: scale });
    await page.emulateMediaFeatures([{ name: "prefers-color-scheme", value: theme }]);
    await page.goto(URL, { waitUntil: "networkidle0" });
    await page.waitForSelector("nav button, aside button");
    await sleep(800);
    await run(page);
    if (["attekintes", "api", "tervezo"].includes(name)) await page.evaluate(() => document.activeElement?.blur());
    await page.screenshot({ path: `${OUT}/${name}-${theme}${suffix}.webp`, type: "webp", quality: 82 });
    await page.close();
    console.log(name, theme);
  }
}
await browser.close();
