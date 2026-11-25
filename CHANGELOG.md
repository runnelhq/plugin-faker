# Änderungen

## Unveröffentlicht

- `faker.rows`: Datensätze mit Personen, Adressen, Kontakten, Zahlen, Datumswerten, `pick` und `pattern`, Locales
  `de` und `en`, reproduzierbar mit `seed`, Prüfung mit `transform.init`, höchstens 10.000.000 Datensätze.
- `faker.fill`: setzt oder ersetzt Felder eingehender Datensätze; `consistent: true` ergibt für denselben Wert
  (mit Feld und `seed`) denselben Ersatz, zum Pseudonymisieren über Tabellen hinweg.
- Befund zu `de_de` in fake-rs 5.1.0 und eigene Listen für deutsche Vornamen, Straßen, Städte mit Postleitzahlen und
  Telefonnummern (README).
