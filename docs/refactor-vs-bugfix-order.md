# Refactor before or after bug fixes? — primary-source research

Research date: 2026-11-12. Scope: sequencing of refactoring vs bug fixing,
researched against the originators' own writings (books, first-party sites).
No code changes; this file is the deliverable.

## The question

Should refactoring be done only after all bugs are fixed, or is refactoring
before/during bug fixing acceptable? Sub-questions: what is the canonical
ordering of "behavior change" vs "structure change" within one work episode,
and what are the recognized failure modes of refactoring first?

## Source positions

### 1. Martin Fowler — Refactoring (book) / martinfowler.com

**Definition (the constraint that drives everything else).**
Fowler, "Definition Of Refactoring" bliki, 2004-09-01,
<https://martinfowler.com/bliki/DefinitionOfRefactoring.html>:

> Refactoring (noun): a change made to the internal structure of software to
> make it easier to understand and cheaper to modify **without changing its
> observable behavior**.
> Refactoring (verb): to restructure software by applying a series of
> refactorings without changing its observable behavior.

**Refactoring during bug fixing is normal and expected.**
Fowler, "Opportunistic Refactoring" bliki, 2011-11-01,
<https://martinfowler.com/bliki/OpportunisticRefactoring.html>:

> Although there are places for some scheduled refactoring efforts, I prefer
> to encourage refactoring as an opportunistic activity, done whenever and
> wherever code needs to cleaned up - by whoever.
>
> This opportunity can come at various parts of implementing some new
> functionality **or fixing a bug**. One is a preparatory refactoring, where
> before you begin to implement something you see that this task would be
> easier if an existing class's API was structured differently. You first
> refactor it to how it ought to be and then start adding your functionality.

**The counter-guardrail (refactor only from green).** Same article:

> This continuous attention to the code is important - but do remember that
> **you should only refactor when your tests are green**.

**Rabbit-hole warning.** Same article:

> There is a genuine danger of going down a rabbit hole here, as you fix one
> thing you spot another, and another, and before long you're deep in yak
> hair. Skillful opportunistic refactoring requires good judgement, where you
> decide when to call it a day.

**Two Hats (never mix the two kinds of change).** Fowler, "An example of
preparatory refactoring", 2015-01-05,
<https://martinfowler.com/articles/preparatory-refactoring-example.html>,
sidebar "The Two Hats":

> In the Refactoring book, I passed on Kent's metaphor of the two hats. His
> notion is that when you're programming, you can operate in one of two modes:
> **refactoring and adding function**. When you're wearing the refactoring
> hat, every change you make preserves observable behavior, keeps the tests
> green, and allows you to make many small changes without going near a
> debugger. When you add function, however, things are more open ended as you
> will add tests and break existing tests. The adding function hat is more
> stressful and riskier, so it's nice to wear the refactoring hat as much as
> possible.

**Preparatory refactoring defined (refactor *first* is a named, legitimate
move).** Same article:

> One useful notion is that of Preparatory Refactoring. This is where I'm
> adding a new feature, and I see that the existing code is not structured in
> such a way that makes adding the feature easy. So first I refactor the code
> into the structure that makes it easy to add the feature, or as Kent Beck
> pithily put it **"make the change easy, then make the easy change."**

### 2. Michael Feathers — Working Effectively with Legacy Code (2004)

Book: *Working Effectively with Legacy Code*, Prentice Hall, 2004,
ISBN 978-0-13-117705-5. Quotes below verified against the publisher's free
sample chapter 1
(<https://www.informit.com/content/images/0131177052/samplechapter/0131177052_ch01.pdf>)
and an archive.org full scan of the same edition.

**Legacy code = untested code; tests are the precondition for safe change.**

> To me, legacy code is simply code without tests. [...]
> Code without tests is bad code. It doesn't matter how well written it is;
> it doesn't matter how pretty or object-oriented or well-encapsulated it is.
> With tests, we can change the behavior of our code quickly and verifiably.
> Without them, we really don't know if our code is getting better or worse.

**Bug fix and refactor are distinct change types.** Chapter 1, "Four Reasons
to Change Software": adding a feature, fixing a bug, improving the design,
optimizing resource usage. On the design change:

> The act of improving design without changing its behavior is called
> refactoring. [...] The key thing about refactoring from a change point of
> view is that **there aren't supposed to be any functional changes when you
> refactor**.

**The sequencing algorithm.** Chapter 2, "The Legacy Code Change Algorithm"
(p. 18):

> When you have to make a change in a legacy code base, here is an algorithm
> you can use.
> 1. Identify change points.
> 2. Find test points.
> 3. Break dependencies.
> 4. Write tests.
> 5. **Make changes and refactor.**

Note the order: tests exist *before* the behavior change; refactoring is the
final step of the same episode, after the change works. And:

> The day-to-day goal in legacy code is to make changes, but not just any
> changes. We want to make functional changes that deliver value while
> bringing more of the system under test. At the end of each programming
> episode, we should be able to point not only to code that provides some new
> feature, but also its tests.

**Cover and Modify (vs Edit and Pray).** Chapter 2:

> Cover and Modify is a different way of making changes. [...] Covering
> software means covering it with tests. When we have a good set of tests
> around a piece of code, we can make changes and find out very quickly
> whether the effects were good or bad.

**The narrow exception to "tests first".** Dependency-breaking refactorings
may precede tests, but conservatively. Preface:

> The refactorings in Dependency-Breaking Techniques are special in that they
> are meant to be done without tests, in the service of putting tests in
> place.

Chapter 2:

> Is this safe to do these refactorings without tests? It can be. [...] The
> trick is to do these initial refactorings **very conservatively**.

### 3. Robert C. Martin — the Boy Scout Rule

"8. The Boy Scout Rule", *97 Things Every Programmer Should Know*, O'Reilly,
2010, <https://www.oreilly.com/library/view/97-things-every/9780596809515/ch08.html>
(O'Reilly blocks bots; quote verified against a scanned copy of the same
book). By "Uncle Bob":

> The Boy Scouts have a rule: "Always leave the campground cleaner than you
> found it." [...] What if we followed a similar rule in our code: **"Always
> check a module in cleaner than when you checked it out."** [...] You don't
> have to make every module perfect before you check it in. You simply have
> to make it a little bit better than when you checked it out.

This is a continuous, per-touch rule — the opposite of a "refactor only after
all bugs are done" gate. (Same rule appears in *Clean Code*, 2008, ch. 1.)

### 4. Kent Beck — red/green/refactor

Beck's canonical statement is *Test-Driven Development: By Example* (Addison-
Wesley, 2002); archive.org's copies are lending-restricted, so the loop text
below is quoted from the original XP wiki page "Test Driven Development"
(<https://wiki.c2.com/?TestDrivenDevelopment>), the primary community source
of the practice, authored by Beck's XP circle:

> add a test, get it to fail, and write code to pass the test [...]
> If you have any duplicate logic, or inexpressive code, refactor to remove
> duplication and increase expressiveness [...]
> Run the tests again, you should still have the Green Bar. If you get the
> Red Bar, then you made a mistake in your refactoring. Fix it now and re-run.

Two ordering facts: the refactor step sits *after* green, never while red;
and refactoring is a first-class step of every micro-cycle, not a phase that
waits for all features/bugs to finish. (Beck's own aphorism for
refactor-first — "make the change easy, then make the easy change" — is
quoted by Fowler above.)

## Counter-positions (when refactor-first is wrong)

1. **While tests are red.** Fowler: "you should only refactor when your tests
   are green." Beck's loop: red during refactor = mistake; revert to green
   first. Refactoring on top of uncommitted broken behavior loses the
   behavior-preservation proof that makes refactoring safe.
2. **Mixed in one change.** Two Hats: you are in one mode at a time. Feathers:
   "there aren't supposed to be any functional changes when you refactor."
   Behavior change and structural change in one commit/change cannot be
   verified (which part caused the new behavior?), and rollback becomes
   all-or-nothing. They are separate steps with tests run in between.
3. **Code about to be deleted or rewritten.** None of the sources prescribes
   refactoring code whose lifetime is over; Fowler's test for opportunistic
   refactoring is that the cleanup pays for the *current* task ("refactoring
   ... makes the code base easier to work with, thus allowing the team to add
   value more quickly"), and his rabbit-hole warning caps the spend. A
   preparatory refactor only makes sense when the code stays and the next
   change(s) reuse it.
4. **Speculative big-bang refactoring.** Fowler: refactoring is opportunistic,
   small, and tied to work being done now; large scheduled efforts are the
   exception, not the rule. Feathers: refactor *after* the change works, in
   the same episode, not as a gate before any behavior work.
5. **Refactoring to postpone the bug fix.** Feathers' algorithm makes the
   behavior change the point of the episode; tests and dependency breaks are
   in its service. Refactor-first is justified only as far as it makes the
   concrete fix cheaper (preparatory refactoring), not as open-ended
   improvement.

## Synthesis — the canonical sequencing

There is no "bugs first, refactor later" phase gate in any primary source.
The agreed model is per-episode, not global:

1. Pick the next behavior change (a bug fix here).
2. If the code is untestable around the change point, do the enabling work
   first: find test points, break dependencies conservatively, write
   (characterization) tests. *This enabling work is itself refactoring — so
   "refactor before the bug fix" is already the prescribed move whenever the
   fix would otherwise be blind.* (Feathers step 1–4; Fowler's preparatory
   refactoring; Beck's "make the change easy".)
3. Make the behavior change; get tests green.
4. Then refactor with tests green, behavior-preserving, separate commit.
5. Opportunistically leave each touched module slightly cleaner (Boy Scout
   Rule), small enough to avoid the rabbit hole.

So: **bugs-first and refactor-first are both wrong as global rules.** The
sources prescribe *interleaved, per-change sequencing*: enabling refactor →
test → fix → refactor, one episode at a time. Global "fix all bugs, then
refactor" delays every fix on untestable code (Feathers: gymnastics without a
net) and forgoes the compounding cleanup (Fowler, Martin). Global "refactor
everything first" is the rejected big-bang anti-pattern.

## Application to recomp2jam

Current state: ~10 open live-playtest bug tickets (#129–#138: planet
carousel, channel scrollbar, options dialog/tabs, room hover/stacked buttons,
black side-screens, emblem lit-state, repaint loop) and four refactor tickets
(#139 ScreenGame God-class extraction, #140 o2jam-net duplication cures, #141
shell-core extraction + mouse_click seam-drift fix, #142 parser cleanup),
already value-ordered with #141 first.

Recommendation (from the sources, mapped to our tickets):

1. **Do #141 first — it is Feathers step 3, not a refactor detour.** The
   seam-drift fix is dependency-breaking in exactly Feathers' sense: it puts
   input/chrome behavior under automated test. Most open bugs (#129–#133,
   #137) are input/chrome-class, so #141 is preparatory refactoring for the
   whole bug batch (Fowler: "make the change easy, then make the easy
   change"). Ship it before touching individual bugs.
2. **Then fix bugs one episode at a time, not "all bugs then refactors".**
   Per bug: pin live-playtest evidence, write the failing rig/repro test
   (Feathers: characterization test before the change), fix, green, then
   small local refactor in the same area if it speeds the next fix — separate
   commits (Two Hats). The Boy Scout Rule applies within each episode.
3. **Gate the big structural tickets on contact, not on bug count.** #139
   (ScreenGame extraction) and #140 (net duplication) start when a bug
   episode touches that area and the extraction makes the fix cheaper;
   neither blocks the bug queue, and neither is done with any test red.
   #142 stays last (already value-ordered).
4. **Guardrails:** never refactor while a test is red; no behavior change
   mixed into a structural commit; no cleanup of code slated for
   deletion/rewrite; keep each refactor small (rabbit-hole rule).

## Verification notes

- All URLs fetched 2026-11-12. martinfowler.com pages read directly.
- Feathers quotes: ch. 1 verified against InformIT's official sample-chapter
  PDF; ch. 2 change algorithm and preface quotes verified against an
  archive.org full scan of the same edition (ISBN 9780131177055).
- Uncle Bob quote verified against a scanned copy of *97 Things Every
  Programmer Should Know* (O'Reilly page bot-blocks; chapter 8).
- Beck book text not quoted directly (archive.org lending-restricted); the
  red/green/refactor loop is quoted from the original XP wiki (wiki.c2.com),
  and Beck's "make the change easy" aphorism is quoted as cited by Fowler.
