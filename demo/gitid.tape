# gitid demo — basic usage.
#
# Render with:  task demo:render   (builds the release binary, then runs vhs)
# or directly:  vhs demo/gitid.tape
#
# Runs against a throwaway HOME via demo/sandbox.sh, so recording does not touch
# your real git identities. See demo/sandbox.sh for the hermetic setup.

Output demo/gitid.gif
Output docs/public/gitid.mp4
Output docs/public/gitid.webm

Set Shell bash
Set FontSize 20
Set Width 1440
Set Height 760
Set Padding 24
Set Margin 0
Set TypingSpeed 55ms
Set Theme "Catppuccin Mocha"

# Hermetic setup — not shown in the recording.
Hide
Type "source demo/sandbox.sh" Enter
Sleep 1s
Show

# 1. Create two identities.
Type "gitid add work --git-name 'Jane Doe' --email jane@corp.example --ssh-key ~/.ssh/id_work --non-interactive"
Sleep 500ms
Enter
Sleep 2s
Type "gitid add personal --git-name 'Jane' --email jane@home.example --ssh-key ~/.ssh/id_personal --non-interactive"
Sleep 500ms
Enter
Sleep 2s

# 2. Assign each to a directory tree — from anywhere, no cd required.
Type "gitid use work ~/code/work" Enter
Sleep 1500ms
Type "gitid use personal ~/code/oss" Enter
Sleep 1500ms

# 3. List profiles — the dot marks the one active for the current directory.
Type "gitid list" Enter
Sleep 2500ms

# 4. Prove it: any repo under ~/code/work now uses the work identity.
Type "cd ~/code/work/app && git config user.email" Enter
Sleep 2500ms

# 5. Inspect what resolves where.
Type "gitid current" Enter
Sleep 2500ms
Type "gitid dirs" Enter
Sleep 2500ms

# 6. Diagnose configuration problems.
Type "gitid doctor" Enter
Sleep 3s
