# Release checklist

Walk this before tagging a version.

1. The working tree is clean and on the main branch.
2. The version in the manifest matches the tag about to be pushed.
3. The changelog has an entry for this version, written for a reader.
4. The tests pass from a clean build directory.
5. The binary runs once by hand: `calc "7 / 2"` prints `3.5`.
6. Tag, push the tag, then announce.
