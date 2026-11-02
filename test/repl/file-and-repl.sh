#!/usr/bin/env bash
moc -v -W M0145 -i \
  <(echo "let x = 1; switch (true) {case true ()}") <<__END__
assert (x == 1);
__END__
