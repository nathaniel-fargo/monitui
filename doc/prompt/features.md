Hey let's work on this. A few things. We should have a `monitui --everywhere` option that opens up the
  monitui in a new terminal window on every active monitor, in a floating hyprland window. Running `super +
  shift + t` opens up btop in this floating fashion. When closing, it should kill all of these other
  processes. We had a seperate `all` thing that was trying to do this and failed, in case you bump into it.
  Other bug: shift + hjkl works but shift + left/right/up/down does not.

  If you wanna get crazy, we just added selecting with most on everything, but it would be super duper sick
  if you could drag monitors around
