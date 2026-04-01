.PHONY: %.accept
THIS_DIR := $(abspath $(dir $(lastword $(MAKEFILE_LIST))))
REPO_ROOT := $(shell git rev-parse --show-toplevel)

%.accept: %.mo
	../run.sh -a $(RUNFLAGS) $<
	@ git -C $(REPO_ROOT) status -s $(CURDIR)/$< $(THIS_DIR)/ok
	@ echo STAGING: $< $(wildcard ok/$(basename $<).*)
	@ if [ -n "$(wildcard $(THIS_DIR)/ok/$(basename $<).*)" ] \
	; then git add --ignore-errors --update $< $(wildcard $(THIS_DIR)/ok/$(basename $<).*) \
	; fi
	@ git -C $(REPO_ROOT) status -s $(CURDIR)/$< $(CURDIR)/ok/$(basename $<).*
