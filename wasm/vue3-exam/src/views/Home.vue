<template>
  <div class="home">
    <div class="hero">
      <h1>🎵 SoundTouch WASM Examples</h1>
      <p class="subtitle">Audio processing in the browser using WebAssembly</p>
    </div>

    <div class="examples">
      <div class="example-card" @click="navigateTo('/simple')">
        <div class="icon">🎼</div>
        <h2>Simple Demo</h2>
        <p>Load audio files, adjust tempo and pitch, and process audio offline</p>
        <ul>
          <li>Load WAV or MP3 files</li>
          <li>Adjust tempo (0.5x - 2.0x)</li>
          <li>Change pitch (-12 to +12 semitones)</li>
          <li>Process and compare audio</li>
          <li>Visualize waveforms</li>
        </ul>
        <button class="try-btn">Try Simple Demo →</button>
      </div>

      <div class="example-card" @click="navigateTo('/worklet')">
        <div class="icon">🎛️</div>
        <h2>AudioWorklet Demo</h2>
        <p>Real-time audio processing with dynamic parameter adjustment</p>
        <ul>
          <li>Process audio in real-time</li>
          <li>Multiple audio sources (file, oscillator, mic)</li>
          <li>Adjust parameters on-the-fly</li>
          <li>Low-latency processing</li>
          <li>AudioWorklet technology</li>
        </ul>
        <button class="try-btn">Try AudioWorklet Demo →</button>
      </div>
    </div>

    <div class="features">
      <h2>Features</h2>
      <div class="feature-grid">
        <div class="feature">
          <h3>⚡ Fast</h3>
          <p>WebAssembly provides near-native performance</p>
        </div>
        <div class="feature">
          <h3>🔧 Flexible</h3>
          <p>Adjust tempo without changing pitch, and vice versa</p>
        </div>
        <div class="feature">
          <h3>🎯 Precise</h3>
          <p>High-quality audio processing algorithms</p>
        </div>
        <div class="feature">
          <h3>🌐 Universal</h3>
          <p>Works in all modern browsers with WebAssembly support</p>
        </div>
      </div>
    </div>

    <div class="about">
      <h2>About SoundTouch</h2>
      <p>
        SoundTouch is an open-source audio processing library for changing the tempo, 
        pitch and playback rates of audio streams or audio files. This is a WebAssembly 
        port of the SoundTouch library, implemented in Rust.
      </p>
      <p>
        <a href="https://github.com/lizc2003/soundtouch-rs" target="_blank">
          View on GitHub →
        </a>
      </p>
    </div>
  </div>
</template>

<script setup>
import { useRouter } from 'vue-router'

const router = useRouter()

function navigateTo(path) {
  router.push(path)
}
</script>

<style scoped>
.home {
  max-width: 1000px;
  margin: 0 auto;
  padding: 20px;
}

.hero {
  text-align: center;
  padding: 60px 20px;
  background: white;
  border-radius: 15px;
  margin-bottom: 30px;
  box-shadow: 0 4px 20px rgba(0,0,0,0.1);
}

.hero h1 {
  font-size: 3em;
  margin-bottom: 10px;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
}

.subtitle {
  font-size: 1.3em;
  color: #666;
  margin-top: 10px;
}

.examples {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(400px, 1fr));
  gap: 30px;
  margin-bottom: 40px;
}

.example-card {
  background: white;
  border-radius: 15px;
  padding: 30px;
  box-shadow: 0 4px 20px rgba(0,0,0,0.1);
  cursor: pointer;
  transition: transform 0.3s, box-shadow 0.3s;
}

.example-card:hover {
  transform: translateY(-5px);
  box-shadow: 0 8px 30px rgba(102, 126, 234, 0.3);
}

.icon {
  font-size: 4em;
  margin-bottom: 15px;
}

.example-card h2 {
  color: #667eea;
  margin-bottom: 10px;
}

.example-card > p {
  color: #666;
  margin-bottom: 20px;
  font-size: 1.1em;
}

.example-card ul {
  list-style: none;
  padding: 0;
  margin-bottom: 20px;
}

.example-card li {
  padding: 8px 0;
  color: #555;
  border-bottom: 1px solid #f0f0f0;
}

.example-card li:before {
  content: "✓ ";
  color: #667eea;
  font-weight: bold;
  margin-right: 8px;
}

.try-btn {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: white;
  border: none;
  padding: 12px 30px;
  border-radius: 5px;
  cursor: pointer;
  font-size: 16px;
  width: 100%;
  transition: transform 0.2s;
}

.try-btn:hover {
  transform: scale(1.05);
}

.features {
  background: white;
  border-radius: 15px;
  padding: 40px;
  margin-bottom: 30px;
  box-shadow: 0 4px 20px rgba(0,0,0,0.1);
}

.features h2 {
  text-align: center;
  color: #667eea;
  margin-bottom: 30px;
  font-size: 2em;
}

.feature-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 30px;
}

.feature {
  text-align: center;
}

.feature h3 {
  font-size: 1.5em;
  margin-bottom: 10px;
}

.feature p {
  color: #666;
}

.about {
  background: white;
  border-radius: 15px;
  padding: 40px;
  box-shadow: 0 4px 20px rgba(0,0,0,0.1);
  text-align: center;
}

.about h2 {
  color: #667eea;
  margin-bottom: 20px;
  font-size: 2em;
}

.about p {
  color: #666;
  line-height: 1.8;
  font-size: 1.1em;
  margin-bottom: 15px;
}

.about a {
  color: #667eea;
  text-decoration: none;
  font-weight: bold;
  font-size: 1.1em;
}

.about a:hover {
  text-decoration: underline;
}

@media (max-width: 768px) {
  .examples {
    grid-template-columns: 1fr;
  }
  
  .hero h1 {
    font-size: 2em;
  }
  
  .feature-grid {
    grid-template-columns: 1fr;
  }
}
</style>

