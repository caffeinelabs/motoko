# Makefile for motoko  demos

# This works with bash

# we need to specify the output if there is more than one .mo file
OUT=main.wasm
LIB=list.mo types.mo server.mo client.mo

SAMPLES:= test

all:
	rm -f main.txt
	moc -r -t -v main.mo >main.txt 2>&1
	cat main.txt

clean:
	rm -f $(OUT) main.txt
