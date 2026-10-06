export const repo = 'https://github.com/Alchemist-Aloha/CapturePort'

export async function getLatestRelease(request = fetch) {
  const response = await request('https://api.github.com/repos/Alchemist-Aloha/CapturePort/releases/latest', {
    headers: { Accept: 'application/vnd.github+json' },
    signal: AbortSignal.timeout(10000),
  })
  if (!response.ok) throw new Error('Release unavailable')
  const release = await response.json()
  if (typeof release.tag_name !== 'string' || !Array.isArray(release.assets)) throw new Error('Invalid release')
  const packages = [
    ['Debian / Ubuntu', /\.deb$/, 'Distribution packages', '.deb'],
    ['Fedora', /\.rpm$/, 'Distribution packages', '.rpm'],
    ['Arch Linux', /\.pkg\.tar\.zst$/, 'Distribution packages', '.pkg.tar.zst'],
    ['Portable archive', /-portable\.tar\.gz$/, 'Portable downloads', '.tar.gz'],
    ['Linux binary', /^captureport-linux-/, 'Portable downloads', 'Binary'],
  ]
  const downloads = release.assets.flatMap(asset => {
    if (!asset || typeof asset.name !== 'string' || asset.name.includes('-debug-') ||
        typeof asset.browser_download_url !== 'string' ||
        !asset.browser_download_url.startsWith(repo + '/releases/download/')) return []
    const type = packages.find(([, pattern]) => pattern.test(asset.name))
    if (!type) return []
    const architecture = asset.name.match(/(?:^|[-_.])(x86_64|amd64|aarch64|arm64|armv7\w*|i[3-6]86)(?=[-_.]|$)/)?.[1]
    const arch = architecture === 'amd64' ? 'x86_64' : architecture === 'aarch64' ? 'arm64' : architecture || 'See filename'
    return [{ name: asset.name, label: type[0], group: type[2], format: type[3], arch, url: asset.browser_download_url }]
  }).sort((a, b) => packages.findIndex(([label]) => label === a.label) - packages.findIndex(([label]) => label === b.label) || a.arch.localeCompare(b.arch) || a.name.localeCompare(b.name))
  if (!downloads.length) throw new Error('No download packages')
  return { version: release.tag_name, downloads }
}
