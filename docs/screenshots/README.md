<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- SPDX-FileCopyrightText: 2026 Noctyros -->
# Maintainer-selected gallery

This is the public promotional gallery, not the application's capture folder.
The project owner chooses every published image, caption and video link.
Existing entries, filenames, ordering and captions must be preserved unless
explicitly authorized. Never automatically copy captures from Pictures here.

## Add a screenshot manually

1. Choose a screenshot you have permission to publish. Remove personal data;
   do not add ROMs, saves or unreviewed firmware artwork. A local capture is not
   automatically licensed for public use merely because RetroHatch made it.
2. Copy only that selected image here, with a descriptive filename. PNG, JPG,
   JPEG, WebP and GIF are allowed at this directory level; subfolders and videos
   remain excluded. Keep files reasonably small for mobile readers.
3. In the root README's USER-MANAGED SCREENSHOT GALLERY markers, append an entry
   like this, replacing the example filename with your actual file:

```html
<p align="center">
  <img src="docs/screenshots/angel.png" alt="Tamagotchi Angel running in RetroHatch" width="360">
  <br><em>Tamagotchi Angel (Angelgotchi)</em>
</p>
```

Repeat entries for other models. GitHub scales the image to the available width.
Keep earlier images/captions and confirm that the relative path exists before
committing. The public branding logo stays at docs/assets/retrohatchlogo.png;
resources/ remains local and ignored.

## Add a gameplay link manually

Upload a video yourself to an appropriate host or GitHub attachment service,
then add a normal link in the root README's USER-MANAGED GAMEPLAY VIDEOS section.
For an optional clickable thumbnail, select its image too and use:

```markdown
[![P's gameplay](docs/screenshots/ps-video-preview.png)](YOUR_ACTUAL_VIDEO_URL)
```

Replace both example values; do not paste a demonstration URL that does not
exist. GitHub does not reliably render arbitrary HTML video embeds. Large MP4
files stay outside Git unless the owner explicitly authorizes otherwise.

No screenshots or videos have been selected by this audit. These examples are
instructions inside code blocks, not rendered gallery placeholders or links.
