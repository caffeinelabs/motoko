all: overview-slides.html html

RELEASE=0.14.0
SHELL=bash
OUT=md

overview-slides.html : overview-slides.md
	pandoc \
	  -t revealjs -s -o overview-slides.html overview-slides.md \
	  -V theme=moon \
	  --strip-comments \
	  -V revealjs-url=https://cdn.jsdelivr.net/npm/reveal.js@4.1.0 \
	  #--standalone \
	  # -V width="\"100%\"" \
	  # -V height="\"100%\"" \
	  # -V margin=0 \
	  # # -V minScale=2 \
	  # # -V maxScale=2

html:
        # Lightweight unstyled snapshot; does not process remark plugins or file embeds.
	rm -rf html
	mkdir -p html
	find md -name "*.md" | while read f; do pandoc -f gfm --toc -t html -s --metadata title="$$f" -o html/$$(basename $$f .md).html $$f; done

preview: _site-install
	npm --prefix site run dev

build: _site-install
	npm --prefix site run build

serve: build
	npm --prefix site run preview

_site-install:
	npm --prefix site install

.PHONY: html preview build serve _site-install
