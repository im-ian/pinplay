# Pinplay user journeys

1. As a terminal user, I want to open a YouTube or other web video in a tiny,
   always-on-top window so I can keep watching while working.
2. As a terminal user, I want to open a local media file using the same command
   so I do not need a separate workflow for downloaded videos.
3. As a viewer, I want to choose the initial size and screen corner so the video
   does not cover the part of the screen where I am working.
4. As a viewer, I want to set the start time, mute state, playback speed, volume,
   and loop behavior from the command line so playback starts exactly as wanted.
5. As a user setting up Pinplay, I want an offline `doctor` command that explains
   whether mpv, yt-dlp, and a JavaScript runtime are available.
6. As a security-conscious user, I want URLs and filenames to be passed as
   process arguments rather than shell text so an unusual source cannot execute
   another command.

## MVP acceptance criteria

- Syntax remains `pinplay <SOURCE> [OPTIONS]`; options may appear before or after
  the source.
- Local files and protocol URLs are accepted. Missing local paths are rejected
  before mpv is launched.
- The default window is 320x180, borderless, draggable, always on top, and placed
  in the bottom-right corner with a 24-pixel margin.
- `nano`, `tiny`, `small`, `medium`, and explicit `WIDTHxHEIGHT` sizes are
  supported; 96x54 must remain valid.
- Pinplay never invokes a shell. It inserts mpv's `--` delimiter before the exact
  source argument.
- `doctor --json` is deterministic and suitable for package-manager smoke tests.

