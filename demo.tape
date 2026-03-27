# VHS demo recording for README
# Run: vhs demo.tape

Output screenshots/demo.gif
Set Width 1400
Set Height 600
Set FontSize 16
Set Theme "Dracula"
Set Padding 20
Set Framerate 15

# Launch game and immediately start (hide shell)
Hide
Type "./target/release/voidfleet"
Enter
Sleep 2s
Enter
Sleep 1s
Show

# Watch travel — stars, fleet, collectibles
Sleep 6s

# Quick look at upgrades
Type "u"
Sleep 2s
Escape

# Visit Pip
Type "b"
Sleep 2s
Type "p"
Sleep 1s
Escape

# Skip to battle/raid
Type " "
Sleep 10s

# Skip to next phase
Type " "
Sleep 8s

# Stats
Tab
Sleep 2s
Escape

# Watch a bit more
Sleep 3s

# Quit
Type "q"
