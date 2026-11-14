# Alpinium configuration

Alpinium uses a Sway-inspired, line-oriented configuration file. Blank lines and lines whose first non-whitespace character is # are ignored. Directives are case-sensitive. Inline comments and include directives are not supported.

## Lookup order

1. Explicit --config PATH command-line argument.
2. $XDG_CONFIG_HOME/alpinium/config, if XDG_CONFIG_HOME is an absolute path.
3. ~/.config/alpinium/config.
4. /etc/alpinium/config.
5. Built-in development defaults if no file exists.

The first existing file is loaded as a complete config. Configs are not merged. If an explicit path is supplied but does not exist or cannot be parsed, Alpinium exits with an error instead of silently selecting another file.

## Variables

Use `set` to define a reusable variable. Variable references are expanded in later directives, including key combinations and command strings.

~~~conf
set $mod Mod4
set $terminal alacritty
set $browser firefox

bindsym $mod+Return exec $terminal
bindsym $mod+w exec $browser
~~~

Variables are processed from top to bottom, so a variable must be defined before it is used. Variable names begin with a letter or underscore and may contain letters, numbers, and underscores. Both `$name` and `${name}` references are supported. Use `$$` for a literal dollar sign.

The default `$mod` is `Mod4` (Super/Windows), and `$terminal` defaults to `alacritty`.

## Directives

| Directive | Values | Default | Meaning |
| --- | --- | --- | --- |
| `set $name value` | variable name and value | `$mod Mod4`, `$terminal alacritty` | Defines a variable for later directives. |
| `bindsym combination action` | modifier/key combination and action | see defaults below | Adds a keyboard binding. The first explicit `bindsym` replaces the built-in binding set. |
| `mode` | `tiling`, `floating` | tiling | Selects global window management. |
| `animations` | `enabled`, `disabled` | enabled | Enables or disables compositor animations. |
| `animation_duration` | milliseconds, e.g. `200ms` | 200ms | Base animation duration. |
| `gaps inner` | non-negative integer | 5 | Inner window gap in pixels. |
| `gaps outer` | non-negative integer | 5 | Outer screen gap in pixels. |
| `corner_radius` | non-negative integer | 8 | Window corner radius in pixels. |
| `shadows` | `enabled`, `disabled` | enabled | Enables or disables window shadows. |

### Supported binding modifiers

- `Mod4` or `Super`: Super/Windows key.
- `Mod1` or `Alt`: Alt key.
- `Ctrl` or `Control`: Control key.
- `Shift`: Shift key.

Modifier names are case-insensitive. Key names use XKB keysym names, such as `Return`, `Tab`, `Escape`, `F1`, or letter keys such as `q`.

### Supported binding actions

| Action | Meaning |
| --- | --- |
| `exec COMMAND` | Runs COMMAND through the user's shell with Alpinium's Wayland socket in `WAYLAND_DISPLAY`. |
| `focus next` | Focuses the next mapped window. |
| `focus prev` | Focuses the previous mapped window. |
| `kill` | Sends a Wayland close request to the focused client. |
| `exit` | Exits Alpinium. |

## Example

~~~conf
set $mod Mod4
set $terminal alacritty
set $browser firefox

mode tiling
animations enabled
animation_duration 200ms
gaps inner 5
gaps outer 5
corner_radius 8
shadows enabled

bindsym $mod+Return exec $terminal
bindsym $mod+b exec $browser
bindsym $mod+Tab focus next
bindsym $mod+Shift+Tab focus prev
bindsym $mod+q kill
bindsym $mod+Shift+e exit
~~~

The first explicit `bindsym` replaces the built-in binding set, so a custom config should include every binding it wants to retain. If no `bindsym` directives are present, Alpinium supplies the built-in bindings using the configured `$mod` value.

## Validation

~~~sh
alpinium --check-config
alpinium --print-config
alpinium --config /path/to/config --check-config
~~~

The parser reports line numbers for malformed directives, undefined variables, unsupported modifiers, duplicate keybindings, and unknown actions.
