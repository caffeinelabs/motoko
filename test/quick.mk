# Note: this rule collection is included from one level deeper

RUN_TEST := $(shell command -v run-test)

TO-TEST = \
  $(patsubst %.mo,_out/%_done,$(wildcard *.mo)) \
  $(patsubst %.sh,_out/%_done,$(wildcard *.sh)) \
  $(patsubst %.wat,_out/%_done,$(wildcard *.wat)) \
  $(patsubst %.did,_out/%_done,$(wildcard *.did)) \


.PHONY: quick

quick: $(TO-TEST)

_out:
	@ mkdir -p $@

# run single test, e.g. make _out/AST-56_done
# _done, not .done, because run-test likes to clean $base.*
_out/%_done: %.mo $(wildcard ../../src/moc) $(RUN_TEST) | _out
	@+ chronic run-test $(RUNFLAGS) $<
	@+ touch $@
_out/%_done: %.sh $(wildcard ../../src/moc) $(RUN_TEST) | _out
	@+ chronic run-test $(RUNFLAGS) $<
	@+ touch $@
_out/%_done: %.wat $(wildcard ../../src/moc) $(RUN_TEST) | _out
	@+ chronic run-test $(RUNFLAGS) $<
	@+ touch $@
_out/%_done: %.did $(wildcard ../../src/didc) $(RUN_TEST) | _out
	@+ chronic run-test $(RUNFLAGS) $<
	@+ touch $@
