#!/bin/sh
# Fetch Perl's Template Toolkit 3.009 from CPAN into corpora/ (gitignored).
# The ported test suite reads upstream's test cases (t/*.t) and fixtures
# (t/test) from it, as they aren't ours to ship, and tpage-compare runs
# its tpage.  Re-running checks what is there instead of fetching again.
#
# The Bugzilla template corpus is a repository of its own,
# template-toolkit-rs-bugzilla, as it is under the MPL.
set -eu

TT_URL=https://cpan.metacpan.org/authors/id/A/AT/ATOOMIC/Template-Toolkit-3.009.tar.gz
TT_SHA256=d6ad23bbf637a59b5dfd1ac006460dfcb185982e4852cde77150fbd085f1f5b6

root=$(cd "$(dirname "$0")/.." && pwd)
corpora=$root/corpora
mkdir -p "$corpora"

tt=$corpora/Template-Toolkit-3.009
if [ -f "$tt/lib/Template.pm" ]; then
    echo "$tt: present"
else
    tarball=$corpora/Template-Toolkit-3.009.tar.gz
    curl -fsSL -o "$tarball" "$TT_URL"
    echo "$TT_SHA256  $tarball" | sha256sum -c --quiet -
    rm -rf "$tt"
    tar -xzf "$tarball" -C "$corpora"
    rm "$tarball"
    echo "$tt: fetched"
fi
