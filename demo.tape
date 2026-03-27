# VHS demo recording for README
# Run: vhs demo.tape
# Requires: vhs (https://github.com/charmbracelet/vhs)

Output screenshots/demo.gif
Set Width 120
Set Height 35
Set FontSize 14
Set Theme "Dracula"
Set TypingSpeed 50ms
Set Padding 10

# Start the game
Type "cargo run --release"
Enter
Sleep 2s

# Title screen — press Enter to start
Enter
Sleep 1s

# Watch travel scene
Sleep 8s

# Open upgrades
Type "u"
Sleep 3s
Escape

# Visit Pip on the bridge
Type "b"
Sleep 3s
# Pet Pip
Type "p"
Sleep 2s
Escape

# Open sector map
Type "m"
Sleep 2s
Escape

# Watch battle play out (skip travel to get there faster)
Type " "
Sleep 15s

# Watch raid
Sleep 10s

# Open stats
Tab
Sleep 3s
Escape

# Watch more travel
Sleep 5s

# Quit
Type "q"
Sleep 1s
