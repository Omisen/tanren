# Kanji data: sources and licence

> **This file is for whoever reads the repository. The obligation is discharged inside
> the app**, on the "Sources and licences" screen reachable from either home screen,
> because CC BY-SA and the EDRDG licence both require attribution in the medium the work
> travels in, and for an app that medium is the installed APK, not a file on GitHub.
>
> That screen is built from `shared/credits.rs` plus `features/kanji/levels.rs::credits`,
> and the edition it shows is read from the data itself rather than typed by hand, so it
> cannot drift from what actually ships. Keep this file and that list saying the same
> thing.

The tables in `levels/` are **derived from [kanjium](https://github.com/mifunetoshiro/kanjium)**
by Uros O., which is itself built on files belonging to the Electronic Dictionary
Research and Development Group (EDRDG) and James William Breen.

Both licences are the same one, and **they add up rather than replace each other**.

## kanjium

- Repository: <https://github.com/mifunetoshiro/kanjium>
- Licence: **Creative Commons Attribution-ShareAlike 4.0**
- Edition used: the `source` field inside each `levels/level-NN.json` records the commit
  that last touched `data/kanjidb.sqlite`, since kanjium carries no version number.

kanjium asks for this specific wording, and Tanren carries it:

> The pitch accent notation, verb particle data, phonetics, homonyms and other additions
> or modifications to EDICT, KANJIDIC or KRADFILE were provided by Uros O. through his
> free database.

## EDRDG: EDICT, KANJIDIC, KRADFILE

Most of what kanjium contains comes from these three, which are the property of the Group
and are used under its licence.

- Project: <https://www.edrdg.org/wiki/index.php/KANJIDIC_Project>
- Licence statement: <http://www.edrdg.org/edrdg/licence.html>
- Licence: **Creative Commons Attribution-ShareAlike 4.0**

## The level ordering is ours, and deliberately so

The order in which kanji are introduced is **not taken from any existing progression
list**. It is computed by `generate.py` with a topological sort over the component data
kanjium publishes in `elements.kanji_parts`, so that a kanji never comes before the
pieces it is built from, with ties broken by stroke count and then frequency. Both inputs
are CC BY-SA, like the rest of the data.

This matters beyond tidiness: published progression orders belong to the services that
designed them and are not free data. Component-first progression is an idea, and ideas are
not owned; a particular ordering of 2,136 characters is a work. Ours is generated, and the
generator is in the repository for anyone to re-run.

## What this means for Tanren

Both statements extend to "any data files which are derived from them". The tables in
this directory are exactly that, so **they are CC BY-SA 4.0**, not MIT. Tanren's code
stays MIT: code and data are separate works, and the share-alike travels with the data.

The EDRDG licence is specific about apps:

> For smartphone and tablet apps, acknowledgement must be made, e.g. on a separate
> screen accessed from a menu, such as one labelled "About", "Sources", etc. It is not
> sufficient just to mention it on a start-up/launch page of the app.

**So an APK that ships these tables needs a Sources screen inside the app, naming both
sources.** It is a condition of use, not a nicety. **That screen now exists**, and with it
the last thing blocking a release that carries kanji data. The `source()` field on each
table feeds it the exact edition in use.

The licence texts travel with the app too, not only their names: the CC BY-SA 4.0 legal
code and the SIL OFL 1.1 are files under `public/`, which ends up inside the package, and
the screen opens them. For the EDRDG statement the screen offers the permanent link
instead, which their licence explicitly allows where including the file is impractical.

## Regenerating

`generate.py` downloads what it needs and rewrites `levels/`. Never edit those files by
hand: an edit would be lost at the next run, and would quietly diverge from the source the
`source` field claims.

## The grade tables, which are on their way out

`first.json` … `secondary.json` at the top of this directory are the **previous**
dataset, derived from KANJIDIC2 and organised by Japanese school year. They are still
read by `features/kanji/data.rs` and go away with the exercise that uses them. Their
generator has already been replaced by the one for `levels/`.
