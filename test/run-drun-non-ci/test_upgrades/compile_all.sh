#!/bin/bash

for f in `ls versions/*.mo`; do
    echo $f
    moc $f
    # $f contains .mo at the end, so we need to remove it
    fname=$(basename $f .mo)
    mv $fname.wasm wasms/
done
