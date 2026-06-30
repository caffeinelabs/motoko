# TODO: Include ld when it supports Wasm64 
QUICK_DIRS = cmp fail idl mo-idl repl run
OTHER_DIRS = bench perf run-deser run-drun trap
TEST_DIRS = $(QUICK_DIRS) $(OTHER_DIRS)
SOURCE_PATHS ?= ../src # will be overridden when building coverage report with nix

MAKE_PAR := $(MAKE) --no-print-directory --load-average -j $(shell getconf _NPROCESSORS_ONLN) --keep-going

all:
	$(foreach test_dir,$(TEST_DIRS),make -C $(test_dir) &&) true

quick:
	$(foreach test_dir,$(QUICK_DIRS),$(MAKE_PAR) -C $(test_dir) quick &&) true

# Most-complete *parallel* per-file target each dir exposes, for coverage runs:
#  - run-drun's `quick` (from quick.mk) silently omits *.drun, so it needs its
#    own `parallel` target to keep the actor/upgrade/migration coverage;
#  - cmp has no per-file rule for *.cmp, so it falls back to the (tiny) serial
#    `all`;
#  - every other dir's `quick` covers its whole glob, file by file.
cov_target = $(if $(filter run-drun,$1),parallel,$(if $(filter cmp,$1),all,quick))

# Run under `make -jN`: GNU make's jobserver bounds the *total* number of
# concurrent tests (across dirs and across files within a dir) at N, so there
# is no oversubscription however many dirs run at once. Instrumented binaries
# append a unique suffix per process to $BISECT_FILE, so the whole suite can
# write into a single _coverage dir concurrently.
# --all-modes compiles every unmarked test in BOTH persistence modes (classical
# *and* enhanced). Without it, the harness default (--legacy-persistence) runs
# single-pass classical, leaving compile_enhanced.ml largely uncovered. The
# run-test recipes append $(EXTRA_RUNFLAGS) only for .mo/.drun (moc codegen);
# the ~2x comp work is what the parallel run above is here to absorb.
coverage: export BISECT_FILE := $(CURDIR)/_coverage/bisect
coverage: export SKIP_RUNNING := yes
coverage: export SKIP_VALIDATE := yes
coverage: export EXTRA_RUNFLAGS := --all-modes
coverage: $(TEST_DIRS:%=cov-%)
	bisect-ppx-report html --coverage-path=_coverage $(foreach dir,$(SOURCE_PATHS),--source-path=$(dir) --source-path=$(dir)/_build/default/) -o coverage
	rm -rf _coverage

.PHONY: coverage coverage-prep $(TEST_DIRS:%=cov-%)
coverage-prep:
	rm -rf _coverage coverage
	mkdir _coverage

$(TEST_DIRS:%=cov-%): cov-%: | coverage-prep
	$(MAKE) -C $* $(call cov_target,$*)

parallel:
	$(foreach test_dir,$(TEST_DIRS),$(MAKE_PAR) -C $(test_dir) quick &&) true

accept:
	$(foreach test_dir,$(TEST_DIRS),make -C $(test_dir) accept &&) true

clean:
	$(foreach test_dir,$(TEST_DIRS),make -C $(test_dir) clean &&) true
