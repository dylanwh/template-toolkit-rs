#!/bin/sh
# Benchmark the Rust port against Perl's Template Toolkit 3.009 (with its
# XS stash, and pure Perl), on the templates in bench/templates with
# bench/data.json.  Prints a Markdown table.
#
#   scripts/bench.sh            # all of it
#   QUICK=1 scripts/bench.sh    # skip the cold-start runs
#
# Measures, per template:
#   render   compiled template cached, as in a long-running process
#   compile  template text to a runnable template, uncached (Perl: parse
#            and eval the generated code; Rust: parse to a document)
#   cold     a whole `tpage` process (hyperfine), for a one-line template
#            and for buglist with its data inlined as TT literals
# Every template's output is first checked to be identical across all
# three engines.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"
work=target/bench
mkdir -p "$work"

# Perl TT, pure and with the XS stash built.
scripts/fetch-corpora.sh >/dev/null
tt=corpora/Template-Toolkit-3.009
xs=corpora/Template-Toolkit-3.009-xs
if [ ! -f "$xs/blib/arch/auto/Template/Stash/XS/XS.so" ]; then
    echo "building Perl TT with the XS stash..." >&2
    rm -rf "$xs"
    cp -r "$tt" "$xs"
    (cd "$xs" && perl Makefile.PL TT_XS_ENABLE=y TT_ACCEPT=y </dev/null >build.log 2>&1 && make >>build.log 2>&1) ||
        { echo "XS build failed: see $xs/build.log" >&2; exit 1; }
fi
perl_xs="perl -I$xs/blib/lib -I$xs/blib/arch"
perl_pure="perl -I$tt/lib"

cargo build -q --release --example bench -p template-toolkit
cargo build -q --release -p tpage-rs
rust=target/release/examples/bench

names="buglist report mail"
data=bench/data.json
dir=bench/templates

# Same output from all three, or no timings.
for name in $names; do
    $rust output $data $dir "$name" >"$work/rust-$name.json"
    $perl_xs bench/bench.pl output xs $data $dir "$name" 2>/dev/null >"$work/xs-$name.json"
    $perl_pure bench/bench.pl output perl $data $dir "$name" 2>/dev/null >"$work/perl-$name.json"
    for other in xs perl; do
        cmp -s "$work/rust-$name.json" "$work/$other-$name.json" ||
            { echo "$name: Rust and Perl ($other) outputs differ; see $work/" >&2; exit 1; }
    done
done

: >"$work/results.jsonl"
for mode in render compile; do
    $rust $mode $data $dir $names | sed "s/^{/{\"engine\":\"rust\",\"mode\":\"$mode\",/" >>"$work/results.jsonl"
    $perl_xs bench/bench.pl $mode xs $data $dir $names 2>/dev/null |
        sed "s/^{/{\"engine\":\"perl-xs\",\"mode\":\"$mode\",/" >>"$work/results.jsonl"
    # The stash plays no part in compiling.
    [ $mode = compile ] && continue
    $perl_pure bench/bench.pl $mode perl $data $dir $names 2>/dev/null |
        sed "s/^{/{\"engine\":\"perl\",\"mode\":\"$mode\",/" >>"$work/results.jsonl"
done

rm -f "$work"/cold-*.json
if [ -z "${QUICK:-}" ]; then
    # Cold processes: buglist with its data as TT literals, so tpage needs
    # nothing but the template.
    cold=$work/cold
    mkdir -p "$cold"
    cp $dir/buglist.tt "$cold/"
    perl -MJSON::PP -e '
        local $/; open(my $fh, "<", $ARGV[0]) or die; my $d = JSON::PP->new->decode(<$fh>);
        sub tt { my ($v) = @_;
            return "{ " . join(", ", map { "$_ => " . tt($v->{$_}) } sort keys %$v) . " }" if ref $v eq "HASH";
            return "[ " . join(", ", map { tt($_) } @$v) . " ]" if ref $v eq "ARRAY";
            (my $s = "$v") =~ s/([\\\x27])/\\$1/g; return "\x27$s\x27" }
        print "[% ", join("; ", map { "$_ = " . tt($d->{$_}) } sort keys %$d), " -%]\n[% PROCESS buglist.tt %]\n";
    ' $data >"$cold/cold.tt"
    printf 'Hello, [%% name %%]!\n' >"$cold/hello.tt"
    for t in hello cold; do
        hyperfine -N -w 5 -r 40 --export-json "$work/cold-$t.json" \
            -n rust "target/release/tpage-rs --define name=World -I $cold $cold/$t.tt" \
            -n perl-xs "$perl_xs $xs/bin/tpage --define name=World --include_path=$cold $cold/$t.tt" \
            >/dev/null 2>&1
    done
fi

python3 -I - "$work" <<'PY'
import json, pathlib, platform, subprocess, sys
work = pathlib.Path(sys.argv[1])
rows = [json.loads(l) for l in (work / "results.jsonl").read_text().splitlines()]
get = lambda engine, mode, name: next((r for r in rows if (r["engine"], r["mode"], r["name"]) == (engine, mode, name)), None)
fmt = lambda us: f"{us / 1000:.2f} ms" if us >= 1000 else f"{us:.0f} µs"

def run(*cmd):
    return subprocess.run(cmd, capture_output=True, text=True).stdout.strip()

cpu = next((l.split(":", 1)[1].strip() for l in open("/proc/cpuinfo") if l.startswith("model name")), platform.processor())
print(f"{cpu}; {run('rustc', '--version')}; perl {run('perl', '-e', 'print $^V')}\n")
print("| template | mode | Rust | Perl TT + XS stash | pure-Perl TT | vs XS | vs pure Perl |")
print("|---|---|---|---|---|---|---|")
for mode in ["render", "compile"]:
    for name in ["buglist", "report", "mail"]:
        r, x, p = (get(e, mode, name) for e in ["rust", "perl-xs", "perl"])
        cells = [fmt(r["median_us"]), fmt(x["median_us"]), fmt(p["median_us"]) if p else "–",
                 f"{x['median_us'] / r['median_us']:.1f}×", f"{p['median_us'] / r['median_us']:.1f}×" if p else "–"]
        print(f"| {name} | {mode} | " + " | ".join(cells) + " |")
for t, label in [("hello", "one-line template"), ("cold", "buglist, data inlined")]:
    f = work / f"cold-{t}.json"
    if not f.exists():
        continue
    # hyperfine 2's schema: each result's wall-clock time per run.
    def median_us(result):
        times = sorted(m["time_wall_clock"]["value"] for m in result["measurements"])
        return times[len(times) // 2] * 1e6
    res = {r["name"]: r for r in json.loads(f.read_text())["results"]}
    rm, xm = median_us(res["rust"]), median_us(res["perl-xs"])
    print(f"| {label} | cold process | {fmt(rm)} | {fmt(xm)} | – | {xm / rm:.1f}× | – |")
print("\nMedians of 11 samples (cold: of 40 runs).  Raw results in target/bench/.")
PY
