export const screens = [
  {
    label: 'Browse',
    title: 'The whole shoot. At a glance.',
    description: 'Browse photos and videos together. Review capture times and import status, then select exactly what comes with you.',
    image: new URL('../assets/captureport_browse.png', import.meta.url).href,
    alt: 'CapturePort media browser showing date-grouped photographs, a RAW file, a video, and selection controls.',
  },
  {
    label: 'Review',
    title: 'Know where every file will land.',
    description: 'See the final destination paths before copying starts. One import plan powers both the preview and the actual import.',
    image: new URL('../assets/captureport_review.png', import.meta.url).href,
    alt: 'CapturePort Review screen displaying the planned import destinations before confirmation.',
  },
  {
    label: 'Organize',
    title: 'Your folders. Your way.',
    description: 'Choose separate destinations for photos and videos. Save presets for your workflow, with verification and backup options.',
    image: new URL('../assets/captureport_setting.png', import.meta.url).href,
    alt: 'CapturePort Settings screen with import configuration and destination controls.',
  },
  {
    label: 'Rename',
    title: 'Give every session a name.',
    description: 'Group captures into shooting sessions and name them in the browser. Use session names in your folder templates.',
    image: new URL('../assets/captureport_rename.png', import.meta.url).href,
    alt: 'CapturePort session renaming interface within the media browser.',
  },
  {
    label: 'Templates',
    title: 'A little structure. A lot of order.',
    description: 'Build folder and filename rules from dates, camera metadata, and sequence numbers. Clickable segments and live examples make it practical.',
    image: new URL('../assets/captureport_segments.png', import.meta.url).href,
    alt: 'CapturePort template editor showing available filename and folder segments.',
  },
]
