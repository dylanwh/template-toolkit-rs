#!/bin/sh
# Fetch the external test corpora into corpora/ (gitignored):
#
#   corpora/bugzilla/harmony      Bugzilla harmony (BMO), pinned
#   corpora/bugzilla/bugzilla     Bugzilla 5.0.6
#   corpora/Template-Toolkit-3.009  Perl TT from CPAN, the parity tests' oracle
#
# The pins matter: tests/bugzilla/fixtures (the plain-TT field-descs
# translation, the version in the Perl mock) match these exact trees.
# Re-running is cheap; whatever is already in place is checked, not fetched.
set -eu

HARMONY_SHA=fcd8aa84faf4efeb25f1d1628e7ef04782ddecdc
BUGZILLA_SHA=e59bafbe0d6bade76b620f58eb91eab0409bb198 # release-5.0.6
TT_URL=https://cpan.metacpan.org/authors/id/A/AT/ATOOMIC/Template-Toolkit-3.009.tar.gz
TT_SHA256=d6ad23bbf637a59b5dfd1ac006460dfcb185982e4852cde77150fbd085f1f5b6

root=$(cd "$(dirname "$0")/.." && pwd)
corpora=$root/corpora
mkdir -p "$corpora/bugzilla"

# checkout DIR URL SHA: a shallow clone of one commit.
checkout() {
    dir=$1 url=$2 sha=$3
    if [ -d "$dir/.git" ] && [ "$(git -C "$dir" rev-parse HEAD)" = "$sha" ]; then
        echo "$dir: at $sha"
        return
    fi
    rm -rf "$dir"
    git init -q "$dir"
    git -C "$dir" fetch -q --depth 1 "$url" "$sha"
    git -C "$dir" -c advice.detachedHead=false checkout -q FETCH_HEAD
    echo "$dir: fetched $sha"
}

checkout "$corpora/bugzilla/harmony" https://github.com/bugzilla/harmony.git "$HARMONY_SHA"
checkout "$corpora/bugzilla/bugzilla" https://github.com/bugzilla/bugzilla.git "$BUGZILLA_SHA"

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
