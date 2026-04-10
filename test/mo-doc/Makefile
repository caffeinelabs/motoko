MO_DOC ?= mo-doc

all:
	mkdir -p _out
	$(MO_DOC) --source $(CURDIR) --output $(CURDIR)/_out --format plain 2>_out/warnings.check
	for f in ok/*.md; do diff $$f _out/$$(basename $$f) || exit 1; done
	if [ -s ok/warnings.check.ok ]; then diff ok/warnings.check.ok _out/warnings.check; fi

accept:
	rm -rf _out/* ok/*.md ok/*.ok
	mkdir -p _out ok
	$(MO_DOC) --source $(CURDIR) --output $(CURDIR)/_out --format plain 2>_out/warnings.check
	cp _out/*.md ok/
	if [ -s _out/warnings.check ]; then cp _out/warnings.check ok/warnings.check.ok; fi

clean:
	rm -rf _out
