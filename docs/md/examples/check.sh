#!/nix/store/i27rhb3nr65rkrwz36bchkwmav6ggsmn-bash-5.3p9/bin/bash

#
# Typechecks all examples in this directory
#

set -e

cd "$(dirname "${BASH_SOURCE[0]}")"

if [ -z "$MOTOKO_CORE" ]
then
  echo "\$MOTOKO_CORE not set. Are you running this in a nix-shell?"
  exit 1
fi

if [ -z "$MOTOKO_BASE" ]
then
  echo "\$MOTOKO_BASE not set. Are you running this in a nix-shell?"
  exit 1
fi


for file in *.mo
do
  echo "$file" ...
  moc --check --package core "$MOTOKO_CORE" --package base "$MOTOKO_BASE" "$file"
done
