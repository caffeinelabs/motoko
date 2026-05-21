set -v

if [ -z "$MOTOKO_CORE" ]
then
  echo "\$MOTOKO_CORE not set. Are you running this in a nix-shell?"
  exit 1
fi

for file in Card-v*.mo
do
  echo "compiling $file" ...
  moc --package core "$MOTOKO_CORE" --stable-types $file
  moc --package core "$MOTOKO_CORE" --idl $file
done

moc --stable-compatible Card-v0.most Card-v1a.most && echo "ok"
moc --stable-compatible Card-v1a.most Card-v1b.most && echo "ok"
moc --stable-compatible Card-v0.most Card-v1c.most && echo "ok"
moc --stable-compatible Card-v1c.most Card-v1d.most && echo "ok"
