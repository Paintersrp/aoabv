CARGO ?= cargo
SIMSTEP_SEED ?= ./testdata/seeds/run_seed_wet_equator.json
SIMSTEP_TICKS ?= 120
SIMSTEP_OUT ?= ./target/tmp.ndjson
GOLDEN ?= ./testdata/golden/run_seed_wet_equator.ndjson

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
