<script setup>
import { computed, onMounted, ref } from 'vue'
import { screens } from './screens.js'
import { getLatestRelease, repo } from './releases.js'
import logo from '../assets/captureport.svg'
import InstallCommand from './InstallCommand.vue'

const selected = ref(0)
const screen = computed(() => screens[selected.value])
const release = ref(null)
const downloadGroups = computed(() => ['Distribution packages', 'Portable downloads'].map(label => ({
  label, downloads: release.value?.downloads.filter(asset => asset.group === label) || [],
})).filter(group => group.downloads.length))
const releaseStatus = ref('Checking the latest release…')
onMounted(async () => {
  try {
    release.value = await getLatestRelease()
    releaseStatus.value = 'Latest release · ' + release.value.version
  } catch {
    releaseStatus.value = 'Direct downloads unavailable. Find the latest packages on GitHub.'
  }
})
function move(event, index) {
  const directions = { ArrowRight: 1, ArrowLeft: -1 }
  if (!(event.key in directions) && !['Home', 'End'].includes(event.key)) return
  event.preventDefault()
  selected.value = event.key === 'Home' ? 0 : event.key === 'End' ? screens.length - 1
    : (index + directions[event.key] + screens.length) % screens.length
  document.getElementById('tab-' + selected.value).focus()
}
</script>

<template>
  <a class="skip" href="#main">Skip to content</a>
  <header class="header">
    <nav class="nav wrap" aria-label="Main navigation">
      <a class="brand" href="#" aria-label="CapturePort home"><img :src="logo" alt="" width="30" height="30" /><span>CapturePort</span></a>
      <div class="nav-links"><a href="#experience">Explore</a><a href="#care">Why CapturePort</a><a class="button small" href="#download">Get CapturePort</a><a class="github-link" :href="repo" aria-label="CapturePort on GitHub" title="View on GitHub"><svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 .75a11.25 11.25 0 0 0-3.558 21.922c.563.104.768-.244.768-.542 0-.267-.01-.974-.015-1.912-3.13.68-3.79-1.51-3.79-1.51-.512-1.3-1.25-1.646-1.25-1.646-1.022-.699.078-.685.078-.685 1.13.08 1.725 1.16 1.725 1.16 1.004 1.72 2.634 1.224 3.275.936.102-.727.393-1.224.715-1.505-2.498-.284-5.124-1.249-5.124-5.562 0-1.23.44-2.234 1.16-3.022-.116-.285-.503-1.43.11-2.98 0 0 .945-.303 3.094 1.155A10.78 10.78 0 0 1 12 6.19c.956.005 1.918.129 2.817.379 2.148-1.458 3.091-1.155 3.091-1.155.615 1.55.228 2.695.112 2.98.722.788 1.158 1.792 1.158 3.022 0 4.324-2.63 5.275-5.136 5.553.404.35.764 1.042.764 2.1 0 1.516-.014 2.739-.014 3.11 0 .3.203.652.774.542A11.252 11.252 0 0 0 12 .75Z" /></svg></a></div>
    </nav>
  </header>

  <main id="main">
    <section class="hero wrap" aria-labelledby="hero-title">
      <h1 id="hero-title">From camera<br />to <span>yours.</span></h1>
      <p class="hero-copy">Every shoot deserves a safe landing.<br />Import your photos and videos. Keep your originals.<br class="desktop-break" /> Make the library your own.</p>
      <div class="actions"><a class="button" href="#download">Get CapturePort</a><a class="text-link" href="#experience">See it in action <svg viewBox="0 0 24 24" aria-hidden="true"><path d="m9 5 7 7-7 7" /></svg></a></div>
      <p class="platform">Native Linux app. Open source. Yours to keep.</p>
      <InstallCommand />
      <figure class="hero-window">
        <img :src="screens[0].image" :alt="screens[0].alt" width="1919" height="1044" fetchpriority="high" />
        <figcaption>CapturePort, with your next import in view.</figcaption>
      </figure>
    </section>

    <section id="experience" class="experience" aria-labelledby="experience-title">
      <div class="wrap">
        <div class="section-intro"><h2 id="experience-title">Less busywork.<br /><span>More next adventure.</span></h2><p>From the first look to the final folder, a considered workflow that puts you in control.</p></div>
        <div class="tabs" role="tablist" aria-label="Explore CapturePort screens">
          <button v-for="(item, index) in screens" :id="'tab-' + index" :key="item.label" role="tab" :aria-selected="selected === index" :aria-controls="'panel-' + index" :tabindex="selected === index ? 0 : -1" @click="selected = index" @keydown="move($event, index)">{{ item.label }}</button>
        </div>
        <div :id="'panel-' + selected" role="tabpanel" :aria-labelledby="'tab-' + selected" tabindex="0" class="demo-panel">
          <div class="demo-copy"><h3>{{ screen.title }}</h3><p>{{ screen.description }}</p></div>
          <figure class="demo-window">
            <img :src="screen.image" :alt="screen.alt" width="1920" height="1045" loading="lazy" />
            <figcaption><span>Actual application screenshot · {{ screen.label }}</span><a :href="screen.image" target="_blank" rel="noopener">View full size <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M7 17 17 7M7 7h10v10" /></svg></a></figcaption>
          </figure>
        </div>
      </div>
    </section>

    <section id="care" class="care" aria-labelledby="care-title">
      <div class="wrap">
        <h2 id="care-title">Made for the files<br />you can’t take twice.</h2>
        <p class="care-lead">The moment is irreplaceable.<br />The import should be predictable.</p>
        <div class="principles">
          <article><h3>Review first.<br /><span>Copy second.</span></h3><p>Exact destination paths, up front. Naming collisions are resolved in the plan, never with a silent overwrite.</p></article>
          <article><h3>A copy isn’t done<br /><span>until it’s verified.</span></h3><p>Copies stay under temporary names until verification succeeds. Interrupted work never masquerades as a finished import.</p></article>
          <article><h3>Your originals.<br /><span>Still original.</span></h3><p>Importing doesn’t move or delete your source files. Your camera or card stays yours to manage.</p></article>
        </div>
        <div class="ownership"><p>No proprietary library.<br /><strong>Just your files, in your folders.</strong></p><a :href="repo + '#where-your-data-lives'" class="light-link">Learn how CapturePort stores your data <svg viewBox="0 0 24 24" aria-hidden="true"><path d="m9 5 7 7-7 7" /></svg></a></div>
      </div>
    </section>

    <section id="download" class="download wrap" aria-labelledby="download-title">
      <img :src="logo" alt="" width="76" height="76" loading="lazy" />
      <h2 id="download-title">Your next shoot.<br /><span>Meet its new home.</span></h2>
      <p>CapturePort for Linux.<br />A focused importer. Not another photo catalog.</p>
      <InstallCommand />
      <p class="release-status" role="status">{{ releaseStatus }}</p>
      <div v-if="release" class="download-options">
        <section v-for="(group, index) in downloadGroups" :key="group.label" class="download-group" :aria-labelledby="'download-group-' + index">
          <h3 :id="'download-group-' + index">{{ group.label }}</h3>
          <ul class="release-downloads">
            <li v-for="asset in group.downloads" :key="asset.name">
              <a :href="asset.url" :title="asset.name" :aria-label="'Download ' + asset.label + ' for ' + asset.arch + ' (' + asset.format + ')'">
                <span class="package-info"><strong>{{ asset.label }}</strong><span class="package-meta">{{ asset.arch }} <span aria-hidden="true">·</span> {{ asset.format }}</span></span>
                <span class="package-action">Download <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 4v12m-5-5 5 5 5-5M5 17v3h14v-3" /></svg></span>
              </a>
            </li>
          </ul>
        </section>
      </div>
      <div class="actions download-actions"><a :class="release ? 'text-link' : 'button'" :href="repo + '/releases/latest'">{{ release ? 'Release notes & all assets' : 'Browse downloads on GitHub' }} <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M7 17 17 7M7 7h10v10" /></svg></a><a class="text-link" :href="repo">View on GitHub <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M7 17 17 7M7 7h10v10" /></svg></a></div>
      <p class="requirements">Linux x86_64 · Vulkan-capable graphics · libgphoto2 for cameras<br />Debian / Ubuntu, Arch, Fedora, and portable packages.<br /><a :href="repo + '#install'">Installation and runtime requirements</a></p>
    </section>
  </main>

  <footer class="footer wrap"><a class="brand" href="#"><img :src="logo" alt="" width="24" height="24" /><span>CapturePort</span></a><p>Keep the moment. Own the library.</p><div><a :href="repo + '/blob/main/LICENSE'">MIT license</a><a :href="repo + '/issues'">Support</a><a :href="repo">Source</a></div></footer>
</template>
