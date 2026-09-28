#!/usr/bin/env bash
# --check takes several files and checks each on its own: `b.mo` does not see
# `x` from `a.mo`, an error in one file does not hide the others, and `Lib.mo`
# is checked once as an import (and once as an entry), reporting its warning once.
cd check-files
moc --check a.mo b.mo c.mo Lib.mo 2>&1
echo "exit $?"
moc -v --check a.mo b.mo c.mo Lib.mo 2>&1 | grep "^-- Checking"
moc --check a.mo Lib.mo 2>&1
echo "exit $?"
# a library that fails is reported once, not again for each importer
moc --check d.mo e.mo f.mo g.mo 2>&1
echo "exit $?"
moc -v --check d.mo e.mo 2>&1 | grep "^-- Parsing Bad.mo"
# everything else takes a single main file
moc -c a.mo Lib.mo 2>&1
echo "exit $?"
moc -r a.mo Lib.mo 2>&1
echo "exit $?"
moc --check --enhanced-migration migrations a.mo Lib.mo 2>&1
echo "exit $?"
