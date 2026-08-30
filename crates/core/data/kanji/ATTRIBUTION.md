# Kanji data: source and licence

The files in this directory are **derived from KANJIDIC2**, which is the property of the
Electronic Dictionary Research and Development Group (EDRDG) and of James William Breen.

- Project: <https://www.edrdg.org/wiki/index.php/KANJIDIC_Project>
- Licence statement: <http://www.edrdg.org/edrdg/licence.html>
- Licence: **Creative Commons Attribution-ShareAlike 4.0**
- Edition used: the `source` field inside each `.json` file records the KANJIDIC2
  `database_version` these tables were generated from.

## What this means for Tanren

The EDRDG licence applies to the dictionary files "and any data files which are derived
from them". The tables in this directory are exactly that, so **they are CC BY-SA 4.0**,
not MIT. The rest of Tanren stays MIT: the code and the data are separate works, and the
share-alike travels with the data.

The licence also states, for software that ships the files:

> acknowledge the usage and source of the files in the documentation, publicity
> material, WWW site of the package
>
> For smartphone and tablet apps, acknowledgement must be made, e.g. on a separate
> screen accessed from a menu, such as one labelled "About", "Sources", etc. It is not
> sufficient just to mention it on a start-up/launch page of the app.

So an APK that carries these tables **must have a Sources screen**. That is a condition
of use, not a nicety.

## Regenerating

`generate.py` downloads KANJIDIC2 and rewrites the tables. Never edit them by hand: an
edit would be lost at the next run, and would also quietly diverge from the dictionary
the `source` field claims.
