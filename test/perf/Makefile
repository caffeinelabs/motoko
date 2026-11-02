RUNFLAGS = -p
export EXTRA_MOC_ARGS = --enhanced-orthogonal-persistence

all:
	run-test $(RUNFLAGS) *.mo

accept:
	run-test $(RUNFLAGS) -a *.mo

clean:
	rm -rf _out

include ../*.mk
