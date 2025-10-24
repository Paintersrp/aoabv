CARGO ?= cargo
SIMSTEP_SEED ?= ./testdata/seeds/seed_wet_equator.json
SIMSTEP_TICKS ?= 20
SIMSTEP_OUT ?= ./target/tmp.ndjson
GOLDEN ?= ./testdata/golden/seed_wet_equator.ndjson

.PHONY: build fmt clippy test check simd simstep golden clean

build:
	$(CARGO) build -p simd -p simstep

fmt:
	$(CARGO) fmt

clippy:
	$(CARGO) clippy -D warnings

test:
	$(CARGO) test -p sim_core

check: fmt clippy build test

simd:
	$(CARGO) run -p simd -- --seed-file $(SIMSTEP_SEED) --port 8787

simstep:
	$(CARGO) run -p simstep -- --seed-file $(SIMSTEP_SEED) --ticks $(SIMSTEP_TICKS) --out $(SIMSTEP_OUT)

golden: simstep
	diff -u $(SIMSTEP_OUT) $(GOLDEN)

clean:
	rm -f $(SIMSTEP_OUT)
	$(CARGO) clean

.PHONY: data-validate data-plan data-manifest data-sample

PY ?= python3

data-validate:
	$(PY) tools/datafetch/datafetch.py validate

data-plan:
	$(PY) tools/datafetch/datafetch.py plan --out -

data-manifest:
	$(PY) tools/datafetch/datafetch.py manifest --out data/manifest/data_manifest.json

data-sample:
	$(PY) tools/datafetch/datafetch.py validate --sample
	$(PY) tools/datafetch/datafetch.py plan --sample --out -
	$(PY) tools/datafetch/datafetch.py manifest --sample --out data/manifest/data_manifest.json
	bash scripts/test_data_catalog.sh
