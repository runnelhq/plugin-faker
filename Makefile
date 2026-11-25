# Runnel-Plugin "faker". Ziele: make build | test | lint | fmt | bench | deny | runnel-test
# Runnel für die Tests gegen den Host, z. B. RUNNEL_BIN=../runnel/bin/runnel
RUNNEL_BIN     ?=
WINDOWS_TARGET := x86_64-pc-windows-msvc

.PHONY: build test lint fmt bench deny golden runnel-test clean

## build: Plugin für dieses System nach target/release/ bauen
build:
	cargo build --release

## test: alle Tests (Tests gegen Runnel nur mit RUNNEL_BIN)
test:
	RUNNEL_BIN=$(RUNNEL_BIN) cargo test --all-targets

## lint: Formatierung und Clippy für Linux- und Windows-Code (rustup target add $(WINDOWS_TARGET))
lint:
	cargo fmt --check
	cargo clippy --all-targets -- -D warnings
	cargo clippy --all-targets --target $(WINDOWS_TARGET) -- -D warnings

## fmt: Quelltext formatieren
fmt:
	cargo fmt

## bench: Durchsatz von faker.rows mit fünf Feldern (ohne Runnel)
bench:
	cargo test --release --test bench -- --ignored --nocapture

## deny: Lizenzen, Schwachstellen und Quellen der Abhängigkeiten (cargo-deny)
deny:
	cargo deny check

## golden: erwartete Datensätze (testdata/rows-*.json) neu schreiben; danach den Diff prüfen
golden:
	UPDATE=1 cargo test --test rows reproducible

## runnel-test: nur die Tests gegen Runnel (RUNNEL_BIN nötig)
runnel-test:
	@test -n "$(RUNNEL_BIN)" || (echo "set RUNNEL_BIN to the runnel program" && exit 2)
	RUNNEL_BIN=$(abspath $(RUNNEL_BIN)) cargo test --test with_runnel -- --nocapture

clean:
	cargo clean
	rm -rf dist
