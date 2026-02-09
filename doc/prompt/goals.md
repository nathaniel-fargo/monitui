The main goals:

  1. Have easy to use presets that configure everything
  (we should use 'p' to enter preset menu, and can
  either load or save presets)
  2. Configure monitor details. Position, resolution
  (not working last time I checked, might be fixed now).
  3. I'd love for some more sort of visual cues for how
  this is going. i.e. if we split the tui into two
  sections (either left/right or top/bottom depending on
   the tui sizing). One for a list of monitors, and the
  other for how they'll be positioned. I want you to get
   really creative for how this should work. Use the
  resolution and sizing to create box size
  approximations of them, and how they fit together. One
   should be able to swap to that other pane, and toggle
   through each of them and move them around with the
  `hjkl` keys, where you can offset the relative y and x
   coordinates of them off of eachother, but they should
   still always snap onto eachother. `shift` modifiers
  could snap the selected monitor to different locations
   (i.e. `shift-k` should move a monitor such that is
  above another monitor, and `shift-l` should move it
  such that it is right of another monitor). Using the
  unmodified keys should also swap them when it doesn't
  make sense to move them otherwise. Consider all three
  monitors left to right, having a monitor selected (by
  tabbing through them), and then pressing `h` or the
  left arrow key, should swap that monitor with the one
  to the left. Then they could press `j/k` (or up/down)
  to vertically shift it up and down a little bit. If
  they shift it all the way up, such that they don't
  share a vertical edge anymore, it should snap over and
   be on top of that other monitor. It should snap such
  that if it was on the left, their left edges are now
  lined up. Same thing should happen top to bottom. You
  should write up this framework in code and do
  sufficient testing on it, independent of the display
  code, that way we can test them seperately. I.e. you
  should generate monitor configurations, and show me
  them visually, as well as test the movement logic, and
   then tie them together afterwards. Let's just have
  movement via this other pane, and configuration in the
   first pane. When a monitor is selected in either
  pane, it's corresponding one should be lit up, and
  there should be a key to swap between panes.
  4. Implement the `apply` thing. If an apply isn't
  saved by pressing `space` or `y` within 10 seconds, it
   should be reverted. Applying presets shouldn't bypass
   this, it should just load the preset into the
  configuration panel.
  5. The most recent configuration that was saved by
  this tui should also be saved and recoverable by going
   into the preset options and selecting like 'most
  recent apply'. This is useful because sometimes
  workspaces get swapped around and it would be nice to
  restore them
  6. Make sure hidden monitors are shown. If you can do
  any probing on monitors that are accessible but not on
   that would be great. So far it seems like we've only
  been able to list monitors that are available, or
  every possible monitor, which includes non-connected
  ports and HEADLESS monitors, which doesn't make a lot
  of sense. If we have to include them, at least put
  them at the bottom, and when loaded in, put them in
  their last used position.
  7. test test test
  8. make it pretty
  9. I'm considering publishing this so it better be
  good.
