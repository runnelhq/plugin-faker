# plugin-faker – Testdaten für Runnel

Runnel-Plugin, das Testdaten erzeugt und echte Daten pseudonymisiert: Personen, Adressen, Kontakte, Zahlen,
Datumswerte und eigene Muster, auf Deutsch oder Englisch. Gebaut mit dem [Rust-SDK](https://github.com/runnelhq/sdk-rust) und
[fake-rs](https://github.com/cksac/fake-rs) (Crate `fake`, MIT oder Apache-2.0).

Das Plugin gehört nicht in den Kern von Runnel, weil es in produktiven Umgebungen nicht gebraucht wird.

| | |
|---|---|
| Namensraum | `faker` |
| Bausteine | Transforms `faker.rows` (Eingabe) und `faker.fill`, Action `faker.value` |
| Runnel | ab S50b (Transforms mit `transform.init`) |
| Lizenz | MIT |

## Beispiel

```yaml
kind: Pipeline
apiVersion: runnel/v1
name: customers
transforms:
  - id: fake
    type: faker.rows
    with:
      count: 1000
      locale: de
      seed: 42
      fields:
        - { name: id, kind: integer, min: 1, max: 99999 }
        - { name: salutation, kind: salutation }
        - { name: firstName, kind: firstName }
        - { name: lastName, kind: lastName }
        - { name: email, kind: email, from: "firstName, lastName" }
        - { name: street, kind: street }
        - { name: houseNumber, kind: houseNumber }
        - { name: postcode, kind: postcode }
        - { name: city, kind: city }
        - { name: birthDate, kind: birthDate, minAge: 18, maxAge: 90 }
        - { name: segment, kind: pick, values: [A, B, C], weights: [1, 3, 6] }
        - { name: customerNo, kind: pattern, pattern: "KD-#####" }
  - id: write
    type: csv.output
    with:
      path: out/customers.csv
links:
  - { from: fake, to: write }
```

Der erste Datensatz daraus (`seed: 42`, erzeugt am 2026-11-25; `birthDate` hängt vom Tag ab):

```json
{"id": 8863, "salutation": "Frau", "firstName": "Christa", "lastName": "Schreiber",
 "email": "christa.schreiber@example.net", "street": "Bergstraße", "houseNumber": "39c",
 "postcode": "42353", "city": "Wuppertal", "birthDate": "1953-11-19", "segment": "C", "customerNo": "KD-25768"}
```

## `faker.rows`

| Feld | Bedeutung |
|---|---|
| `count` | Anzahl der Datensätze, höchstens 10.000.000 |
| `locale` | `de` (Standard) oder `en` |
| `seed` | Startwert der Zufallszahlen. Gleicher `seed` und gleiche Konfiguration ergeben dieselben Daten, auf jedem System. Ohne `seed` nennt das Log den verwendeten Wert (`seed 123…`) |
| `fields` | Liste der Felder: `name`, `kind` und die Optionen der Art |

Der Wert eines Feldes hängt nur von `seed`, der Nummer des Datensatzes und dem Namen des Feldes ab. Ein neues Feld
ändert die anderen also nicht.

## `faker.fill`

Setzt oder ersetzt Felder eingehender Datensätze: Ein Feld, das es schon gibt, wird ersetzt (mit dem Typ der Art),
ein neues wird angehängt. Die anderen Felder bleiben, wie sie sind. Optionen wie bei `faker.rows`, ohne `count`.

```yaml
  - id: pseudo
    type: faker.fill
    with:
      consistent: true
      seed: ${PSEUDONYM_SEED}
      fields:
        - { name: firstName, kind: firstName }
        - { name: lastName, kind: lastName }
        - { name: email, kind: email, from: "firstName, lastName" }
        - { name: customerNo, kind: pattern, pattern: "KD-#####" }
```

- **`consistent: true`:** Derselbe Eingabewert ergibt denselben Ersatz – berechnet aus Wert, Name des Feldes und
  `seed`. So passen Schlüssel über Tabellen und Läufe hinweg: `customerNo` „4711“ wird in der Kunden- und in der
  Auftragstabelle zu derselben Nummer, wenn beide Felder gleich heißen. `null` bleibt `null`. Felder, die es in der
  Eingabe nicht gibt, bekommen je Datensatz neue Werte.
- **`seed` ist der Schlüssel:** Wer `seed` kennt, kann Ersatzwerte durch Ausprobieren von Eingaben zurückführen. Den
  Wert deshalb nicht in die Pipeline schreiben, sondern aus einer Variablen nehmen. Ohne `seed` nimmt `consistent` den
  Wert 0; die Prüfung warnt dann. Pseudonymisieren ersetzt nur die genannten Felder; es anonymisiert nicht.
- Ohne `consistent` bekommt jeder Datensatz neue Werte (wie `faker.rows`).
- `faker.fill` erlaubt `copies`.

## `faker.value`

Ein einzelner Wert, z. B. für Tests in Workflows; Optionen wie ein Feld von `faker.rows` (ohne `name`), dazu `locale`
und `seed`. Das Ergebnis steht in `results.<id>.output`.

```yaml
  - id: iban
    type: faker.value
    with:
      kind: iban
      country: AT
```

### Arten (`kind`) und Typen

| Art | Typ | Optionen und Inhalt |
|---|---|---|
| `firstName`, `lastName`, `fullName` | string | Vor- und Nachname einer Person; alle Personenfelder eines Datensatzes gehören zu derselben Person |
| `salutation` | string | `Frau`/`Herr` bzw. `Ms.`/`Mr.`, passend zum Vornamen |
| `birthDate` | date | `minAge` (18), `maxAge` (80); `to`: Stichtag des Alters (Standard heute) |
| `email` | string | `vorname.nachname@example.de`; mit `from: "firstName, lastName"` aus den Werten dieser Felder (Umlaute als `ae`, `oe`, `ue`, `ss`). Nur Domains `example.*` (RFC 2606), keine echten Adressen |
| `phone` | string | deutsche Festnetz- und Mobilnummern bzw. nordamerikanische Nummern aus dem für Filme reservierten Bereich 555-01xx |
| `street`, `houseNumber` | string | `Goethestraße`, `12a` bzw. `Gleichner Valley`, `1393` |
| `postcode`, `city` | string | `de`: echte Städte, die Postleitzahl liegt im Bereich der Stadt; `en`: erfundene Orte, fünfstellige Postleitzahl |
| `country` | string | Länder (`de` vor allem Deutschland und Nachbarländer) |
| `integer` | int | `min` (1), `max` (1000), beide eingeschlossen |
| `decimal` | decimal | `min` (0), `max` (1000), `scale` (2 Nachkommastellen); exakt als Text |
| `float` | float | `min` (0), `max` (1), optional `scale` zum Runden |
| `boolean` | bool | `rate`: Anteil von `true` (0,5) |
| `date` | date | `from` (2020-01-01), `to` (2029-12-31) |
| `datetime` | datetime | `from`, `to` als Datum oder Zeit mit Offset; Ausgabe in UTC (`2024-01-01T16:19:01Z`) |
| `uuid` | string | UUID Version 4 |
| `pick` | string | `values`: Liste; optional `weights` (gleich viele, nicht negativ) |
| `pattern` | string | `pattern`: `#` Ziffer, `?` Großbuchstabe, `*` Ziffer oder Großbuchstabe, `\` nächstes Zeichen unverändert, z. B. `KD-#####` |
| `company` | string | `de`: „Schäfer & Weber KG“, „Busch Elektrotechnik GmbH“, „Schmitt und Partner“ (Nachnamen und Rechtsformen aus fake-rs, deutsche Muster); `en`: „Feil LLC“, „Hills, Kohler and Wolff“ |
| `iban` | string | gültige Prüfziffer (ISO 13616); `country`: `DE` (Standard bei `de`), `AT`, `CH`, `LI`, `LU`, `NL`, `GB` (Standard bei `en`), `BE` |
| `bic` | string | Format einer BIC (Bank, Land, Ort, manchmal Filiale); `country` wie bei `iban`, jedes Länderkürzel |
| `vatId` | string | USt-IdNr. mit gültiger Prüfziffer: `DE` (ISO 7064), `AT` (`ATU…`), `GB` |
| `lorem` | string | `words` (5) Wörter oder `sentences` Sätze Lorem ipsum (fake-rs) |

**Für alle Arten:** `nullRate` – Anteil leerer Werte (`null`) von 0 bis 1, um Wege für fehlende Werte zu testen.

IBANs, BICs und USt-IdNrn. sind formal gültig, gehören aber zu keinem echten Konto oder Unternehmen – außer durch
Zufall. Für Tests, die nichts überweisen oder melden, reicht das; echte Systeme sollte man damit nicht füttern.

### Prüfung (`transform.init`)

`runnel validate` und der Editor kennen die Felder mit ihren Typen (z. B. `birthDate` als `date`, `decimal` als
`decimal`), sodass folgende Schritte geprüft werden können. Fehler stehen am Feld der Konfiguration, z. B.
`fields[1].min: min (5) is greater than max (1)` oder `fields[0].kind: unknown kind "nonsense"`. Werte, die noch
Variablen sind (`${…}`), prüft `init` nicht; steht eine Variable in `kind`, ist das Schema hinter `faker.rows` offen.

### Grenzen und Strom

- `count` höchstens 10.000.000; darüber scheitert die Prüfung und der Lauf mit einer Meldung.
- Erzeugt wird in Batches zu 500 Datensätzen mit den Krediten des Protokolls: Nichts wird ganz im Speicher gehalten,
  ein langsamer Schritt dahinter bremst das Plugin.
- Durchsatz mit fünf Feldern (vier Kerne): ohne Runnel (`make bench`) etwa 700.000 Datensätze/s; in Runnel mit
  `csv.output` 1 Mio. Datensätze in 4 s.

## Daten: fake-rs und eigene Listen

Geprüft mit `fake` 5.1.0, Locale `de_de` (Befund vor dem ersten PR):

| Daten | `de_de` in fake-rs | im Plugin |
|---|---|---|
| Vornamen | deutsch (etwa 120), ohne Geschlecht | eigene Listen mit Geschlecht, damit `salutation` passt (fake-rs ergibt z. B. „Herr Christa Winkler“) |
| Nachnamen | deutsch (etwa 115) | **aus fake-rs** |
| Straße | „Friedrich allee“: Name und Endung mit Leerzeichen | eigene Liste („Goethestraße“, „Lindenweg“) |
| Hausnummer | englische Formate, oft fünfstellig („32846“) | eigene (1–150, manchmal mit Buchstabe) |
| Postleitzahl | teils US-Format („32215-4996“), ohne Bezug zum Ort | eigene: echte Städte mit ihrem Bereich |
| Ort | erfunden („Hohen-Rolf Schmidburg“) | eigene Liste mit 42 Städten |
| Telefon | US-Formate („1-580-167-6578 x3186“) | eigene deutsche Formate |
| Firma | Rechtsformen deutsch, Muster englisch („Schmitt and Horn OHG“) | Nachnamen und Rechtsformen aus fake-rs, deutsche Muster und Branchen aus eigener Liste |
| Land | nur „Deutschland“, „Germany“ | eigene Liste |

Für `en` kommen Nachnamen, Straßenendungen, Orte und Länder aus fake-rs; Vornamen mit Geschlecht und Telefonnummern
aus eigenen Listen. Die eigenen Listen sind selbst erstellt (MIT); Städte und Postleitzahlbereiche sind Fakten. Die
Zufallszahlen kommen nicht aus fake-rs, sondern aus einem eigenen, festgelegten Generator (SplitMix64), damit dieselbe
Konfiguration auf jedem System dieselben Daten ergibt, auch wenn `rand` sich ändert.

## Installieren

1. Archiv für das System aus den [Releases](https://github.com/runnelhq/plugin-faker/releases) laden
   (`runnel-plugin-faker_<version>_windows_amd64.zip` oder `…_linux_amd64.zip`) und mit `SHA256SUMS` prüfen.
2. In den Ordner `plugins/` des Projekts entpacken; es entsteht `plugins/faker/` mit `plugin.yaml`, Programm,
   `LICENSE`, `README.md` und `THIRD_PARTY_NOTICES`.
3. Freigeben:

```bash
runnel plugins trust D:/flows
```

Selbst bauen: `make dist` (Linux) bzw. `scripts/dist.sh x86_64-pc-windows-msvc` unter Windows.

## Entwickeln

Das Rust-SDK kommt als Git-Tag (`v0.1.0`, siehe `Cargo.toml`).

| Zweck | Befehl |
|---|---|
| Tests (jede Art mit `de` und `en`, feste erwartete Datensätze in `testdata/`, Ablehnungen) | `make test` |
| Lint (rustfmt, Clippy für Linux- und Windows-Code) | `make lint` |
| Erwartete Datensätze neu schreiben, wenn sich die Erzeugung bewusst ändert | `make golden`, danach den Diff prüfen |
| Lizenzen und Schwachstellen | `make deny` |
| Benchmark | `make bench` |
| Release-Archiv mit Lizenzhinweisen und `SHA256SUMS` | `make dist`; Release: Tag `v0.1.0` (Workflow `release.yml`, Windows auf eigenem Runner) |
| Test gegen Runnel (Pipeline `faker.rows` → `core.to-result`) | `make runnel-test RUNNEL_BIN=../runnel/bin/runnel` |
