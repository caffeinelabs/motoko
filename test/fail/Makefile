RUNFLAGS = -t

all:
	run-test $(RUNFLAGS) *.mo

accept:
	run-test $(RUNFLAGS) -a *.mo

accept-par:
	find . -maxdepth 1 -name '*.mo' -print0 | xargs -0 -P4 -I{} run-test $(RUNFLAGS) -a {}

clean:
	rm -rf _out

include ../*.mk
