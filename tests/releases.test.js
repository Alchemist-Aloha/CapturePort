import { test } from 'node:test'
import assert from 'node:assert/strict'
import { getLatestRelease, repo } from '../src/releases.js'

test('latest release provides trusted package links and rejects unavailable releases', async () => {
  const asset = name => ({ name, browser_download_url: `${repo}/releases/download/v1/${name}` })
  const result = await getLatestRelease(async url => {
    assert.equal(url, 'https://api.github.com/repos/Alchemist-Aloha/CapturePort/releases/latest')
    return { ok: true, json: async () => ({ tag_name: 'v1', assets: [
      asset('captureport-linux-x86_64'), asset('captureport-1-x86_64.pkg.tar.zst'),
      asset('captureport-1-x86_64-portable.tar.gz'), asset('captureport_1_amd64.deb'),
      asset('captureport-1.x86_64.rpm'), asset('captureport-debug-1.pkg.tar.zst'),
      asset('SHA256SUMS'), { name: 'unsafe.deb', browser_download_url: 'https://example.com/file' },
      { name: null }, null,
    ] }) }
  })
  assert.equal(result.version, 'v1')
  assert.deepEqual(result.downloads.map(item => item.label), ['Debian / Ubuntu', 'Fedora', 'Arch Linux', 'Portable archive', 'Linux binary'])
  assert.equal(result.downloads[0].url, asset('captureport_1_amd64.deb').browser_download_url)
  assert.ok(result.downloads.every(item => item.arch === 'x86_64'))
  assert.deepEqual(result.downloads.map(item => item.group), ['Distribution packages', 'Distribution packages', 'Distribution packages', 'Portable downloads', 'Portable downloads'])
  assert.deepEqual(result.downloads.map(item => item.format), ['.deb', '.rpm', '.pkg.tar.zst', '.tar.gz', 'Binary'])
  const armRelease = await getLatestRelease(async () => ({ ok: true, json: async () => ({ tag_name: 'v1', assets: [asset('captureport_1_arm64.deb'), asset('captureport_1_amd64.deb'), asset('captureport-linux-aarch64')] }) }))
  assert.deepEqual(armRelease.downloads.map(item => item.arch), ['arm64', 'x86_64', 'arm64'])
  await assert.rejects(getLatestRelease(async () => ({ ok: false })))
  for (const data of [{}, { tag_name: 'v1', assets: [] }]) {
    await assert.rejects(getLatestRelease(async () => ({ ok: true, json: async () => data })))
  }
  await assert.rejects(getLatestRelease(async () => { throw new Error('Network error') }))
})
