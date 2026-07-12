# Homebrew ships a vendor hook that auto-activates mise before config.fish.
# Disable it so config.fish can establish the trusted config path first and
# activate mise exactly once.
set -gx MISE_FISH_AUTO_ACTIVATE 0
