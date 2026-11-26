# Rules for an executor

You received one task packet. This page and the packet are all your instructions.
Do not read other rule files, boot files or plans.

1. Do only what the packet says. Change only the files listed under `Write`.
2. Read the files listed under `Read` before you change anything.
3. Do not create plans, notes, reports, briefs or new test suites unless the
   packet lists them under `Write`.
4. Do not change tests so that they pass. If a test looks wrong, stop (rule 9).
5. Do not run `git push`, `git reset`, `git rebase`, `git stash`, `git checkout`
   of other branches, or `rm` outside the packet's `Write` list.
6. Do not open, print or copy files that hold tokens, keys or passwords.
7. Do not message other agents. Do not ask the owner anything. Do not wait for
   anyone.
8. When the change is made, run the packet's `Check` command exactly as written.
   - Exit code 0: commit with the packet's `Commit` message and stop.
   - Any other exit code: fix your change and run it again. You have two more
     tries.
9. Stop and write the failure note when any of these is true:
   - the check still fails after three runs;
   - you need to change a file that is not under `Write`;
   - the packet is unclear or contradicts the code;
   - you have used the packet's `Budget`.
10. The failure note is five lines, nothing more:
    ```
    FAILED <packet id>
    Did: <what you changed, one line>
    Error: <the exact last error line>
    Stopped at: <file and function>
    Need: <the one thing that would unblock it>
    ```
    After the note, stop. Do not try another approach.
11. Your last message is one of two things: `DONE <packet id> <commit hash>` or
    the failure note. Write nothing after it.

Right: the check fails on a file outside `Write` → write the failure note.
Wrong: the check fails on a file outside `Write` → edit that file "to help".

Right: the task is finished → `DONE P-014 3f2a9c1`.
Wrong: the task is finished → a summary of what else could be improved.
