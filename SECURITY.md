# Security policy

herdr-fingers reads what is on your screen and puts pieces of it on the
clipboard, into your pane, or into a command you configured. A flaw in how it
reads, matches or hands that text over is worth a quiet report, not a public
issue.

## Supported versions

Only the latest release receives fixes. Reinstall with
`herdr plugin install nathan-poncet/herdr-fingers` to get them.

## Reporting a vulnerability

Use GitHub's private vulnerability reporting:
**Security → Report a vulnerability** on
[github.com/nathan-poncet/herdr-fingers](https://github.com/nathan-poncet/herdr-fingers/security/advisories/new).
Nothing you write there is visible to anyone but the maintainer until a fix
is out.

Please include what you observed, how to reproduce it, and the herdr-fingers,
Herdr and OS versions. You will get an acknowledgement within seven days, and
the fix ships in the next release with credit to you unless you prefer
otherwise.

## What counts

- Screen content reaching anything other than the clipboard, the source
  pane, or the command you configured for that modifier.
- A crafted screen dump that makes the overlay run something, hang, or
  crash the Herdr session.
- A `:paste:` or shell action receiving text you did not pick.

## What does not

- The plugin reading the visible screen of the pane you invoked it from:
  that is what it is for, and it only reads while the overlay is open.
- The terminal you attach from receiving the copied text over OSC 52: Herdr
  forwards it, and your terminal decides whether to accept it.
- Custom actions doing whatever the command line you wrote does.
