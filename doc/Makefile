preview: _site-install
	npm --prefix site run dev

build: _site-install
	npm --prefix site run build

serve: build
	npm --prefix site run preview

_site-install:
	npm --prefix site install

.PHONY: preview build serve _site-install
