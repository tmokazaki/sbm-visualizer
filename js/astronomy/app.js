/**
 * Main Application Orchestrator for Astronomy 3D Visualizer.
 * Delegates all physics, integrations, tidal tensors, and Keplerian orbital telemetry to WebAssembly.
 */
import { AU_M, DAY_S, YEAR_S, getBodyEmoji } from './constants.js';
import { getCentricFrameConfig, isBodyRelevantToCentricMode, showCentricToast } from './centric_config.js';
import { initWasmEngine, createNBodyEngine } from './wasm_bridge.js';
import {
  createSpacetimeMesh,
  updateSpacetimeMesh,
  createShadowConeGroup,
  buildShadowConeMeshes,
  updateShadowConeTransform,
  createSatelliteReticleLayer,
  createGravitationalBasinsLayer,
  buildPlanetBasins,
  updatePlanetBasinPositions,
  createSpatialVectorGridLayer,
  createEquipotentialContoursLayer,
  buildEquipotentialContours,
  createSpaceProbeLayer
} from './layers.js';
import {
  spawnSwarm,
  stepSwarm,
  clearSwarm,
  getActiveSwarmEngine
} from './particles.js';
import {
  flyTo,
  openSatelliteInspector,
  updateSatelliteInspectorCard,
  openBodyPlaceCard,
  setupUIInteractions
} from './ui.js';

// Application State
export const state = {
  presetId: 'inner_solar_system_jupiter',
  bodies: [],
  time_s: 0.0,
  step_count: 0,
  integrator: 'yoshida4',
  enable_gr: false,
  isPlaying: true,
  timeMultiplier: 100,
  subSteps: 10,
  scaleMode: 'perceptual',
  sceneScale: 1.0,
  showTrails: true,
  showVectors: false,
  showForces: true,
  showSpacetime: true,
  showHillSpheres: true,
  showGrid: true,
  selectedBodyIndex: 2, // Earth default
  cameraFollowIndex: -1,
  selectedSatelliteId: null,
  isTrackingSatellite: false,
  isolateSelectedOrbit: false,
  hoveredSatelliteId: null,
  gravitationalFrameMode: 'auto',
  activeCentricBody: 'Earth',
  showShadowCone: true,
  showTestParticles: true,
  visVivaColoring: true,
  testParticles: [],
  sunlitCount: 0,
  umbraCount: 0,
  penumbraCount: 0
};

// Three.js Core Objects
let scene, camera, renderer, controls;
let wasmNBody = null;
let bodyMeshes = [];
let trailLines = [];
let hillSphereMeshes = [];
let eclipticGrid = null;
let spacetimeMesh = null;
let shadowConeGroup = null;
let gravitationalBasinsGroup = null;
let spatialVectorGridGroup = null;
let equipotentialContoursGroup = null;
let spaceProbeGroup = null;
let satelliteLayers = null;

export function toSceneCoords(physPos) {
  return new THREE.Vector3(
    physPos[0] * state.sceneScale,
    physPos[1] * state.sceneScale,
    physPos[2] * state.sceneScale
  );
}

export async function initApp() {
  console.log('🚀 Initializing SBM Astronomy Visualizer with WebAssembly Physics Engine...');
  await initWasmEngine();

  // 1. Setup Three.js Scene, Camera & Renderer
  const container = document.getElementById('canvas-container') || document.body;
  scene = new THREE.Scene();
  scene.background = new THREE.Color(0x020617);

  camera = new THREE.PerspectiveCamera(45, window.innerWidth / window.innerHeight, 0.01, 10000);
  camera.position.set(0, -32, 22);

  renderer = new THREE.WebGLRenderer({ antialias: true, alpha: false, powerPreference: 'high-performance' });
  renderer.setSize(window.innerWidth, window.innerHeight);
  renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
  renderer.shadowMap.enabled = false;
  container.appendChild(renderer.domElement);

  // 2. Camera Controls
  controls = new THREE.OrbitControls(camera, renderer.domElement);
  controls.enableDamping = true;
  controls.dampingFactor = 0.08;
  controls.maxDistance = 2000;
  controls.minDistance = 0.05;

  // 3. Lighting
  const ambLight = new THREE.AmbientLight(0xffffff, 0.55);
  scene.add(ambLight);
  const sunLight = new THREE.PointLight(0xffffff, 1.8, 0, 0);
  sunLight.position.set(0, 0, 0);
  scene.add(sunLight);

  // 4. Initialize Visual Layers
  spacetimeMesh = createSpacetimeMesh();
  scene.add(spacetimeMesh);

  shadowConeGroup = createShadowConeGroup();
  scene.add(shadowConeGroup);

  gravitationalBasinsGroup = createGravitationalBasinsLayer(scene);
  spatialVectorGridGroup = createSpatialVectorGridLayer(scene);
  equipotentialContoursGroup = createEquipotentialContoursLayer(scene);
  spaceProbeGroup = createSpaceProbeLayer(scene);
  satelliteLayers = createSatelliteReticleLayer(scene);

  // Ecliptic Polar Grid
  const gridHelper = new THREE.PolarGridHelper(35, 16, 8, 64, 0x38bdf8, 0x1e293b);
  gridHelper.rotation.x = Math.PI / 2;
  scene.add(gridHelper);
  eclipticGrid = gridHelper;

  // 5. Load Astronomical Preset into WebAssembly N-Body Engine
  loadPreset(state.presetId);

  // 6. UI Interactions & Event Listeners
  setupEventListeners();
  setupUIInteractions(state, {
    onCentricChange: (newFrame) => setCentricBody(newFrame),
    onFlyToSatellite: () => {
      const sat = state.testParticles.find(p => p.id === state.selectedSatelliteId);
      if (sat && sat.mesh) {
        flyTo(camera, controls, sat.mesh.position, 1.8, 1000);
      }
    },
    onDeselectSatellite: () => {
      state.selectedSatelliteId = null;
      state.isTrackingSatellite = false;
      state.isolateSelectedOrbit = false;
      if (satelliteLayers) {
        satelliteLayers.reticleGroup.visible = false;
        satelliteLayers.periapsisMarker.visible = false;
        satelliteLayers.apoapsisMarker.visible = false;
      }
    },
    onSelectBody: (idx) => {
      state.selectedBodyIndex = idx;
      openBodyPlaceCard(state.bodies[idx], state);
      if (bodyMeshes[idx]) {
        flyTo(camera, controls, bodyMeshes[idx].position, 3.5, 1000);
      }
    },
    onFlyToBody: (idx) => {
      if (bodyMeshes[idx]) {
        flyTo(camera, controls, bodyMeshes[idx].position, 3.5, 1000);
      }
    },
    onGenerateOrbits: (preset, count) => {
      spawnSwarm(scene, state.activeCentricBody || 'Earth', preset, count, state);
    },
    onClearOrbits: () => {
      clearSwarm(scene, state);
    },
    onSelectSatelliteById: (satId) => {
      const sat = state.testParticles.find(p => p.id === satId);
      if (sat) selectSatellite(sat);
    }
  });

  // 7. Window Resize Listener
  window.addEventListener('resize', onWindowResize);

  // 8. Start Animation Loop
  requestAnimationFrame(animate);
}

function loadPreset(presetId) {
  state.presetId = presetId;
  state.time_s = 0;
  state.step_count = 0;

  // Instantiate WebAssembly N-Body Engine
  wasmNBody = createNBodyEngine(presetId);
  syncBodiesFromWasm();

  // Determine characteristic distance scale
  let maxDist = 1.0;
  state.bodies.forEach(b => {
    const d = Math.hypot(b.pos[0], b.pos[1], b.pos[2]);
    if (d > maxDist) maxDist = d;
  });
  state.sceneScale = 30.0 / (maxDist || 1.0);

  rebuildSceneMeshes();

  // Default Centric Body: Earth or Sun
  const defaultCentric = state.bodies.some(b => b.name === 'Earth') ? 'Earth' : 'Sun';
  setCentricBody(defaultCentric);

  // Spawn Swarm Particles around active centric body (300 particles default)
  if (state.activeCentricBody !== 'Sun') {
    spawnSwarm(scene, state.activeCentricBody, 'cislunar', 300, state);
  }
}

function syncBodiesFromWasm() {
  if (!wasmNBody) return;
  const bodiesData = wasmNBody.get_bodies();
  state.bodies = bodiesData.map((b, idx) => ({
    id: idx,
    name: b.name,
    mass: b.mass_kg,
    radius: b.radius_m / 1e3, // km
    pos: [b.position_m[0], b.position_m[1], b.position_m[2]],
    vel: [b.velocity_mps[0], b.velocity_mps[1], b.velocity_mps[2]],
    color: getBodyColor(b.name, idx)
  }));
}

function getBodyColor(name, idx) {
  const palette = {
    Sun: '#fbbf24',
    Mercury: '#94a3b8',
    Venus: '#f59e0b',
    Earth: '#38bdf8',
    Moon: '#cbd5e1',
    Mars: '#ef4444',
    Jupiter: '#fb923c',
    Saturn: '#facc15',
    Uranus: '#2dd4bf',
    Neptune: '#60a5fa',
    Pluto: '#a855f7',
    Io: '#facc15',
    Europa: '#e2e8f0',
    Ganymede: '#94a3b8',
    Callisto: '#64748b'
  };
  return palette[name] || '#38bdf8';
}

function rebuildSceneMeshes() {
  // Clear old meshes
  bodyMeshes.forEach(m => scene.remove(m));
  bodyMeshes = [];
  trailLines.forEach(l => scene.remove(l));
  trailLines = [];

  // Create Celestial Body Meshes
  state.bodies.forEach((b, idx) => {
    let visualRadius = 0.28;
    if (state.scaleMode === 'perceptual') {
      if (b.name === 'Sun') visualRadius = 1.4;
      else if (b.name === 'Jupiter') visualRadius = 0.55;
      else if (b.name === 'Earth') visualRadius = 0.28;
      else if (b.name === 'Moon') visualRadius = 0.09;
      else if (b.name === 'Venus') visualRadius = 0.26;
      else if (b.name === 'Mars') visualRadius = 0.18;
    } else {
      visualRadius = Math.max(0.12, (b.radius || 6371) * 1e3 * state.sceneScale * 0.5);
    }

    const geom = new THREE.SphereGeometry(visualRadius, 32, 32);
    let mat;
    if (b.name === 'Sun') {
      mat = new THREE.MeshBasicMaterial({ color: new THREE.Color(b.color) });
    } else {
      mat = new THREE.MeshStandardMaterial({
        color: new THREE.Color(b.color),
        roughness: 0.4,
        metalness: 0.1
      });
    }

    const mesh = new THREE.Mesh(geom, mat);
    mesh.userData = { bodyIndex: idx, name: b.name };
    mesh.position.copy(toSceneCoords(b.pos));
    scene.add(mesh);
    bodyMeshes.push(mesh);

    // Orbital ring
    if (idx !== 0) {
      const dist = Math.hypot(b.pos[0], b.pos[1], b.pos[2]);
      const rSc = dist * state.sceneScale;
      if (rSc > 0.1) {
        const curve = new THREE.EllipseCurve(0, 0, rSc, rSc, 0, 2 * Math.PI, false, 0);
        const points = curve.getPoints(128);
        const ringGeo = new THREE.BufferGeometry().setFromPoints(points);
        const ringMat = new THREE.LineBasicMaterial({
          color: new THREE.Color(b.color),
          transparent: true,
          opacity: 0.35
        });
        const ringLine = new THREE.Line(ringGeo, ringMat);
        ringLine.rotation.x = 0;
        scene.add(ringLine);
        trailLines.push(ringLine);
      }
    }
  });

  // Basins
  buildPlanetBasins(gravitationalBasinsGroup, state.bodies, toSceneCoords, state.sceneScale);
}

export function setCentricBody(bodyName) {
  state.activeCentricBody = bodyName;
  const cfg = getCentricFrameConfig(bodyName);
  showCentricToast(cfg);

  // Omnibox chip
  const chipText = document.getElementById('centric-chip-text');
  if (chipText) {
    const modePrefix = state.gravitationalFrameMode === 'auto' ? 'Auto: ' : '';
    chipText.innerText = `${modePrefix}${cfg.frameName} (${bodyName})`;
  }

  // Dropdown
  const selFrame = document.getElementById('select-gravitational-frame');
  if (selFrame) selFrame.value = bodyName;

  // Rebuild shadow cone meshes for new anchor body
  buildShadowConeMeshes(shadowConeGroup, bodyName, state.scaleMode, state.sceneScale);

  // Equipotential contours
  const anchor = state.bodies.find(b => b.name === bodyName);
  if (anchor) {
    buildEquipotentialContours(equipotentialContoursGroup, anchor, toSceneCoords);
  }

  // Spawn Swarm Particles around anchor if not Sun
  if (bodyName !== 'Sun') {
    spawnSwarm(scene, bodyName, 'cislunar', 300, state);
  } else {
    clearSwarm(scene, state);
  }
}

function setupEventListeners() {
  // Preset selector
  const selPreset = document.getElementById('preset-selector') || document.getElementById('preset-select');
  if (selPreset) {
    selPreset.addEventListener('change', (e) => loadPreset(e.target.value));
  }

  // Play / Pause
  const btnPlay = document.getElementById('btn-play-pause') || document.getElementById('btn-play');
  if (btnPlay) {
    btnPlay.addEventListener('click', () => {
      state.isPlaying = !state.isPlaying;
      btnPlay.innerText = state.isPlaying ? 'Pause' : 'Play';
      btnPlay.classList.toggle('active', state.isPlaying);
    });
  }

  // Scale Mode Toggle
  const btnScale = document.getElementById('btn-scale-mode');
  if (btnScale) {
    btnScale.addEventListener('click', () => {
      state.scaleMode = state.scaleMode === 'perceptual' ? 'true' : 'perceptual';
      btnScale.innerText = `Scale: ${state.scaleMode === 'perceptual' ? 'Perceptual' : 'True 1:1'}`;
      rebuildSceneMeshes();
    });
  }

  // Reset Button
  const btnReset = document.getElementById('btn-reset');
  if (btnReset) {
    btnReset.addEventListener('click', () => {
      loadPreset(state.presetId);
    });
  }

  // Layer Checkboxes
  const bindCheckbox = (id, callback) => {
    const el = document.getElementById(id);
    if (el) el.addEventListener('change', (e) => callback(e.target.checked));
  };

  bindCheckbox('chk-layer-shadow-cone', (v) => {
    state.showShadowCone = v;
    if (shadowConeGroup) shadowConeGroup.visible = v && (state.activeCentricBody !== 'Sun');
  });

  bindCheckbox('chk-layer-test-particles', (v) => {
    state.showTestParticles = v;
  });

  bindCheckbox('chk-layer-visviva', (v) => {
    state.visVivaColoring = v;
  });

  bindCheckbox('chk-layer-basins', (v) => {
    if (gravitationalBasinsGroup) gravitationalBasinsGroup.visible = v;
  });

  // Layer Menu Particle Spawn Buttons
  const btnSwarm = document.getElementById('btn-gen-particles-swarm');
  if (btnSwarm) {
    btnSwarm.addEventListener('click', () => spawnSwarm(scene, state.activeCentricBody || 'Earth', 'swarm', 300, state));
  }
  const btnRings = document.getElementById('btn-gen-particles-rings');
  if (btnRings) {
    btnRings.addEventListener('click', () => spawnSwarm(scene, state.activeCentricBody || 'Earth', 'rings', 100, state));
  }
  const btnCislunar = document.getElementById('btn-gen-particles-cislunar');
  if (btnCislunar) {
    btnCislunar.addEventListener('click', () => spawnSwarm(scene, state.activeCentricBody || 'Earth', 'cislunar', 50, state));
  }
  const btnClearMenu = document.getElementById('btn-clear-particles-menu');
  if (btnClearMenu) {
    btnClearMenu.addEventListener('click', () => clearSwarm(scene, state));
  }

  // Integrator selector
  const selInt = document.getElementById('integrator-selector');
  if (selInt) {
    selInt.addEventListener('change', (e) => {
      state.integrator = e.target.value;
    });
  }

  // General Relativity Toggle
  const btnGr = document.getElementById('btn-toggle-gr');
  if (btnGr) {
    btnGr.addEventListener('click', () => {
      state.enable_gr = !state.enable_gr;
      btnGr.innerText = `1PN GR: ${state.enable_gr ? 'ON' : 'OFF'}`;
      btnGr.classList.toggle('active', state.enable_gr);
    });
  }

  // Time Scale Slider
  const timeSlider = document.getElementById('time-multiplier-slider');
  const timeVal = document.getElementById('time-multiplier-val');
  if (timeSlider) {
    timeSlider.addEventListener('input', (e) => {
      const val = parseFloat(e.target.value);
      state.timeMultiplier = val;
      if (timeVal) timeVal.innerText = `${val.toFixed(1)}x`;
    });
  }

  // Visual Toggles
  const bindToggle = (id, prop, onUpdate) => {
    const el = document.getElementById(id);
    if (el) {
      el.addEventListener('change', (e) => {
        state[prop] = e.target.checked;
        if (onUpdate) onUpdate(state[prop]);
      });
    }
  };

  bindToggle('toggle-trails', 'showTrails');
  bindToggle('toggle-spacetime', 'showSpacetime', (v) => { if (spacetimeMesh) spacetimeMesh.visible = v; });
  bindToggle('toggle-grid', 'showGrid', (v) => { if (eclipticGrid) eclipticGrid.visible = v; });
  bindToggle('toggle-shadow-cone', 'showShadowCone', (v) => { if (shadowConeGroup) shadowConeGroup.visible = v; });
  bindToggle('toggle-test-particles', 'showTestParticles');
  bindToggle('toggle-vis-viva', 'visVivaColoring');

  // Raycasting for planet & satellite selection
  const raycaster = new THREE.Raycaster();
  const mouse = new THREE.Vector2();

  window.addEventListener('click', (e) => {
    if (e.target.closest('.panel') || e.target.closest('header') || e.target.closest('#gmap-place-card') || e.target.closest('#bottom-toolbar')) return;

    mouse.x = (e.clientX / window.innerWidth) * 2 - 1;
    mouse.y = -(e.clientY / window.innerHeight) * 2 + 1;

    // Check satellite click proximity (within 20 px)
    let clickedSat = null;
    let minClickDist = 20;
    const w = window.innerWidth, h = window.innerHeight;

    for (let i = 0; i < state.testParticles.length; i++) {
      const tp = state.testParticles[i];
      if (!tp.mesh || !tp.mesh.visible) continue;
      const wp = tp.mesh.position.clone().project(camera);
      if (wp.z > 1.0) continue;
      const sx = ((wp.x + 1) * w) / 2;
      const sy = ((-wp.y + 1) * h) / 2;
      const dPx = Math.hypot(e.clientX - sx, e.clientY - sy);
      if (dPx < minClickDist) {
        minClickDist = dPx;
        clickedSat = tp;
      }
    }

    if (clickedSat) {
      selectSatellite(clickedSat);
      return;
    }

    // Check body raycasting
    raycaster.setFromCamera(mouse, camera);
    const hits = raycaster.intersectObjects(bodyMeshes.filter(m => m.visible));
    if (hits.length > 0) {
      const idx = hits[0].object.userData.bodyIndex;
      state.selectedBodyIndex = idx;
      openBodyPlaceCard(state.bodies[idx], state);
      flyTo(camera, controls, hits[0].object.position, 3.5, 1000);
    }
  });
}

function selectSatellite(tp) {
  state.selectedSatelliteId = tp.id;
  state.isTrackingSatellite = true;

  if (satelliteLayers) {
    satelliteLayers.reticleGroup.visible = true;
    satelliteLayers.reticleGroup.position.copy(tp.mesh.position);
  }

  const sun = state.bodies.find(b => b.name === 'Sun');
  const anchor = state.bodies.find(b => b.name === tp.anchorBodyName);
  let sunRel = [0, AU_M, 0];
  if (sun && anchor) {
    sunRel = [sun.pos[0] - anchor.pos[0], sun.pos[1] - anchor.pos[1], sun.pos[2] - anchor.pos[2]];
  }

  openSatelliteInspector(tp, getActiveSwarmEngine(), sunRel, sun ? sun.radius * 1e3 : 696340e3);
}

function onWindowResize() {
  camera.aspect = window.innerWidth / window.innerHeight;
  camera.updateProjectionMatrix();
  renderer.setSize(window.innerWidth, window.innerHeight);
}

// Main 60 FPS Render Loop
let lastTime = performance.now();
let frameCount = 0;
let fpsTimer = 0;

function animate(currentTime) {
  requestAnimationFrame(animate);

  const deltaMs = currentTime - lastTime;
  lastTime = currentTime;
  const dtSec = Math.min(0.1, deltaMs / 1000);

  // FPS calculation
  frameCount++;
  fpsTimer += dtSec;
  if (fpsTimer >= 0.5) {
    const fps = Math.round(frameCount / fpsTimer);
    const hudFps = document.getElementById('hud-fps');
    if (hudFps) hudFps.innerText = `${fps} FPS`;
    frameCount = 0;
    fpsTimer = 0;
  }

  if (state.isPlaying && wasmNBody) {
    const simDt = dtSec * (state.timeMultiplier * 86400) * 0.05;
    state.time_s += simDt;
    state.step_count += state.subSteps;

    // 1. Advance N-Body System in WebAssembly
    wasmNBody.step(simDt, state.subSteps, state.integrator, state.enable_gr);

    // 2. Sync Body Positions from WebAssembly flat buffer
    const posFlat = wasmNBody.get_positions_flat();
    const velFlat = wasmNBody.get_velocities_flat();
    for (let i = 0; i < state.bodies.length; i++) {
      state.bodies[i].pos[0] = posFlat[i * 3];
      state.bodies[i].pos[1] = posFlat[i * 3 + 1];
      state.bodies[i].pos[2] = posFlat[i * 3 + 2];
      state.bodies[i].vel[0] = velFlat[i * 3];
      state.bodies[i].vel[1] = velFlat[i * 3 + 1];
      state.bodies[i].vel[2] = velFlat[i * 3 + 2];

      if (bodyMeshes[i]) {
        bodyMeshes[i].position.copy(toSceneCoords(state.bodies[i].pos));
      }
    }

    // 3. Advance Keplerian Swarm in WebAssembly
    if (state.activeCentricBody !== 'Sun') {
      stepSwarm(simDt, state.subSteps, state, toSceneCoords);
    }

    // 4. Update Spacetime Potential Funnels
    if (state.showSpacetime && spacetimeMesh) {
      updateSpacetimeMesh(spacetimeMesh, state.bodies, toSceneCoords);
    }

    // 5. Update Shadow Cone Transform
    if (state.showShadowCone && shadowConeGroup) {
      const sun = state.bodies.find(b => b.name === 'Sun');
      const anchor = state.bodies.find(b => b.name === state.activeCentricBody);
      updateShadowConeTransform(shadowConeGroup, anchor, sun, toSceneCoords);
    }

    // 6. Update Basins
    updatePlanetBasinPositions(gravitationalBasinsGroup, state.bodies, toSceneCoords);
  }

  // Camera Tracking & Satellite Reticle Update
  if (state.selectedSatelliteId && satelliteLayers) {
    const selSat = state.testParticles.find(p => p.id === state.selectedSatelliteId);
    if (selSat && selSat.mesh) {
      satelliteLayers.reticleGroup.visible = true;
      satelliteLayers.reticleGroup.position.copy(selSat.mesh.position);
      satelliteLayers.reticleGroup.quaternion.copy(camera.quaternion);
      satelliteLayers.reticleGroup.rotateZ(0.015);

      if (state.isTrackingSatellite) {
        controls.target.copy(selSat.mesh.position);
      }

      // Update inspector card telemetry
      const sun = state.bodies.find(b => b.name === 'Sun');
      const anchor = state.bodies.find(b => b.name === selSat.anchorBodyName);
      let sunRel = [0, AU_M, 0];
      if (sun && anchor) {
        sunRel = [sun.pos[0] - anchor.pos[0], sun.pos[1] - anchor.pos[1], sun.pos[2] - anchor.pos[2]];
      }
      updateSatelliteInspectorCard(selSat, getActiveSwarmEngine(), sunRel, sun ? sun.radius * 1e3 : 696340e3);
    }
  }

  // Update HUD
  const hudTime = document.getElementById('hud-time');
  if (hudTime) {
    const days = (state.time_s / DAY_S).toFixed(2);
    hudTime.innerText = `T + ${days} days`;
  }

  // Update Controls & Render
  controls.update();
  renderer.render(scene, camera);
}

// Bootstrap Application on Window Load
window.addEventListener('DOMContentLoaded', () => {
  initApp().catch(err => {
    console.error('Fatal initialization error:', err);
    document.title = 'ERROR INITIALIZING ASTRO VISUALIZER';
  });
});
