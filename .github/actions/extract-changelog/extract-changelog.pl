#!/usr/bin/env perl
# Extract one changelog section for a release body.
#
# Usage: extract-changelog.pl <file> <mode> [version]
#   mode = version     -> the `## <version> (YYYY-MM-DD)` section
#   mode = unreleased  -> the first `## ` section (title-agnostic), unless it
#                         is a dated released section (then there is nothing
#                         unreleased to extract)
#
# The extracted body (without the heading) goes to stdout; diagnostics go to
# stderr:
#   `warning: <msg>`  -> no section to extract: body is empty, exit 0
#   `error: <msg>`    -> malformed changelog / bad usage, exit 1
#
# Exit status separates the two: 0 means a section was extracted (or the
# changelog legitimately has none), non-zero means the changelog is wrong.
# The body length is the signal for "extracted": a non-empty body means a
# section was found, and the caller reports `extractable` from that.

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

# The heading itself is always dropped: the body is the entries only. Nothing
# else is rewritten, so `mode: version` keeps byte-for-byte the body the
# release workflow has always published (leading blank lines are not part of
# the entries and are dropped; the trailing blank line before the next heading
# is kept, as before).
sub body_of {
    my ($body) = @_;
    $body = "" unless defined $body;
    $body =~ s/^\n+//;
    return $body;
}

# A dated released heading, e.g. `## 1.16.1 (2026-09-16)`.
sub is_released_heading {
    return $_[0] =~ /^## [0-9]+\.[0-9]+\.[0-9]+ \(\d\d\d\d-\d\d-\d\d\)$/;
}

my ($file, $mode, $version) = @ARGV;
# An unset/blank mode means the default, same as the action input's default.
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
    # that has unreleased entries above the latest release must still resolve
    # a released version.
    $changelog =~ /^## \Q$version\E \(\d\d\d\d-\d\d-\d\d\)\n+(.*?)^##/sm
        or warning("Changelog does not look right for this version: no '$version' section found (expected a '## $version (YYYY-MM-DD)' heading)");
    print body_of($1);
    exit 0;
}

if ($mode eq 'unreleased') {
    # First `## ` section, whatever it is called, from just after the heading
    # line to the next `## ` heading.
    $changelog =~ /^(## [^\n]*)\n+(.*?)^##/sm
        or warning("Changelog does not look right: no '## ' section found");
    my $heading = $1;
    my $body = $2;
    # Guard: a dated released section as the first section means there is
    # nothing unreleased; never report an already-shipped section as if it
    # were unreleased.
    if (is_released_heading($heading)) {
        warning("Changelog has no unreleased entries: the first section is the released section '$heading'");
    }
    print body_of($body);
    exit 0;
}

error("unknown mode '$mode' (expected 'version' or 'unreleased')");
