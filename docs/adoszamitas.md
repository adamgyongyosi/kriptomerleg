# Az adószámítás szabályai

[← vissza a README-hez](../README.md)

A számítás az Szja tv. 67/C. §-án és a NAV tájékoztatóin alapul:

1. **Éves eredmény** = az év kivételei (bevétel) − az év befizetései (kriptóra fordított kiadás), forintban. Nyereség után 15% SZJA, határidő a következő év május 20.
2. **2022 előtti tételek:** a 15%-os kriptoszabály 2022-től él. A korábban nem adózott ügyletek a NAV szerint a **2022-es eredménybe** számítanak, ezért az app a 2021-es (és korábbi) befizetéseket 2022-höz adja; a 2021-es évnek nincs külön eredménye.
3. **Adókiegyenlítés** (67/C. § (5)–(7)): a veszteség 15%-a érvényesíthető, legfeljebb a tárgyévi és az előző két évben bevallott kriptoadó erejéig, csökkentve az előző két évben már érvényesített kiegyenlítéssel (25SZJA-06 lap 202–209. sor → C lap 75. sor).
4. **Csak a bevallott veszteség vonható le később.** A NAV szerint az adókiegyenlítés csak akkor alkalmazható, ha a veszteséget a keletkezés évének bevallásában (164. sor „a” oszlop) feltüntették. Az *Évek és adó* oldal *Bevallva* oszlopában, illetve az *Áttekintés* figyelmeztetésében évenként jelölhető, melyik évét vallottad be. Ha nincs jelölve:
   - lejárt határidejű évnél (pl. a 2024-es veszteség, határidő 2025. május 20.) az app **bevallatlannak** veszi, és nem vonja le;
   - a még be nem adott évnél (a folyó év) bevallottnak veszi, mert a bevallási segéd szerint fogod kitölteni.

   Ha egy korábbi év vesztesége kimaradt, az adott évi nyomtatványon pótolható (az elévülésig, 5 évig), utána jelöld be. Az *önellenőrzés-segéd* (Bevallási segéd oldal) megmutatja a nyomtatványt (pl. 24SZJA), a 164. sor „a” oszlopába írandó összeget és a várható adócsökkenést. Nyitott kérdés, hogy ez önellenőrzés („O”) vagy helyesbítés („H”): a veszteség a fizetendő adót nem változtatja, ezért az útmutató szerint inkább helyesbítés – ezt a NAV-val (1819) érdemes egyeztetni.
5. **Kisértékű ügylet** (67/C. § (2)): nem kell bevételt megállapítani abból az ügyletből, amelynek bevétele legfeljebb a minimálbér 10%-a, ha aznap nem volt más bevételt hozó ügylet, és az ilyen bevételek éves összege nem haladja meg a minimálbért. A minimálbér az év első napján érvényes havi összeg: 2022: 200 000, 2023: 232 000, 2024: 266 800, 2025: 290 800, 2026: 322 800 Ft (a 2026-os érték sajtóforrásból). Ismeretlen évre az app nem alkalmaz mentességet. Ha az éves összeg túllépi a minimálbért, egyik ilyen bevétel sem mentes. A vételek (kiadások) ettől még teljes összegben számítanak. A beállítás alapból ki van kapcsolva (*Évek és adó* oldal); importált tételnél a napi összevont sor ügyletszáma dönt.
6. **Időkorlát:** a 2025-ös bevallástól (2025. évi LXXXIII. tv.) bármely korábbi év bevallott, még fel nem használt vesztesége figyelembe vehető; a régi kétéves szabály a 2022-től kezdődő évekre ugyanezt adja.

Példa: 2022–2025 között csak befizetések (összesen 18 027 000 Ft veszteség), 2026-ban 24 021 003 Ft nyereség. Bruttó adó 3 603 150 Ft. Ha mind a négy év veszteségét bevallottad, 2 704 050 Ft adókiegyenlítés jár, és 899 100 Ft marad fizetendő; ha egyiket sem, 3 603 150 Ft a fizetendő.
