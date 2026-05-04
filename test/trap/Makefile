RUNFLAGS =

all:
	run-test $(RUNFLAGS) *.mo

accept:
	run-test $(RUNFLAGS) -a *.mo

clean:
	rm -rf _out

include ../*.mk

.PHONY: check _out/%.checked
check: $(patsubst %.mo,_out/%.checked,$(wildcard *.mo))

## for each %.mo test there should be 4 ok/%.*.ok files with defined contents
_out/%.checked: %.mo
	@ (cat $(patsubst %.mo,ok/%.run*.ok,$^) \
	| grep -e "execution error, arithmetic overflow" \
	       -e "execution error, numeric overflow" \
	       -e "execution error, value out of bounds" \
	| wc -l | grep 3 > /dev/null) \
	|| (cat $(patsubst %.mo,ok/%.run*.ok,$^); ls $(patsubst %.mo,ok/%.run*.ok,$^); false)
	@ (cat $(patsubst %.mo,ok/%.wasm-run.ok,$^) \
	| grep -e "runtime trap: unreachable" \
	       -e "runtime trap: integer overflow" \
	| wc -l | grep 1 > /dev/null) \
	|| (cat $(patsubst %.mo,ok/%.wasm-run.ok,$^); ls $(patsubst %.mo,ok/%.wasm-run.ok,$^); false)
	@ (ls $(patsubst %.mo,ok/%.*.ok,$^) | wc -l | grep 4 > /dev/null) \
	|| (ls $(patsubst %.mo,ok/%.*.ok,$^); false)
