# Änderungen

## v0.2.0 (unveröffentlicht)

- Referenz für `faker.rows`, `faker.fill` und `faker.value` (ADR 0014 von Runnel, E15b): gesetzte Felder bzw.
  Ergebnis, weitergegebene Datensätze, Fehler und Grenzen. Runnel zeigt sie in der Hilfe des Editors; ein Test prüft
  sie mit `check_references` des SDK.
- Rust-SDK `v0.2.0`.

## v0.1.0 (2026-11-25)

- Arten `company`, `iban` (Land wählbar, gültige Prüfziffer), `bic`, `vatId` (DE, AT, GB mit Prüfziffer), `lorem`
  (Wörter oder Sätze); `nullRate` für alle Arten.
- Action `faker.value` für einen einzelnen Wert.
- Rust-SDK `v0.1.0` per Git-Tag statt `../sdk-rust`.
- Release-Archive für Windows und Linux mit `THIRD_PARTY_NOTICES` und `SHA256SUMS` (Workflow `release.yml`).
- `faker.rows`: Datensätze mit Personen, Adressen, Kontakten, Zahlen, Datumswerten, `pick` und `pattern`, Locales
  `de` und `en`, reproduzierbar mit `seed`, Prüfung mit `transform.init`, höchstens 10.000.000 Datensätze.
- `faker.fill`: setzt oder ersetzt Felder eingehender Datensätze; `consistent: true` ergibt für denselben Wert
  (mit Feld und `seed`) denselben Ersatz, zum Pseudonymisieren über Tabellen hinweg.
- Befund zu `de_de` in fake-rs 5.1.0 und eigene Listen für deutsche Vornamen, Straßen, Städte mit Postleitzahlen und
  Telefonnummern (README).
