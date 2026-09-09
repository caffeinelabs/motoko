#!/usr/bin/env bash

set -e
set -o pipefail

# workaround until https://github.com/Lelio-Brun/Obelisk/issues/15
TEMP_FILE=$(mktemp $(basename $1).XXX)
cp $1 ${TEMP_FILE}
sed -i 's/\[@recover\.cost.*\]//' ${TEMP_FILE}

obelisk -i ${TEMP_FILE} |
# Insert new line after def
sed -e 's/::= /&\n    /' |
# Transform
sed -f $(dirname "${BASH_SOURCE[0]}")/grammar.sed |
# Remove line breaks
sed  -e ':a' -e 'N' -e '$!ba' -e 's/\n\ \ \ \ \ \ */ /g' |
# Scrub grammar-mode parameters that were split across a line break above
sed -e 's/(B, R, L)//g' -e 's/(<ob>, <ob>, <exp_cont>)//g' -e 's/(<bl>, <bl>, <exp_cont_tight>)//g' -e 's/(<bl>, R, L)//g' -e 's/(R, R, L)//g' -e 's/(R, L)//g' -e 's/(<ob>, <exp_cont>)//g' -e 's/(<bl>, <exp_cont_tight>)//g' -e 's/(R)//g'
