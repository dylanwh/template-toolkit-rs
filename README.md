# template-toolkit-rs

\o just a heads up this is an llm generated rust crate, however this README is
100% my own writing. I am responsible for this code existing, and I have a
reasonable understanding of it.  Most of this will deal with the process that
created this code.

## What Does This Do

These rust crates provide a template-toolkit parser which is about 14x faster
than the perl version and supports all features except `[% PERL %] / [% RAWPERL
%]`. It's able to parse all the templates in bugzilla (and a few other projects)
producing by-for-byte identical output. It also can run in a webbrowser wasm,
see [playground][]

[playground]: https://template-toolkit-rs.github.io
[beef::lean::Cow]: https://docs.rs/beef

## Quality

Having reviewed this code, it is of fair to middling quality. It uses `unwrap()`
liberally.  Many of the cases are fine, but many are not. No model is able to
correct these in bulk in a way that does not cause me concern - either it will try to replace `.unwrap()` with `expect()`
or `expect()` with `.map_err_with(|| unreachable!())`. When a custom harness forbids all of those, it will use `unwrap_or`
which makes the problems impossible to find.

A fully-loaded koolaid man will suggest the fix for this is fuzzing, and I suppose
there is truth there. I think if I find a use for this crate I'll go through and
use my human reasoning on each unwrap().

## Technology

I have used Google Argon 4, Claude Opus 5.5, and Claude Fable to produce almost
all the code in this repository.  I manage those processes inside emacs using a
package called agent-shell. For those that do emacs, it's similar to using an
inferior python/lisp/etc process where you can access it from any buffer. 

The llm runs on an egress-blocked virtual machine.

## Methodology

I don't put any value in using specific framing devices with those LLMs. I have
not read the most recent research and will be happy to be proven wrong, but I
think framing the llm with "You are a ninth level paladin..." is basically
homeopathy. 

What I have found surprisingly effective is to ask the llm:

> write a tool named `tpage-compare` which runs every template in a directory through Perl's 
> `tpage` and our `tpage-rs`. Report where their output differs.

Using tpage-compare against a collection of a few thousand templates I let the
llm churn from friday evening to saturday morning and I awoke to it passing all
the non-`[% PERL %]` templates. 

I think an incredibly useful way of using this technology is to parallelize
mechanical research. For instance, would [beef::lean::Cow][] work in 32bit web
assembly? (yes) Does mimalloc make a difference for performance on x86 linux?
(also yes, though only 2%). Those tests were carried out in parallel.

## Licensing 

There is no universe where this isn't a derivative work of Template Toolkit.
Ethically I believe all llm-generated code is a derivative of open source.  This
project like Template-Toolkit is licensed under the same terms as Perl itself.

