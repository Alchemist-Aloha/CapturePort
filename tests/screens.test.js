import { test } from 'node:test'
import assert from 'node:assert/strict'
import { existsSync, readFileSync } from 'node:fs'
import { screens } from '../src/screens.js'

test('walkthrough screens have unique labels, descriptions and real local images', () => {
  assert.equal(screens.length, 5)
  assert.equal(new Set(screens.map(screen => screen.label)).size, 5)
  for (const screen of screens) {
    assert.ok(screen.title && screen.description && screen.alt)
    assert.ok(existsSync(new URL(screen.image)))
    const bytes = readFileSync(new URL(screen.image))
    assert.equal(bytes.subarray(0, 4).toString(), 'RIFF')
    assert.equal(bytes.subarray(8, 12).toString(), 'WEBP')
  }
})
