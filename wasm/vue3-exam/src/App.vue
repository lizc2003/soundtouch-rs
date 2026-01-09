<template>
  <div id="app">
    <nav class="navbar">
      <h2 @click="navigateHome" style="cursor: pointer;">SoundTouch WASM Examples</h2>
      <div class="nav-links">
        <router-link to="/" class="nav-link">Home</router-link>
        <router-link to="/simple" class="nav-link">Simple Demo</router-link>
        <router-link to="/worklet" class="nav-link">AudioWorklet Demo</router-link>
      </div>
    </nav>
    <div class="content">
      <router-view />
    </div>
  </div>
</template>

<script setup>
import { useRouter } from 'vue-router'

const router = useRouter()

function navigateHome() {
  router.push('/')
}
</script>

<style>
* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

body {
  font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  min-height: 100vh;
  padding: 20px;
}

#app {
  max-width: 1200px;
  margin: 0 auto;
}

.navbar {
  background: white;
  border-radius: 10px;
  padding: 20px 30px;
  margin-bottom: 20px;
  box-shadow: 0 2px 10px rgba(0,0,0,0.1);
  display: flex;
  justify-content: space-between;
  align-items: center;
  flex-wrap: wrap;
}

.navbar h2 {
  color: #667eea;
  margin-right: 20px;
}

.nav-links {
  display: flex;
  gap: 15px;
}

.nav-link {
  padding: 10px 20px;
  border-radius: 5px;
  text-decoration: none;
  color: #555;
  font-weight: 500;
  transition: all 0.3s;
  border: 2px solid transparent;
}

.nav-link:hover {
  background: #f0f0f0;
}

.nav-link.router-link-active {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: white;
  border-color: #667eea;
}

.content {
  padding: 20px 0;
}
</style>
