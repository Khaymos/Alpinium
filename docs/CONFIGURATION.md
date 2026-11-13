# Alpinium configuration

Alpinium uses a Sway-inspired line-oriented configuration file. Blank lines and lines whose first non-whitespace character is # are ignored. Directives are case-sensitive. Inline comments and include directives are not supported by the first parser milestone.

## Lookup order

1. Explicit --config PATH command-line argument.
2. $XDG_CONFIG_HOME/alpinium/config, if XDG_CONFIG_HOME is an absolute path.
3. ~/.config/alpinium/config.
4. /etc/alpinium/config.
5. Built-in development defaults if no file exists.

The first existing file is loaded as a complete config. Configs are not merged in this milestone. If an explicit path is supplied but does not exist or cannot be parsed, Alpinium exits with an error instead of silently selecting another file.

## Directives

| Directive | Values | Default | Meaning |
| --- | --- | --- | --- |
| mode | tiling, floating | tiling | Selects the global window-management mode for the whole session. |
| animations | enabled, disabled | enabled | Enables or disables compositor animations. |
| animation_duration | milliseconds, e.g. 200ms | 200ms | Base animation duration. |
| gaps inner | non-negative integer | 5 | Inner window gap in pixels. |
| gaps outer | non-negative integer | 5 | Outer screen gap in pixels. |
| corner_radius | non-negative integer | 8 | Window corner radius in pixels. |
| shadows | enabled, disabled | enabled | Enables or disables window shadows. |

## Examples

### Tiling session

~~~conf
mode tiling
animations enabled
animation_duration 200ms
gaps inner 5
gaps outer 5
corner_radius 8
shadows enabled
~~~

### Floating session

~~~conf
mode floating
animations enabled
animation_duration 250ms
gaps inner 0
gaps outer 0
corner_radius 10
shadows enabled
~~~

The two modes are selected at configuration load time. They are implemented by the same Alpinium compositor binary and are not separate applications.

## Validation

~~~sh
alpinium --check-config
alpinium --print-config
alpinium --config /path/to/config --check-config
~~~

The parser reports the line number for malformed or unknown directives.
