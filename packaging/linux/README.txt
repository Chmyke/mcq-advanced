MCQ Advanced for Linux (x86_64)

  mcq-studio   the teacher's editor: write, publish and correct papers
  mcq-player   the student's program: take a paper and save the copy

Install for your user only, without root:

  sh install.sh

Both programs then appear in your applications menu. Remove them with: sh uninstall.sh

They need the WebKitGTK 4.1 and GTK 3 libraries, already present on most desktops.
If a program does not start, install them:

  Debian, Ubuntu:  sudo apt install libwebkit2gtk-4.1-0 libgtk-3-0 libxdo3
  Fedora:          sudo dnf install webkit2gtk4.1 gtk3 libxdo

Built on Ubuntu 22.04: works on Ubuntu 22.04, Debian 12, Fedora 37 and later.

MCQ Advanced is free software under the GNU General Public License, version 3 (LICENSE.txt).
The libraries it includes are listed with their licenses in THIRD-PARTY-NOTICES.txt.
Source code: https://github.com/Chmyke/mcq-advanced
