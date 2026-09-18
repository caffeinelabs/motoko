#!/usr/bin/env perl
# Extract one changelog section for a release body.
#
# Usage: extract-changelog.pl <file> <mode> [version]
#   mode = version     -> the `## <version> (YYYY-MM-DD)` section
#   mode = unreleased  -> everything above the first released
#                         `## <version> (YYYY-MM-DD)` section
#
# The extracted body (without the heading) goes to stdout; diagnostics go to
# stderr:
#   `warning: <msg>`  -> no section to extract: body is empty, exit 0
#   `error: <msg>`    -> malformed changelog / bad usage, exit 1
#
# Exit status separates the two: 0 means a section was extracted (or the
# changelog legitimately has none), non-zero means the changelog is wrong. The
# caller reports `extractable` from whether the body is non-empty.

use strict;
use warnings;

sub warning {
    print STDERR "warning: $_[0]\n";
    exit 0;
}

sub error {
    print STDERR "error: $_[0]\n";
    exit 1;
}

# The heading itself is always dropped: the body is the entries only. Leading
# blank lines are not entries, so they go too; everything else is kept as is.
sub body_of {
    my ($body) = @_;
    $body = "" unless defined $body;
    $body =~ s/^\n+//;
    return $body;
}

# A released section heading, e.g. `## 1.16.1 (2026-09-16)`. This is the marker
# that ends the unreleased entries, whether or not they have a heading of their
# own (`## Next`).
my $released_heading = qr/^## [0-9]+\.[0-9]+\.[0-9]+ \(\d\d\d\d-\d\d-\d\d\)/m;

my ($file, $mode, $version) = @ARGV;
$mode = 'version' if !defined $mode || $mode !~ /\S/;
$mode =~ s/^\s+|\s+$//g;
$version =~ s/^\s+|\s+$//g if defined $version;

error("no changelog file given") unless defined $file && length $file;

open my $fh, '<', $file or error("cannot read $file: $!");
local $/;
my $changelog = <$fh>;
close $fh;

if ($mode eq 'version') {
    error("mode 'version' requires a version") unless defined $version && length $version;
    # Any `## <version> (date)` section, not only the first one: a changelog
    # with unreleased entries above the latest release must still resolve a
    # released version.
    $changelog =~ /^## \Q$version\E \(\d\d\d\d-\d\d-\d\d\)\n+(.*?)^##/sm
        or warning("Changelog does not look right for this version: no '$version' section found (expected a '## $version (YYYY-MM-DD)' heading)");
    print body_of($1);
    exit 0;
}

if ($mode eq 'unreleased') {
    # Everything above the first released section: the heading of the
    # unreleased entries (`## Next`, or whatever it is called) is optional, so
    # skip the leading heading block -- the titles and blank lines before the
    # first entry -- rather than look for one. Only that block, so a `#` line
    # inside the entries themselves is left alone.
    my $head = $changelog;
    $head =~ s/$released_heading.*//s;
    $head =~ s/\A(?:[ \t]*\#[^\n]*\n|[ \t]*\n)*//;
    my $body = body_of($head);
    warning("Changelog has no unreleased entries above the latest release") unless $body =~ /\S/;
    print $body;
    exit 0;
}

error("unknown mode '$mode' (expected 'version' or 'unreleased')");
