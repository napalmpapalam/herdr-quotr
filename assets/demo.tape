# Demo recording for herdr-quotr. Regenerate with:
#   cargo build --release && vhs assets/demo.tape
#
# Drives the picker against assets/demo.jsonl via QUOTR_TRANSCRIPT, so the recording needs no
# herdr server and no live Claude session. Keys only — VHS cannot drive the mouse, which is
# quotr's primary input; `V` and `j` stand in for a drag.

Require vhs

Output assets/demo.gif

Set Shell "bash"
Set FontSize 14
Set Width 1200
Set Height 760
Set Padding 20
Set TypingSpeed 55ms

Hide
Type "clear && QUOTR_TRANSCRIPT=assets/demo.jsonl target/release/herdr-quotr" Enter
Sleep 2s
Show

# Opens on the newest line. Turn jumps are exact where `j` counts drift with wrapping, so
# navigate by turn first and only step within one.
Sleep 1.5s
Type "g" Sleep 600ms
Type "]" Sleep 900ms
Type "jj" Sleep 700ms

# Select a paragraph, whole lines.
Type "V" Sleep 500ms
Type "jj" Sleep 1s

# Ask about it. The box opens under the quote.
Type "c" Sleep 700ms
Type "does the breaker share this bucket?" Sleep 400ms
Enter Sleep 2s

# A second pair, from the table in the last answer — a table always quotes linewise.
Type "]" Sleep 600ms
Type "]" Sleep 900ms
Type "jjjjjj" Sleep 800ms
Type "V" Sleep 500ms
Type "jjjj" Sleep 1s
Type "c" Sleep 700ms
Type "is the middle row still true for PUT?" Sleep 400ms
Enter Sleep 2.5s

# Both pairs are banked; `s` would send them into the agent's input box, unsubmitted.
Sleep 1.5s
Hide
Type "q"
