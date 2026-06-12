# Blue Time

A simple Pomodoro timer for GNOME, written in Rust.

Blue Time lives in your top bar as a status indicator and follows the
Pomodoro technique: focused work sessions separated by short breaks, with a
long break after every few sessions.

## Features

- Focus / short break / long break sessions with configurable durations
- Start, pause, resume, skip and reset controls
- Status indicator in the GNOME top bar with live progress
- Desktop notifications and sound when a session ends
- Optional auto-start of the next session
- Runs in the background — closing the window keeps the timer going
- Daily statistics and session history
- Optional autostart on login
- Adapts automatically to the system light/dark theme (libadwaita)

## Requirements

- Fedora with GNOME (tested on GNOME Shell 50)
- The [AppIndicator and KStatusNotifierItem Support](https://extensions.gnome.org/extension/615/appindicator-support/)
  GNOME Shell extension, for the top bar indicator
- Rust (stable) and GTK 4 / libadwaita development libraries to build:

```sh
sudo dnf install gcc gtk4-devel libadwaita-devel
```

## Building

```sh
cargo build --release
```

## License

[MIT](LICENSE)
