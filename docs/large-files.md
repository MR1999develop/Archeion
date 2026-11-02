# Large files saved as chunks

Design note for the "large file handling" of Archeion: how a file that
changes a little is backed up without copying it again in full.

## What the user gets

- On an **unencrypted** destination, files of at least the profile's
  threshold (`large_files.threshold_mb`, 500 MB unless changed, 0 =
  never; Storage section of the profile) are saved as chunks. Smaller
  files stay what they always were: one zip per file or a plain copy,
  openable with any tool.
- On an **encrypted** destination every file is saved as chunks, whatever
  its size.
- A **complete** backup still holds every file whole (zip, plain copy or
  AES zip), so it can be restored with 7-Zip alone (and the recovery key
  for an encrypted one). Only the incrementals after it hold chunks.
- An incremental stores only the chunks the destination does not hold
  yet, plus the file's chunk list in the manifest. Identical chunks are
  stored once across the whole destination.

## Chunking

FastCDC with normalized chunking and a Gear rolling hash
(`crates/engine/src/chunker.rs`): minimum 256 KiB, average 1 MiB, maximum
4 MiB. Boundaries depend on the content around them, so an insertion or a
deletion in the middle of a file changes one or two chunks and the rest
are found again.

Why 1 MiB: a 1 TB file gives about a million chunks. Each one is an entry
of two values in a manifest and, when new, a file on disk with about
100 bytes of zip overhead (128 with AES). Smaller chunks would find more
duplicates but multiply files and manifest size; larger ones would store
more for a small edit. The parameters are **not** part of the format:
every chunk is named by its content and every file lists its chunks with
their sizes, so changing them only makes the next backup store more new
chunks. Tests use 1 KiB / 4 KiB / 16 KiB through a hidden `Tuning`.

## On disk

```
2026-11-01_120000_C\              complete archive
  manifest.jsonl                  big.vhdx: whole copy + chunk list
  zipped\C\VMs\big.vhdx.zip       the whole copy, as before
2026-11-02_120000_I\              incremental
  manifest.jsonl                  big.vhdx: chunk list only
  zipped\                         (whole copies of small files)
  chunks\3f\3f9a…c2.zip           a new chunk (plain: chunks\3f\3f9a…c2)
```

A chunk file has the form of the destination's other files:

| Destination | Chunk file | Inside |
| --- | --- | --- |
| plain copies | `chunks/<ab>/<name>` | the bytes |
| one zip per file | `chunks/<ab>/<name>.zip` | one entry `<name>`, deflated unless the file's type is already compressed |
| encrypted | `chunks/<ab>/<name>.zip` | one AES-256 entry `data`, undated, password = recovery key |

`<name>` is 64 hex digits: the BLAKE3 hash of the chunk, or on an
encrypted destination a BLAKE3 keyed with a key derived from the master
key (`"Archeion 2026-11-01 chunk name key v1"`), so a name cannot be
matched against the hash of a known file. The same content has the same
name in every archive of the destination: names reveal which chunks
repeat, nothing else. `<ab>` is the first byte, so no folder holds more
than a few thousand files.

## Manifest (format 2)

A stored file entry may carry:

```json
"chunks":{"whole":true,"list":[["3f9a…c2",1048576],["07bb…41",734003]]}
```

- `whole: true` — complete archives (and merged complete ones): the
  archive also holds the whole copy, and each chunk can be read from it
  at the offset given by the sizes of the chunks before it.
- `whole: false` — the archive holds only the list; the file is the
  concatenation of the chunks, read wherever they are.

The sizes must add up to the file size; a manifest where they do not is
refused when read. The whole-file BLAKE3 (`hash`) stays as before and is
checked after reassembly. The header's `format` is 2 when a manifest can
hold chunk lists; format 1 manifests are still read.

## Where a chunk is read

A chunk can be read from any archive of the destination that has it:

1. a chunk file `chunks/<ab>/<name>` in some archive folder, or
2. a byte range of a whole copy whose entry has `whole: true`.

Since chunks are named by their content, any copy is as good as another.
The rule every operation keeps is: **every chunk listed by a manifest of
the destination can be read from at least one archive of the
destination**. And, by construction, from an archive no newer than the one
listing it, which is why restoring a date only looks at the archives up
to that date and why deleting the newest archive needs no chunk work.

Reading a run of chunks that are consecutive ranges of one whole copy
opens that copy once and reads it forward (skipping the gaps); a range
before the current position starts a new pass. Every chunk is checked
against its name before use.

## The complete-archive question

The incrementals must find the chunks of the file version kept in the
complete archive. Three options were weighed:

| | Space at each complete | Robustness | Complexity |
| --- | --- | --- | --- |
| (a) complete stores the whole copy **and** the chunks | doubles the chunked files | simple, chunks always files | low |
| (b) chunk references point into the whole copy (**chosen**) | none extra | same as today: a damaged whole copy breaks what depends on it in both designs | reading inside a zip entry from an offset: done by reading forward (zips and AES zips cannot seek inside deflated or encrypted data) |
| (c) store chunked files of complete archives as chunks only | none | breaks the rule that a complete archive opens with 7-Zip alone | low |

(b) keeps complete archives exactly as before and costs no extra space.
Its costs are handled as follows:

- **Reading forward**: a restore reads the whole copy once in the common
  case; only a file whose content moved backwards (rare) reopens it.
- **Merges**: when a merge drops a whole copy whose ranges are still
  listed by a remaining archive, those chunks are cut out of it into
  chunk files of the merged archive (see below).
- **Verify**: the full check reads every whole copy and checks that it
  splits into its chunk list.

## Backup

For each file to copy, `ChunkPolicy::storage` (core,
`large_files.rs`) decides:

- `Whole`: as before.
- `WholeAndChunkList` (complete archives, chunked files): the whole copy
  is written while a chunker reads the same bytes and lists the chunks.
- `ChunksOnly` (incrementals of a chain that knows chunks): the file is
  read through the chunker; each chunk not found in the destination
  (chunk files of every archive, ranges of every whole copy, chunks of
  the archive being built) is written as a chunk file, under a temporary
  name renamed once complete.

Chunk files written for a file that then failed, or left over by an
interrupted run and no longer needed, are removed before the commit.

**Free space**: before copying, a chunked file counts its growth plus one
chunk (or its full size when it is new), not its size: a large file
edited a little no longer looks like it needs its full size. A file
rewritten throughout counts less than it writes; the full-disk check
during the copy then stops the run, nothing is committed, and what was
written is reused by the next run.

**Resume**: whole copies are journalled as before. A whole copy with a
chunk list is journalled too, and its list is computed again from the
source on resume (reading, no writing). A chunks-only file is not
journalled: the next run reads it again and reuses every chunk file of
the unfinished archive once it reads back whole (a damaged one is written
again).

## Migration

Existing archives keep their format and stay readable. Chunks start with
the next complete archive written by this version:

- the newest complete archive of a destination has format 1 and chunking
  is on (encrypted destination, or a threshold set) → the next
  **automatic** backup is complete (`needs_new_chain`), starting a chain
  whose complete archive lists chunks;
- an incremental the user asks for explicitly on such a chain stays
  whole (it continues the old chain);
- an unencrypted destination with the threshold at "never" is not forced
  into a complete backup: nothing would be chunked.

So a chain never mixes: incrementals chunk only when their chain's
complete archive is format 2. (Correctness would not depend on it, since
any chunk is found wherever it is, but without a chunk list in the
complete archive the first incremental would store every chunk.)

An older Archeion refuses format 2 manifests ("newer than this version").

## Merge and deletion

A merge fuses a group into its last archive, as before; for chunks:

1. The merged manifest takes each stored file's chunk list from the member
   that holds its newest content.
2. A merged **complete** archive holds every file whole: a chunks-only
   file is put back together (each chunk checked, then the whole hash)
   into a whole copy that keeps its list.
3. Chunk files: the merged archive receives a chunk file for every chunk
   that a remaining manifest (outside the group, or the merged one) lists
   and that nothing remaining provides (no chunk file or whole copy
   outside the group, no whole copy in the merged archive). It is hard
   linked from a member's chunk file, or cut out of a member's whole copy.
   Everything else goes away with the members: that is the garbage
   collection, and it frees the space of versions no longer listed.
4. The journal, `.merging` folder and rename commit are unchanged, so an
   interruption leaves either the old archives or the merged one.

The scheduled merge still never touches the initial complete archive.
Deleting an intermediate incremental is a merge with the next one;
deleting the newest is a removal (nothing older lists its chunks).

Chunk files outside the group that become unused are not collected until
their own archive is merged.

## Restore, versions, extract

A restore (a list of items, each a file or folder at one date) takes
each file from the newest archive up to its item's date that stores it;
a chunks-only file is reassembled from the chunks of the archives up to
the newest date restored, each checked against its name, then the whole
file against its hash, into a temporary file renamed at the end.
Versions and search work on the manifests as before.

`archeion extract --destination <folder> --to <folder> [--archive <name>]
[--path <file or folder>] [--overwrite] [--password … | --recovery-key …
| --stdin]` restores from a destination folder alone, read-only, without
settings or a profile (secrets also from `ARCHEION_PASSWORD` /
`ARCHEION_RECOVERY_KEY`).

## Integrity check

The full check also reads every chunk file and checks it against its
name, checks that every chunk a manifest lists can be read somewhere, and
that every whole copy with a chunk list splits into that list. The
restore test reassembles the chunked files it samples.

## Recovery by hand

A complete archive needs nothing more than before. For an incremental's
chunked file, the manifest (encrypted manifests open with the recovery
key) lists its chunks in order; each one is the file `chunks/<ab>/<name>`
of that archive or an earlier one (open the zip, with the recovery key if
encrypted), or else the bytes at the listed offset of the whole copy in
the complete archive. Concatenating them in order gives the file.

## Known limits

- Each manifest repeats the chunk lists of the files it stores: about 80
  bytes per chunk in the file, 40 in memory. Every backup that chunks
  reads all manifests of the destination to know which chunks exist.
- `bytes_stored` of an archive still counts a chunked file at its full
  size.
- Chunk files, like whole copies, are not flushed one by one before the
  commit rename.
