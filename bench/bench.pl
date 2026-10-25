#!/usr/bin/perl
# The Perl side of the benchmarks; bench/bench.rs is the Rust side, and
# scripts/bench.sh runs both.
#
#   bench.pl MODE STASH DATA DIR NAME...
#
# MODE is `output` (each template's output, to check the two sides agree),
# `render` (the compiled template cached, as in a long-running process) or
# `compile` (text to a runnable template, uncached).  STASH is `xs` or
# `perl`.  Prints a JSON line per template.
use strict;
use warnings;
use JSON::PP;
use Time::HiRes qw(time);
use Template;
use Template::Config;

my ($mode, $stash, $data_file, $dir, @names) = @ARGV;
$Template::Config::STASH = $stash eq 'xs' ? 'Template::Stash::XS' : 'Template::Stash';

my $vars = do {
  open(my $fh, '<', $data_file) or die "$data_file: $!\n";
  local $/;
  JSON::PP->new->decode(<$fh>);
};
my $json = JSON::PP->new->canonical;
binmode(STDOUT, ':utf8');

# Per-operation seconds: calibrated so a sample takes at least 50 ms, then
# 11 samples; the median and the best.
sub measure {
  my ($op) = @_;
  $op->() for 1 .. 3;
  my $n = 1;
  while (1) {
    my $t = time;
    $op->() for 1 .. $n;
    last if time - $t >= 0.05;
    $n *= 2;
  }
  my @per_op;
  for (1 .. 11) {
    my $t = time;
    $op->() for 1 .. $n;
    push @per_op, (time - $t) / $n;
  }
  @per_op = sort { $a <=> $b } @per_op;
  return { median_us => $per_op[5] * 1e6, min_us => $per_op[0] * 1e6, iterations => $n };
}

my $tt = Template->new({ INCLUDE_PATH => $dir }) or die Template->error;
for my $name (@names) {
  my $file = "$name.tt";
  if ($mode eq 'output') {
    my $out = '';
    $tt->process($file, $vars, \$out) or die $tt->error;
    print $json->encode({ name => $name, output => $out }), "\n";
  }
  elsif ($mode eq 'render') {
    my $result = measure(sub { my $out = ''; $tt->process($file, $vars, \$out) or die $tt->error });
    print $json->encode({ name => $name, %$result }), "\n";
  }
  elsif ($mode eq 'compile') {
    my $text = do { open(my $fh, '<', "$dir/$file") or die "$file: $!\n"; local $/; <$fh> };
    my $provider = $tt->context->load_templates->[0];
    # A template given as a reference is compiled afresh each time.
    my $result = measure(sub { my ($doc, $error) = $provider->fetch(\$text); die $doc if $error });
    print $json->encode({ name => $name, %$result }), "\n";
  }
  else {
    die "unknown mode: $mode\n";
  }
}
