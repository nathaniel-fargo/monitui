# monitui

> *Because dragging windows around `hyprctl` commands is so 2023.*

A delightfully minimal TUI for wrangling your Hyprland monitors. Built with Rust and way too much coffee.

![Demo](media/demo.mp4)

## What's This?

You know that moment when you plug in your laptop at your desk and all your windows scatter across monitors like startled pigeons? Yeah, we fixed that.

`monitui` is a terminal UI that makes Hyprland monitor configuration actually *pleasant*. Move monitors around with `hjkl`, drag them with your mouse, save presets, and generally feel like a wizard.

## Features That Spark Joy ✨

- **Visual canvas** - See your monitors laid out spatially (not just a config file!)
- **Keyboard shortcuts** - `hjkl` to move, `Shift+HJKL` to snap to edges, arrow keys work too
- **Mouse support** - Because sometimes you just want to drag things
- **Smart snapping** - Monitors auto-align and never overlap (we're not animals)
- **Presets** - Save your desk/couch/coffee-shop setups and switch instantly
- **Live preview** - See changes before you apply them
- **Safety net** - 10-second confirmation window (for when you inevitably break everything)

![Three monitors](media/three-monitors.png)
*Managing three monitors without losing your mind*

## Installation

```bash
cargo install --path .
```

Or if you're fancy:

```bash
cargo build --release
sudo cp target/release/monitui /usr/local/bin/
```

## Usage

Just run it:

```bash
monitui
```

### Keybindings

| Key | Action |
|-----|--------|
| `hjkl` / arrows | Move selected monitor |
| `Shift+HJKL` / `Shift+arrows` | Snap to far edge |
| `Tab` / `Shift+Tab` | Select monitor |
| `1-9` | Assign workspace |
| `W` | Clear workspace assignments |
| `d` / `e` | Disable / enable monitor |
| `r` | Cycle resolution |
| `s` | Cycle scale |
| `+` / `-` | Adjust scale |
| `p` | Presets menu (press `1-9` to load, `s` to save) |
| `a` | Apply configuration |
| `q` / `Esc` | Quit |

You can also click on monitors with your mouse like it's the future.

![Two monitors](media/two-monitors.png)
*Your desk setup, but make it TUI*

## Presets

Hit `p` to open the preset menu. Your configs live in `~/.config/monitui/presets/`.

Press number keys `1-9` to instantly load a preset. No more typing out the same `hyprctl` incantations every morning.

![Ideal setup](media/ideal-setup.png)
*The dream: all your monitors exactly where you want them*

## Pro Tips

- **Drag monitors** - Click and drag on the canvas to rearrange (it feels magical)
- **Auto-snap** - Monitors automatically snap together when you get close
- **No overlaps** - The layout engine prevents monitors from living inside each other (we've all been there)
- **Safety first** - When you apply changes, you get 10 seconds to confirm. If you break everything, it auto-reverts.

## Why Does This Exist?

Look, we love `hyprctl`. It's powerful. But have you ever tried to mentally calculate monitor positions while staring at a JSON blob? That's a fast track to madness.

`monitui` gives you a visual representation of your monitors and lets you move them around like a normal human being. You can even use your mouse! Revolutionary, we know.

## Requirements

- Hyprland (obviously)
- A terminal emulator
- At least one monitor (technically optional but highly recommended)

## Contributing

Found a bug? Have an idea? PRs welcome! This started as a "quick afternoon project" and spiraled into a full monitor management suite. Such is life.

## License

MIT - Go wild, have fun, don't sue us if you drop a monitor while using this.

---

*Built with [ratatui](https://github.com/ratatui-org/ratatui), love, and an unhealthy number of external displays.*
