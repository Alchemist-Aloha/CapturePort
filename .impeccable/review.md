# Website verification

Reviewed the actual full-page render at desktop 1440×1000 and mobile 390×844.
The page carries the requested Apple-inspired pacing, large branded type,
original app screenshots, and clear exploration and download actions.
Mobile screenshots remain uncropped, with full-size original links.

Browser checks: no horizontal overflow or JavaScript errors; all five screenshots
load; clicking each selector changes its panel; ArrowRight, Home, and End update
selection and keyboard focus. Reduced-motion mode was used for stable captures.
The Vue production build and native Node asset test pass. The mechanical design
detector returned no findings. npm reported no dependency vulnerabilities.

Verdict: ready for static hosting. Visual review was performed in-thread;
no independent reviewer was delegated. GitHub Pages deployment requires pushing
the website branch and selecting GitHub Actions in the repository's Pages settings.
