import { check, Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { inDesktopApp } from "./api";

export const RELEASES_URL = "https://github.com/adamgyongyosi/kriptomerleg/releases/latest";

/**
 * Új verzió keresése a GitHub Release `latest.json` fájlja alapján. Hálózati hiba vagy hiányzó
 * fájl esetén (pl. aláíratlan build) `null`, ha `quiet`; különben a hibát továbbadja.
 */
export async function findUpdate(quiet = true): Promise<Update | null> {
  if (!inDesktopApp()) return null;
  try {
    return await check({ timeout: 15000 });
  } catch (e) {
    if (quiet) return null;
    throw new Error(`Nem sikerült frissítést keresni: ${e}`);
  }
}

/** Letöltés, telepítés és újraindítás. Linuxon csak az AppImage frissíthető így. */
export async function installUpdate(update: Update, onProgress?: (percent: number | null) => void): Promise<void> {
  let total = 0;
  let done = 0;
  try {
    await update.downloadAndInstall((ev) => {
      if (ev.event === "Started") total = ev.data.contentLength ?? 0;
      if (ev.event === "Progress") {
        done += ev.data.chunkLength;
        onProgress?.(total ? Math.round((done / total) * 100) : null);
      }
    });
  } catch (e) {
    // Régi (2026. október 31. előtti) példány a lecserélt aláírókulccsal: csak kézi letöltéssel frissíthető.
    if (/signature|aláírás/i.test(String(e))) {
      throw new Error(
        "Ez a példány még a régi aláírókulccsal készült, ezért az új kiadást nem tudja ellenőrizni. Töltsd le egyszer kézzel a legújabbat a kiadások oldaláról; onnantól a frissítés automatikus.",
      );
    }
    throw new Error(
      `A frissítés nem telepíthető automatikusan (${e}). Töltsd le kézzel a kiadások oldaláról (.deb csomagnál ez a megoldás).`,
    );
  }
  await relaunch();
}
