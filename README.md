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
* `bsptab-rs transfer <WID0> <WID1>` is equivalent to `attach` in the older bsptab implementation, just much faster. Quoting, "Attach window <wid0> to tabbed container <wid1>. If <wid0> is a tabbed container, detach the active window and attach it to the new container. If <wid1> is not a tabbed container, call create <wid1> first."
    * This is more like i3-style tabs.
    * Due to bugs in `tabbed`, this has to completely rip apart the first tabbed and remake it. 
* For convenience, any window/tabbed id can also be a bspc node\_sel, which simply calls `bspc query -N -n <node_sel>` at runtime to get the correct window id. This means that you can use strings like "focused" or "west" when using `bsptab-rs`.

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
#   {h,j,k,l}           (create) merge focused tabbed/window with target tabbed/window
#   arrows              (transfer) attach focused window with target tabbed/window

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
    bsptab-rs create focused {west,south,north,east}
super + t; {Left,Down,Up,Right}
    bsptab-rs transfer focused {west,south,north,east}
```

### Demo

- Combine two windows into a single `tabbed`
- Open a new window as a tab

[demo1.webm](https://user-images.githubusercontent.com/11140316/223055843-8e64e3a6-cfd5-41e4-a456-2c9394ea8280.webm)



- Merge multiple `tabbed`s together
- Detach one window

[demo2.webm](https://user-images.githubusercontent.com/11140316/223056125-db92b8eb-8db9-4b54-9e83-ae590ba843b7.webm)


- Transfer from one `tabbed` to another
- Detach all windows from a `tabbed`

[demo3.webm](https://user-images.githubusercontent.com/11140316/223056151-6b444fb3-fbdc-4702-b81d-e16f59fe1bc9.webm)



### TODO

- [ ] Proper testing (how do I test a program like this properly?)
- [ ] Better error handling
- [x] Support for bspc node selectors by passing to `bspc query -N -n`
    - [ ] Would it be cleaner/faster to `send` to the socket directly instead of calling a `bspc` command?
- [ ] Maintain tab ordering more consistently
- [ ] Parity with original bsptab (maybe not fully?)
    - [x] attach
    - [ ] autoattach
