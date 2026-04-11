{ pkgs, base-doc, docs, coverage }:
pkgs.runCommand "report-site"
{
  buildInputs = [ pkgs.tree ];
} ''
  mkdir -p $out
  ln -s ${base-doc} $out/base-doc
  ln -s ${docs} $out/docs
  ln -s ${coverage} $out/coverage
  cd $out;
  # generate a simple index.html, listing the entry points
  ( echo docs/overview-slides.html;
    echo docs/html/motoko.html;
    echo base-doc/
    echo coverage/ ) | \
    tree -H . -l --fromfile -T "Motoko build reports" > index.html
''
