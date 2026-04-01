.PHONY: %.accept
THIS_DIR := $(abspath $(dir $(lastword $(MAKEFILE_LIST))))
REPO_ROOT := $(shell git rev-parse --show-toplevel)

%.accept: %.mo
	../run.sh -a $(RUNFLAGS) $<
	@ git -C $(REPO_ROOT) status -s $(CURDIR)/$< $(CURDIR)/ok
	@ echo STAGING: $< $(wildcard ok/$(basename $<).*)
	@ set -- $(CURDIR)/ok/$(basename $<).*; \
	  [ -e "$$1" ] && git add --ignore-errors $(CURDIR)/$< "$$@" || true
	@ git -C $(REPO_ROOT) status -s $(CURDIR)/$< $(CURDIR)/ok/$(basename $<).*
