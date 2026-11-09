<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- SPDX-FileCopyrightText: 2026 Noktyros -->

# Security and privacy

RetroHatch reads user-selected ROM/save files and stores private local data.
Malformed input, filesystem traversal, unsafe file replacement or native audio/
window integration issues are worth reporting. Do not treat checksums as proof
that a file is trustworthy or as authentication.

Use GitHub private vulnerability reporting if the owner has enabled it. Otherwise
ask the maintainer for a private reporting channel without posting exploit
details or sensitive files. No security mailbox or response-time guarantee is
invented here. Repository access/visibility remains owner-controlled.

Never upload ROMs, firmware, private saves, user-data folders, tokens, absolute
personal paths or raw diagnostic captures to public issues. Describe the device,
application revision, Windows/toolchain version and a minimal synthetic reproducer
where possible. Sanitize any logs you explicitly choose to share.

Ordinary application data is local under `%LOCALAPPDATA%\RetroHatch`. The
application currently has no implemented cloud service, account system or ROM
upload mechanism. The development data-directory override must point outside
the checkout. Cargo/build tooling can access package registries independently
of the emulator. Back up your own saves before testing development builds.
