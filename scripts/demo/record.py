#!/usr/bin/env python3
"""Record the README demo: drive a private Herdr session in a pseudo-terminal
with scripted keystrokes and write an asciicast, then render it.

    ./scripts/demo/make-sample-repo.sh /tmp/sample-app
    ./scripts/demo/record.py /tmp/sample-app demo.cast
    agg --theme github-dark --font-size 14 --idle-time-limit 3 demo.cast assets/demo.gif
    ffmpeg -i assets/demo.gif -pix_fmt yuv420p -vf "scale=trunc(iw/2)*2:trunc(ih/2)*2,fps=15" assets/demo.mp4

Needs Herdr with this plugin linked or installed, fish, and a stopped or
absent `fingers-demo` session. The session is stopped again afterwards.
"""
import fcntl
import json
import os
import pty
import select
import struct
import subprocess
import sys
import termios
import time

HERE = os.path.dirname(os.path.abspath(__file__))
COLS, ROWS = 104, 26
SESSION = "fingers-demo"
PREFIX = "\x02"  # ctrl+b

repo = sys.argv[1] if len(sys.argv) > 1 else sys.exit(__doc__)
out_path = sys.argv[2] if len(sys.argv) > 2 else "demo.cast"

os.makedirs(os.path.join(HERE, "xdg", "fish"), exist_ok=True)
os.makedirs(os.path.join(HERE, "xdg-data"), exist_ok=True)
with open(os.path.join(HERE, "fish", "config.fish")) as source, open(
    os.path.join(HERE, "xdg", "fish", "config.fish"), "w"
) as target:
    target.write(source.read())

env = dict(os.environ)
for name in list(env):
    if name.startswith("HERDR_"):
        del env[name]
env.update(
    HERDR_CONFIG_PATH=os.path.join(HERE, "herdr-config.toml"),
    SHELL=os.path.join(HERE, "shell.sh"),
    TERM="xterm-256color",
    COLORTERM="truecolor",
    LANG="en_US.UTF-8",
)

def attach():
    """Starts a Herdr client (and the session's server if needed) in a PTY."""
    pid, fd = pty.fork()
    if pid == 0:
        os.chdir(repo)
        os.execvpe("herdr", ["herdr", "--session", SESSION], env)
    fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", ROWS, COLS, 0, 0))
    return pid, fd


def drain(fd, seconds, sink=None):
    """Reads the PTY for `seconds`, appending to the cast when `sink` is set."""
    end = time.time() + seconds
    while True:
        remaining = end - time.time()
        if remaining <= 0:
            return
        ready, _, _ = select.select([fd], [], [], min(remaining, 0.05))
        if ready:
            try:
                data = os.read(fd, 65536)
            except OSError:
                return
            if not data:
                return
            if sink is not None:
                sink(data)


# Warm-up, off camera: a first attach dismisses Herdr's welcome screen when
# it shows, clears the pane and leaves it on a single fresh prompt.
warm_pid, warm_fd = attach()
drain(warm_fd, 2.5)
os.write(warm_fd, b"\r")
drain(warm_fd, 0.8)
os.write(warm_fd, b"clear\r")
drain(warm_fd, 0.8)
os.write(warm_fd, PREFIX.encode())
drain(warm_fd, 0.2)
os.write(warm_fd, b"q")
drain(warm_fd, 1.0)
os.waitpid(warm_pid, 0)

pid, fd = attach()
start = time.time()
cast = open(out_path, "w")
header = {"version": 2, "width": COLS, "height": ROWS, "timestamp": int(start), "title": "herdr-fingers"}
cast.write(json.dumps(header) + "\n")


def pump(seconds):
    drain(
        fd,
        seconds,
        lambda data: cast.write(
            json.dumps([round(time.time() - start, 4), "o", data.decode("utf-8", "replace")]) + "\n"
        ),
    )


def key(data, pause=0.0):
    os.write(fd, data.encode())
    if pause:
        pump(pause)


def type_text(text, per_char=0.06, pause=0.4):
    for ch in text:
        key(ch, per_char)
    pump(pause)


def enter(pause=1.0):
    key("\r", pause)


def fingers(pause=1.2):
    key(PREFIX, 0.25)
    key("f", pause)


# Hints below assume the qwerty layout and this exact screen content: the
# bottom-most match gets "a", then "s", "d", "f", "w", "e" going up.
pump(2.0)  # the client draws the existing pane
type_text("git status"); enter(1.4)
fingers(1.6); key("s", 1.3)  # copy src/api/router.rs
type_text("nvim ", pause=0.5)
fingers(1.6); key("D", 1.4)  # Shift: paste docs/deploy.md
pump(0.6); key("\x15", 0.4)  # ctrl+u clears the prompt
type_text("git log --oneline -3"); enter(1.2)
type_text("git diff ", pause=0.5)
fingers(1.5); key("\t", 0.9); key("S", 0.9); key("D", 0.9); key("\t", 1.6)  # multi-select two SHAs, paste
pump(1.2); key("\x15", 0.3)
type_text("cat README.md"); enter(1.0)
fingers(2.2); key("\x1b", 1.0)  # show the URL hints, leave
pump(1.5)
key(PREFIX, 0.2); key("q", 1.0)  # detach
pump(0.8)
cast.close()
try:
    os.waitpid(pid, 0)
except ChildProcessError:
    pass
subprocess.run(["herdr", "--session", SESSION, "server", "stop"], env=env, capture_output=True)
subprocess.run(["herdr", "session", "delete", SESSION], env=env, capture_output=True)
print("wrote", out_path)
