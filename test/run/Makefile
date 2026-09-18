RUNFLAGS =

JOBS ?= $(if $(filter-out 0,$(NIX_BUILD_CORES)),$(NIX_BUILD_CORES),$(shell getconf _NPROCESSORS_ONLN))
TEST_RUNNER ?= test-runner

all:
	$(TEST_RUNNER) -b -j $(JOBS) $(CURDIR)

accept:
	$(TEST_RUNNER) -b -j $(JOBS) -a $(CURDIR)

clean:
	rm -rf _out

include ../*.mk
