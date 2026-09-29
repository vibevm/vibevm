# Code review

How to review a pull request.

1. Read the description first. If it does not say what changed and why, ask
   for that before reading the diff.
2. Read the tests before the implementation. A change with no test needs a
   reason in the description.
3. Check the boundaries: empty input, the largest value, the error path.
4. Mark each comment as blocking or as a suggestion. Never leave it unsaid.
5. One pass, then approve or request changes. Do not trickle comments in.

Commit hygiene is in [[commit-messages]]. Wording is in [[writing-style]].
