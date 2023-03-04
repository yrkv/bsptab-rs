# bsptab-rs

This is essentially just a rust rewrite of [bsptab](https://github.com/albertored11/bsptab) with
changes that align much better with how I'd like to use it.

### Example keybindings for sxhkd

This is a (slightly edited) excerpt from my current sxhkdrc file.

```
######################
# tabbed manipulation
# -------------------
# super + t             base chord for all tabbed manipulation
#   t                   (create) create new tabbed container on focused window
#   r                   (detach) remove focused window from tabbed container
#   shift + r           (detach --all) remove all windows from tabbed container
#   e                   (embed) attach next opened window to focused window
#   z                   (embed terminal) attach a terminal to focused window
#   {h,j,k,l}           (merge) merge focused tabbed/window with target tabbed/window
super + t; t
    bsptab-rs create focused
super + t; r
    bsptab-rs detach focused
super + t; shift + r
    bsptab-rs detach --all focused
super + t; e
    bsptab-rs embed focused
super + t; {super +, } z
    { , } bsptab-rs embed focused & alacritty
super + t; {h,j,k,l}
    bsptab-rs create focused $(bspc query -N -n {west,south,north,east})
```
