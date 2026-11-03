#!/usr/bin/env node
// Változásnapló (CHANGELOG.md, Keep a Changelog formátum) kezelése:
//   node scripts/changelog.mjs release [verzió]  – az [Unreleased] szakaszt a kiadás verziójára és mai dátumára nevezi át
//   node scripts/changelog.mjs notes <verzió>    – kiírja a verzió szakaszát (ha nincs, az [Unreleased]-et) – a CI használja
// A verzió alapból <fő>.<al>.<commitszám>, ahol a commitszám a következő commité (rev-list --count HEAD + 1),
// mert a kiadási szakaszt a pusholás előtti utolsó commitba kell tenni.
import { execSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";

const FILE = new URL("../CHANGELOG.md", import.meta.url);
const REPO = "https://github.com/adamgyongyosi/kriptomerleg";
const [cmd, arg] = process.argv.slice(2);
const text = readFileSync(FILE, "utf8").replace(/\r\n/g, "\n");

/** Egy `## [név]` szakasz törzse a következő `## [`-ig vagy a linkekig, vagy null. */
function section(name) {
  const lines = text.split("\n");
  const start = lines.findIndex((l) => l.startsWith(`## [${name}]`));
  if (start < 0) return null;
  let end = lines.findIndex((l, i) => i > start && (l.startsWith("## [") || /^\[[^\]]+\]: /.test(l)));
  if (end < 0) end = lines.length;
  return lines.slice(start + 1, end).join("\n").trim();
}

if (cmd === "notes") {
  if (!arg) throw new Error("Használat: node scripts/changelog.mjs notes <verzió>");
  process.stdout.write((section(arg) || section("Unreleased") || "") + "\n");
} else if (cmd === "release") {
  const pkg = JSON.parse(readFileSync(new URL("../package.json", import.meta.url), "utf8"));
  const count = Number(execSync("git rev-list --count HEAD").toString().trim()) + 1;
  const version = arg ?? pkg.version.split(".").slice(0, 2).join(".") + "." + count;
  if (!section("Unreleased")) throw new Error("Az [Unreleased] szakasz üres: nincs mit kiadni.");
  if (section(version) !== null) throw new Error(`A ${version} szakasz már létezik.`);
  const prev = text.match(/^\[Unreleased\]: .*\/compare\/v(.+)\.\.\.HEAD$/m)?.[1];
  if (!prev) throw new Error("Nem találom az [Unreleased] összehasonlító linkjét a fájl végén.");
  const d = new Date();
  const date = [d.getFullYear(), d.getMonth() + 1, d.getDate()].map((n) => String(n).padStart(2, "0")).join("-");
  const out = text
    .replace("## [Unreleased]\n", `## [Unreleased]\n\n## [${version}] – ${date}\n`)
    .replace(
      `[Unreleased]: ${REPO}/compare/v${prev}...HEAD`,
      `[Unreleased]: ${REPO}/compare/v${version}...HEAD\n[${version}]: ${REPO}/compare/v${prev}...v${version}`,
    );
  writeFileSync(FILE, out);
  console.log(`CHANGELOG.md: [Unreleased] → [${version}] – ${date}`);
} else {
  console.error("Használat: node scripts/changelog.mjs release [verzió] | notes <verzió>");
  process.exit(1);
}
