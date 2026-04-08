MO_DOC ?= mo-doc

all:
	mkdir -p _out
	$(MO_DOC) --source $(CURDIR) --output $(CURDIR)/_out --format plain
	for f in ok/*.md; do diff $$f _out/$$(basename $$f) || exit 1; done

accept:
	mkdir -p _out ok
	$(MO_DOC) --source $(CURDIR) --output $(CURDIR)/_out --format plain
	cp _out/field-docs.md _out/module-comment.md _out/module-comment-before-imports.md ok/

clean:
	rm -rf _out
