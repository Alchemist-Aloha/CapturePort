# CapturePort website

Website-only orphan branch, independent of the Rust application's history.
A static Vue 3 showcase using the supplied CapturePort screenshots.
No server, router, analytics, or external font requests.

## Develop

Requires Node.js 22.12+ (Node 24 recommended).

```sh
npm ci
npm run dev
npm test
npm run build
npm run preview
```

## Host

Push the `website` branch. In repository **Settings → Pages**, select
**GitHub Actions** as the build source. The included workflow builds and
deploys `dist/`. No deployment has been performed locally.

You can also upload `dist/` to any static host. Relative asset URLs support
both a domain root and a repository subdirectory.

## Assets

The five WebP screenshots in `assets/` are lossless conversions of the app
repository's PNG assets, preserving their original pixels. They show the actual application, not a
functional browser version. The walkthrough switches between those screens;
**View full size** opens the original image.

The icon and self-hosted Outfit and Spectral fonts come from the app's bundled
assets. Font licenses are in `assets/fonts/`. Outfit intentionally carries
the app's recognizable voice into oversized marketing type; Spectral retains
its wordmark. Product claims follow the application's README and spec.

The application source, releases, and installation instructions remain on
[the main repository](https://github.com/Alchemist-Aloha/CapturePort).
