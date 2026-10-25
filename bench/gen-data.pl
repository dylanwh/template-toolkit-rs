#!/usr/bin/perl
# Writes bench/data.json: the variables every benchmark case renders with.
# Deterministic (a fixed linear congruential generator), so the file only
# changes when this script does.
use strict;
use warnings;
use JSON::PP;

my $seed = 42;
sub rnd { my ($n) = @_; $seed = ($seed * 1103515245 + 12345) % 2**31; return $seed % $n }
sub pick { my @list = @_; return $list[rnd(scalar @list)] }

my @products = qw(Core Firefox Toolkit DevTools Thunderbird Calendar Infrastructure WebExtensions);
my @components = ('General', 'Layout', 'Networking', 'Graphics', 'DOM: Events', 'JavaScript Engine', 'Build Config', 'Preferences');
my @statuses = qw(NEW ASSIGNED REOPENED RESOLVED VERIFIED);
my @resolutions = qw(FIXED INVALID WONTFIX DUPLICATE WORKSFORME);
my @people = map { { name => $_->[0], email => $_->[1] } } (
  ['Ada Lovelace', 'ada@example.org'], ['Grace Hopper', 'grace@example.org'],
  ['Alan Turing', 'alan@example.org'], ['Katherine Johnson', 'kj@example.org'],
  ['Edsger Dijkstra', 'ewd@example.org'], ['Barbara Liskov', 'liskov@example.org'],
  ['Donald Knuth', 'knuth@example.org'], ['Frances Allen', 'fran@example.org'],
);
my @words = qw(crash when opening tab with large image after update the dialog menu
  fails to render slow memory leak in worker scroll jank broken link focus lost on
  reload incorrect encoding of <script> & "quoted" text timeout during sync);
my @keywords = qw(crash regression perf intermittent sec-low dataloss access);

my @bugs;
for my $id (1 .. 200) {
  my $status = pick(@statuses);
  my $open = $status =~ /^(NEW|ASSIGNED|REOPENED)$/ ? 1 : 0;
  push @bugs, {
    id         => 100000 + $id * 7,
    product    => pick(@products),
    component  => pick(@components),
    status     => $status,
    resolution => $open ? '' : pick(@resolutions),
    is_open    => $open,
    assignee   => pick(@people),
    reporter   => pick(@people),
    summary    => join(' ', map { pick(@words) } 1 .. 4 + rnd(12)),
    keywords   => [map { pick(@keywords) } 1 .. rnd(4)],
    votes      => rnd(50),
    priority   => 'P' . (1 + rnd(5)),
    changed    => sprintf('2026-%02d-%02d %02d:%02d', 1 + rnd(12), 1 + rnd(28), rnd(24), rnd(60)),
  };
}

my $data = {
  title   => 'Bugs changed this week',
  columns => [map { { name => $_->[0], title => $_->[1] } }
    ([id => 'ID'], [product => 'Product'], [status => 'Status'], [assignee => 'Assignee'],
     [summary => 'Summary'], [keywords => 'Keywords'], [votes => 'Votes'], [changed => 'Changed'])],
  bugs    => \@bugs,
};

open(my $out, '>', $ARGV[0] // 'bench/data.json') or die "$!\n";
print $out JSON::PP->new->canonical->pretty->encode($data);
