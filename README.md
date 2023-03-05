# bsptab-rs

This is essentially just a rust rewrite of [bsptab](https://github.com/albertored11/bsptab) with
changes that align much better with how I'd like to use it.

I found the original `bsptab` quite nice, but I kept finding myself wanting to:

- merge multiple `tabbed`s together all at once
- completely explode a `tabbed`
- immediately embed exactly one window to a `tabbed`

For these reasons, these are the exact features implemented in my version. The main draw for this version is the simple/intuitive interface:

* `bsptab-rs create [WID]...` takes a list of window ids, which all get combined into a single `tabbed -c -d` instance. Any existing tabbed instances get flattened out, so you never end up with nested `tabbed`s. This simple behavior enables all of the following:
    * `create <WID>` -- turning any window into a tabbed instance.
    * `create <WID0> <WID1>` -- if `WID0` and `WID1` are both normal windows, this creates a `tabbed` and adds both windows to it.
    * `create <WID> <TAB>` -- if `WID` is a normal window and `TAB` is a `tabbed`, this will add `WID` to it.
    * `create <TAB0> <TAB1>` -- if `TAB0` and `TAB1` are both `tabbed`s, this will merge both into one `tabbed`.
* `bsptab-rs detach <TAB>` takes the id of a `tabbed` window and detaches the currently focused window (reparenting it to the root).
    * `detach --all <TAB>` -- will instead detach all of the windows, deleting the tabbed instance under normal operation. Good for if you decide you no longer want some windows tabbed together, and would rather see them all at once.
    * no-op if the provided window id is not a `tabbed`.
* `bsptab-rs embed <WID>` first calls `create <WID>` and then creates a one-time listener for a new node being added to the bspwm tree. When a new node is created (i.e. the next opened window), it gets attached with `<WID>`.
    * `embed <WID> & <command>` turns `WID` into a `tabbed` and embeds the window opened by `command` with it. This is particularly useful for opening a new terminal in the same `tabbed`, but can work for anything that opens a window.
    * Do note that the listener applies to any node opened anywhere.
* For convenience, any window/tabbed id can also be "focused", which simply replaces it with the focused window at runtime. Most of my personal keybinds use this.

### Installation

Install [tabbed](https://tools.suckless.org/tabbed) and rust.

`cargo build --release` creates a binary at `target/release/bsptab-rs` which you can copy to somewhere in your path.

### Example keybindings for sxhkd

This is a (slightly edited/simplified) excerpt from my current sxhkdrc file.

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

### TODO

- [ ] Proper testing (how do I test a program like this properly?)
- [ ] Support for bspc node selectors by passing to `bspc query -N -n`
- [ ] Maintain tab ordering more consistently
- [ ] Parity with original bsptab (maybe not fully?)
