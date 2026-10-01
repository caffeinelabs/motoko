RUNFLAGS = -d

all:
	test-runner -b $(CURDIR)

accept:
	test-runner -b -a $(CURDIR)

parallel: $(patsubst %.mo,_out/%_done,$(wildcard *.mo)) $(patsubst %.drun,_out/%_done,$(wildcard *.drun))

clean:
	rm -rf _out

include ../*.mk

# Override the .mo rule from quick.mk to use our RUNFLAGS
_out/%_done: %.mo $(wildcard ../../src/moc) $(RUN_TEST) | _out
	@+ chronic run-test $(RUNFLAGS) $<
	@+ touch $@

# Add .drun rule (not in quick.mk)
_out/%_done: %.drun $(wildcard ../../src/moc) $(RUN_TEST) | _out
	@+ chronic run-test $(RUNFLAGS) $<
	@+ touch $@
