# mrss
Multi RSS Reader

Press `?` in the terminal UI to view available key bindings.

## Configuration

Configuration is stored in a platform-specific directory:

- Windows: `%AppData%\\rssq\\config.toml`
- Unix: `$XDG_CONFIG_HOME/rssq/config.toml` (defaults to `~/.config/rssq/config.toml`)

The file is created on first run with default settings:

```toml
[ui]
theme = "dark"
unread_only = true

[opener]
command = "xdg-open" # platform specific default

[keys]
quit = "q"
open = "o"
refresh = "r"
```
