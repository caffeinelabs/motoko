.PHONY: %.only

%.only: %.mo
	run-test $(RUNFLAGS) $<
