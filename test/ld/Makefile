RUNFLAGS =

all:
	run-test $(RUNFLAGS) *.wat

accept:
	run-test $(RUNFLAGS) -a *.wat

clean:
	rm -rf _out

WASM_CLANG?=clang-19
WASM_LD?=wasm-ld-19
MO_LD?=../../src/mo-ld

_out/%.lib.o: %.c | _out
	$(WASM_CLANG) --compile -fpic --target=wasm64 --optimize=3 \
		-fno-builtin -ffreestanding \
		$< --output $@

_out/%.lib.wasm: _out/%.lib.o | _out
	$(WASM_LD) -mwasm64 --import-memory --shared --no-entry --gc-sections \
		--import-undefined --export=__wasm_call_ctors --experimental-pic \
		$< -o $@

_out/%.base.wasm: %.wat | _out
	wat2wasm --enable-memory64 --debug-names $< -o $@

_out/%.linked.wasm: _out/%.base.wasm _out/%.lib.wasm
	$(MO_LD) -b _out/$*.base.wasm -l _out/$*.lib.wasm -o _out/$*.linked.wasm

_out/%.wat: _out/%.wasm
	wasm2wat --enable-memory64 $< -o $@

_out/%.valid: _out/%.wasm
	wasm-validate --enable-memory64 $< > $@ 2>&1 || true


include ../*.mk
