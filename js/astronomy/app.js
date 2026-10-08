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
  getActiveSwarmEngine,
  setSwarmPerturbationMode,
  getSwarmPerturbationMode,
  getCentricPerturbersFlat
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
  showTethers: true,
  showSpacetime: true,
  showHillSpheres: true,
  showGrid: true,
  showBasins: true,
  showVectorGrid: false,
  showEquipotentials: true,
  perturbationMagnifier: 5000,
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
  penumbraCount: 0,
  viewMode: 'google_maps',
  isTopDown2D: false
};

// Three.js Core Objects
let scene, camera, renderer, controls;
let wasmNBody = null;
let bodyMeshes = [];
let trailLines = [];
let hillSphereMeshes = [];
let vectorArrows = [];
let forceVectorArrows = [];
let forceTetherLines = [];
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
    onPresetChange: (presetId) => loadPreset(presetId),
    onReset: () => loadPreset(state.presetId),
    onToggleScaleMode: () => {
      state.scaleMode = state.scaleMode === 'perceptual' ? 'true' : 'perceptual';
      const btn = document.getElementById('btn-scale-mode');
      if (btn) btn.innerText = `Scale: ${state.scaleMode === 'perceptual' ? 'Perceptual' : 'True 1:1'}`;
      rebuildSceneMeshes();
    },
    onCameraFocus: (val) => focusCameraOn(val),
    onZoom: (factor) => {
      const offset = camera.position.clone().sub(controls.target);
      camera.position.copy(controls.target).add(offset.multiplyScalar(factor));
      controls.update();
    },
    onToggleTilt: () => {
      state.isTopDown2D = !state.isTopDown2D;
      const dist = camera.position.distanceTo(controls.target);
      const lbl = document.getElementById('tilt-label');
      if (lbl) lbl.innerText = state.isTopDown2D ? '2D' : '3D';
      if (state.isTopDown2D) {
        camera.position.set(controls.target.x, controls.target.y - 0.001, controls.target.z + dist);
      } else {
        camera.position.set(controls.target.x, controls.target.y - dist * 0.82, controls.target.z + dist * 0.55);
      }
      controls.update();
    },
    onRecenter: () => {
      const anchor = state.bodies.find(b => b.name === state.activeCentricBody) || state.bodies.find(b => b.name === 'Earth') || state.bodies[0];
      if (anchor) {
        const bIdx = state.bodies.indexOf(anchor);
        state.selectedBodyIndex = bIdx;
        openBodyPlaceCard(anchor, state);
        flyTo(camera, controls, toSceneCoords(anchor.pos), 3.5, 1000);
      }
    },
    onCompass: () => {
      const dist = camera.position.distanceTo(controls.target);
      camera.position.set(controls.target.x, controls.target.y - dist * 0.85, controls.target.z + dist * 0.52);
      controls.update();
    },
    onFocusHillSphere: () => {
      state.showHillSpheres = true;
      hillSphereMeshes.forEach(h => h.mesh.visible = true);
      const sel = hillSphereMeshes.find(h => h.bodyIndex === state.selectedBodyIndex);
      if (sel) {
        flyTo(camera, controls, toSceneCoords(state.bodies[state.selectedBodyIndex].pos), sel.sceneRadius * 2.8, 1000);
      }
    },
    onCameraView: (view) => {
      const dist = camera.position.distanceTo(controls.target);
      if (view === 'top') {
        camera.position.set(controls.target.x, controls.target.y - 0.001, controls.target.z + dist);
      } else if (view === 'side') {
        camera.position.set(controls.target.x, controls.target.y - dist, controls.target.z);
      } else {
        camera.position.set(controls.target.x + dist * 0.6, controls.target.y - dist * 0.6, controls.target.z + dist * 0.5);
      }
      controls.update();
    },
    onToggleForces: (v) => {
      state.showForces = v;
      updateForceVectorsAndTethers();
    },
    onUpdateMagnifier: (v) => {
      state.perturbationMagnifier = v;
      updateForceVectorsAndTethers();
    },
    onToggleTethers: (v) => {
      state.showTethers = v;
      updateForceVectorsAndTethers();
    },
    onToggleSpacetime: (v) => {
      state.showSpacetime = v;
      if (spacetimeMesh) spacetimeMesh.visible = v;
    },
    onToggleHills: (v) => {
      state.showHillSpheres = v;
      hillSphereMeshes.forEach(h => h.mesh.visible = v);
    },
    onToggleTrails: (v) => {
      state.showTrails = v;
      trailLines.forEach(l => l.visible = v);
    },
    onToggleVectors: (v) => {
      state.showVectors = v;
      vectorArrows.forEach(a => a.visible = v);
    },
    onToggleGrid: (v) => {
      state.showGrid = v;
      if (eclipticGrid) eclipticGrid.visible = v;
    },
    onToggleBasins: (v) => {
      if (gravitationalBasinsGroup) gravitationalBasinsGroup.visible = v;
    },
    onToggleTestParticles: (v) => {
      state.showTestParticles = v;
      state.testParticles.forEach(p => {
        if (p.mesh) p.mesh.visible = v;
        if (p.orbitLine) p.orbitLine.visible = v && !state.isolateSelectedOrbit;
      });
    },
    onToggleShadowCone: (v) => {
      state.showShadowCone = v;
      if (shadowConeGroup) shadowConeGroup.visible = v && (state.activeCentricBody !== 'Sun');
    },
    onCockpitOpen: () => {
      updateClassicHUD();
    },
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
      updateForceVectorsAndTethers();
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
    },
    onCyclePerturbationMode: () => {
      const curMode = getSwarmPerturbationMode();
      // Cycle: 2 (Full) -> 0 (TwoBody) -> 1 (ThirdBody) -> 2 (Full)
      const nextMode = (curMode + 1) % 3;
      setSwarmPerturbationMode(nextMode);
      state.perturbationMode = nextMode;

      const chipText = document.getElementById('perturbation-mode-text');
      const labels = [
        'Perturbations: Two-Body (Keplerian)',
        'Perturbations: Third-Body (Lunisolar)',
        'Perturbations: Full (3rd-Body + J2)'
      ];
      if (chipText) {
        chipText.innerText = labels[nextMode];
      }

      const toast = document.getElementById('gravitational-centric-toast');
      const toastTitle = document.getElementById('centric-toast-title');
      const toastDesc = document.getElementById('centric-toast-desc');
      if (toast && toastTitle && toastDesc) {
        toastTitle.innerText = `PERTURBATION DYNAMICS: ${labels[nextMode].toUpperCase()}`;
        const descriptions = [
          'Pure unperturbed central Keplerian two-body gravity (d²r/dt² = -μr/r³)',
          'Non-inertial direct and d\'Alembert reflex accelerations from Moon & Sun (Danby 1988)',
          'Full astrodynamics: Central + Lunisolar 3rd-body + Central body J2 oblateness (Kaula 1966)'
        ];
        toastDesc.innerText = descriptions[nextMode];
        toast.style.display = 'flex';
        clearTimeout(toast.timeoutId);
        toast.timeoutId = setTimeout(() => {
          toast.style.display = 'none';
        }, 4000);
      }
    }
  });

  // 7. Parse URL Query Parameters for Direct Linking / Automated Testing
  const params = new URLSearchParams(window.location.search);
  if (params.has('centric')) {
    setCentricBody(params.get('centric'));
  }
  if (params.has('particles')) {
    const count = parseInt(params.get('particles'), 10) || 300;
    spawnSwarm(scene, state.activeCentricBody || 'Earth', 'swarm', count, state);
  }
  if (params.has('perturb')) {
    const pMode = parseInt(params.get('perturb'), 10) || 0;
    setSwarmPerturbationMode(pMode);
    state.perturbationMode = pMode;
    const chipText = document.getElementById('perturbation-mode-text');
    const labels = [
      'Perturbations: Two-Body (Keplerian)',
      'Perturbations: Third-Body (Lunisolar)',
      'Perturbations: Full (3rd-Body + J2)'
    ];
    if (chipText && labels[pMode]) chipText.innerText = labels[pMode];
  }
  if (params.has('sat')) {
    const rawSat = params.get('sat');
    const sat = state.testParticles.find(p => p.id === rawSat || p.id === `sat_${rawSat}` || p.name === `SAT-${String(rawSat).padStart(3, '0')}`);
    if (sat) {
      selectSatellite(sat);
      if (params.get('isolate') === '1') {
        state.isolateSelectedOrbit = true;
        const btnIsolate = document.getElementById('btn-sat-isolate');
        if (btnIsolate) btnIsolate.classList.add('active');
      }
    }
  }
  if (params.get('layers') === '1') {
    const layersMenu = document.getElementById('gmap-layers-menu');
    if (layersMenu) layersMenu.style.display = 'flex';
  }
  if (params.get('cockpit') === '1') {
    const toggleBtn = document.getElementById('btn-toggle-view-mode');
    if (toggleBtn) toggleBtn.click();
  }

  // 8. Window Resize Listener
  window.addEventListener('resize', onWindowResize);

  // 9. Start Animation Loop
  requestAnimationFrame(animate);
}

function focusCameraOn(val) {
  if (val === '-1') {
    flyTo(camera, controls, new THREE.Vector3(0, 0, 0), 25, 1000);
  } else if (val === 'emb') {
    const earth = state.bodies.find(b => b.name === 'Earth');
    if (earth) flyTo(camera, controls, toSceneCoords(earth.pos), 3.5, 1000);
  } else if (val === 'jupiter') {
    const jup = state.bodies.find(b => b.name === 'Jupiter');
    if (jup) flyTo(camera, controls, toSceneCoords(jup.pos), 6.0, 1000);
  } else {
    const bIdx = parseInt(val, 10);
    if (!isNaN(bIdx) && state.bodies[bIdx]) {
      flyTo(camera, controls, toSceneCoords(state.bodies[bIdx].pos), 3.5, 1000);
    }
  }
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
  hillSphereMeshes.forEach(h => scene.remove(h.mesh));
  hillSphereMeshes = [];
  vectorArrows.forEach(a => scene.remove(a));
  vectorArrows = [];
  forceVectorArrows.forEach(a => scene.remove(a));
  forceVectorArrows = [];
  forceTetherLines.forEach(l => scene.remove(l));
  forceTetherLines = [];

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
        scene.add(ringLine);
        trailLines.push(ringLine);
      }
    }

    // Hill Sphere Mesh (Roche Lobe approx)
    if (idx !== 0 && b.name !== 'Sun') {
      const primary = state.bodies[0];
      const dx = b.pos[0] - primary.pos[0];
      const dy = b.pos[1] - primary.pos[1];
      const dz = b.pos[2] - primary.pos[2];
      const a = Math.hypot(dx, dy, dz);
      const rH_m = a * Math.cbrt(b.mass / (3.0 * (primary.mass || 1.989e30)));
      let rH_scene = rH_m * state.sceneScale;
      if (state.scaleMode === 'perceptual') {
        rH_scene = Math.max(0.6, Math.min(6.0, rH_scene * 2.5));
      }
      const hGeo = new THREE.SphereGeometry(rH_scene, 24, 16);
      const hMat = new THREE.MeshBasicMaterial({
        color: new THREE.Color(b.color),
        wireframe: true,
        transparent: true,
        opacity: 0.22,
        depthWrite: false
      });
      const hMesh = new THREE.Mesh(hGeo, hMat);
      hMesh.position.copy(toSceneCoords(b.pos));
      hMesh.visible = state.showHillSpheres;
      scene.add(hMesh);
      hillSphereMeshes.push({ mesh: hMesh, bodyIndex: idx, sceneRadius: rH_scene });
    }

    // Velocity Arrow
    const velMag = Math.hypot(b.vel[0], b.vel[1], b.vel[2]);
    const velDir = velMag > 0 ? new THREE.Vector3(b.vel[0] / velMag, b.vel[1] / velMag, b.vel[2] / velMag) : new THREE.Vector3(1, 0, 0);
    const arrow = new THREE.ArrowHelper(velDir, toSceneCoords(b.pos), 1.8, b.color, 0.4, 0.2);
    arrow.visible = state.showVectors;
    scene.add(arrow);
    vectorArrows.push(arrow);
  });

  // Populate Body Selector Dropdown in Right Telemetry Panel
  const selDropdown = document.getElementById('body-selector-dropdown');
  if (selDropdown) {
    selDropdown.innerHTML = state.bodies.map((b, idx) => `<option value="${idx}">${b.name}</option>`).join('');
    selDropdown.value = state.selectedBodyIndex;
  }

  // Basins
  buildPlanetBasins(gravitationalBasinsGroup, state.bodies, toSceneCoords, state.sceneScale);
  updateForceVectorsAndTethers();
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

function updateForceVectorsAndTethers() {
  forceVectorArrows.forEach(a => scene.remove(a));
  forceTetherLines.forEach(l => scene.remove(l));
  forceVectorArrows = [];
  forceTetherLines = [];

  if ((!state.showForces && !state.showTethers) || state.bodies.length < 2) return;
  const selIdx = state.selectedBodyIndex;
  if (selIdx < 0 || selIdx >= state.bodies.length) return;

  const bodyI = state.bodies[selIdx];
  const posI = toSceneCoords(bodyI.pos);
  const G = 6.67430e-11;

  const forces = [];
  for (let j = 0; j < state.bodies.length; j++) {
    if (j === selIdx) continue;
    const bodyJ = state.bodies[j];
    const dx = bodyJ.pos[0] - bodyI.pos[0];
    const dy = bodyJ.pos[1] - bodyI.pos[1];
    const dz = bodyJ.pos[2] - bodyI.pos[2];
    const r2 = dx * dx + dy * dy + dz * dz;
    const r = Math.sqrt(r2);
    if (r <= 1e-12) continue;

    const fMag = (G * bodyI.mass * bodyJ.mass) / r2;
    forces.push({ targetIndex: j, name: bodyJ.name, color: bodyJ.color, fMag, dx, dy, dz, r });

    // 3D Force Tether Line (Beam)
    if (state.showTethers) {
      const posJ = toSceneCoords(bodyJ.pos);
      const geo = new THREE.BufferGeometry().setFromPoints([posI, posJ]);
      const mat = new THREE.LineBasicMaterial({
        color: new THREE.Color(bodyJ.color),
        transparent: true,
        opacity: 0.45
      });
      const line = new THREE.Line(geo, mat);
      scene.add(line);
      forceTetherLines.push(line);
    }
  }

  // 3D Force Vectors
  if (state.showForces && forces.length > 0) {
    const maxF = Math.max(...forces.map(f => f.fMag));
    forces.forEach(f => {
      const dir = new THREE.Vector3(f.dx, f.dy, f.dz).normalize();
      const ratio = f.fMag / (maxF || 1.0);
      const len = Math.max(0.8, Math.min(5.0, 3.0 * ratio * (state.perturbationMagnifier ? Math.log10(state.perturbationMagnifier) / 3.0 : 1.0)));
      const arrow = new THREE.ArrowHelper(dir, posI, len, f.color, 0.35, 0.2);
      scene.add(arrow);
      forceVectorArrows.push(arrow);
    });
  }
}

function updateClassicHUD() {
  // 1. Invariants & Energy from WASM
  if (wasmNBody && wasmNBody.get_conservation_metrics) {
    try {
      const m = wasmNBody.get_conservation_metrics();
      if (m) {
        const badge = document.getElementById('energy-badge');
        if (badge) {
          const err = m.relative_energy_error || 0;
          badge.innerText = `ΔE/E₀: ${err.toExponential(2)}`;
          badge.className = err < 1e-6 ? 'badge badge-green' : (err < 1e-4 ? 'badge badge-gold' : 'badge badge-purple');
        }
        const totE = document.getElementById('hud-total-energy');
        if (totE) totE.innerText = `${(m.total_energy_j || 0).toExponential(4)} J`;
        const splitE = document.getElementById('hud-energy-split');
        if (splitE) splitE.innerText = `T=${(m.kinetic_energy_j || 0).toExponential(3)} J, V=${(m.potential_energy_j || 0).toExponential(3)} J`;
        const linMom = document.getElementById('hud-lin-mom');
        if (linMom) linMom.innerText = `${(m.linear_momentum_magnitude || 0).toExponential(3)} kg·m/s`;
        const angMom = document.getElementById('hud-ang-mom');
        if (angMom) angMom.innerText = `${(m.angular_momentum_magnitude || 0).toExponential(3)} kg·m²/s`;
      }
    } catch (_) {}
  }

  // 2. Sim Time & Steps
  const simTime = document.getElementById('sim-time-text');
  if (simTime) {
    const days = (state.time_s / DAY_S).toFixed(2);
    simTime.innerText = `${days} days`;
  }
  const simSteps = document.getElementById('sim-steps-text');
  if (simSteps) simSteps.innerText = state.step_count.toLocaleString();

  // 3. Right panel telemetry for selected body
  const selIdx = state.selectedBodyIndex;
  if (selIdx >= 0 && selIdx < state.bodies.length) {
    const b = state.bodies[selIdx];
    const nameEl = document.getElementById('inspector-body-name');
    if (nameEl) nameEl.innerText = b.name;
    const bBadge = document.getElementById('inspector-badge');
    if (bBadge) bBadge.innerText = selIdx === 0 ? 'Primary' : `Body #${selIdx}`;

    const telMass = document.getElementById('tel-mass');
    if (telMass) telMass.innerText = `${b.mass.toExponential(4)} kg`;
    const telRad = document.getElementById('tel-radius');
    if (telRad) telRad.innerText = `${(b.radius || 0).toLocaleString()} km`;

    const spd = Math.hypot(b.vel[0], b.vel[1], b.vel[2]);
    const telSpd = document.getElementById('tel-speed');
    if (telSpd) telSpd.innerText = `${(spd / 1e3).toFixed(3)} km/s`;

    const dist = Math.hypot(b.pos[0], b.pos[1], b.pos[2]);
    const telDist = document.getElementById('tel-dist');
    if (telDist) telDist.innerText = `${(dist / AU_M).toFixed(4)} AU (${(dist / 1e3).toLocaleString()} km)`;

    const telPos = document.getElementById('tel-pos');
    if (telPos) telPos.innerText = `[${(b.pos[0]/AU_M).toFixed(3)}, ${(b.pos[1]/AU_M).toFixed(3)}, ${(b.pos[2]/AU_M).toFixed(3)}] AU`;
    const telVel = document.getElementById('tel-vel');
    if (telVel) telVel.innerText = `[${(b.vel[0]/1e3).toFixed(2)}, ${(b.vel[1]/1e3).toFixed(2)}, ${(b.vel[2]/1e3).toFixed(2)}] km/s`;

    // Osculating elements from WASM
    if (selIdx !== 0 && wasmNBody && wasmNBody.get_osculating_elements) {
      try {
        const el = wasmNBody.get_osculating_elements(selIdx, 0);
        if (el) {
          const setEl = (id, txt) => { const e = document.getElementById(id); if (e) e.innerText = txt; };
          setEl('el-a', `${(el.semi_major_axis_m / AU_M).toFixed(4)} AU`);
          setEl('el-e', el.eccentricity.toFixed(5));
          setEl('el-inc', `${el.inclination_deg.toFixed(2)}°`);
          setEl('el-raan', `${el.raan_deg.toFixed(2)}°`);
          setEl('el-argp', `${el.arg_periapsis_deg.toFixed(2)}°`);
          setEl('el-nu', `${el.true_anomaly_deg.toFixed(2)}°`);
          const perDays = (el.orbital_period_s / DAY_S).toFixed(2);
          setEl('el-period', `${perDays} d`);
          setEl('el-apsis', `q=${(el.periapsis_distance_m/AU_M).toFixed(3)} / Q=${(el.apoapsis_distance_m/AU_M).toFixed(3)} AU`);
        }
      } catch (_) {}
    }
  }

  // 4. Dynamic Scale Bar
  const scaleText = document.getElementById('gmap-scale-text');
  if (scaleText) {
    const distScene = camera.position.distanceTo(controls.target);
    const invScale = 1.0 / (state.sceneScale || 1.0);
    const physDistM = distScene * invScale;
    const physDistAU = physDistM / AU_M;
    if (physDistAU >= 0.05) {
      scaleText.innerText = `${physDistAU.toFixed(2)} AU (${(physDistM / 1e9).toFixed(0)}M km)`;
    } else {
      scaleText.innerText = `${(physDistM / 1e6).toFixed(0)}k km`;
    }
  }
}

function setupEventListeners() {
  // Layer Checkboxes inside #gmap-layers-menu
  const bindCheckbox = (id, prop, onUpdate) => {
    const el = document.getElementById(id);
    if (el) {
      el.addEventListener('change', (e) => {
        state[prop] = e.target.checked;
        if (onUpdate) onUpdate(state[prop]);
      });
    }
  };

  bindCheckbox('chk-layer-forces', 'showForces', () => updateForceVectorsAndTethers());
  bindCheckbox('chk-layer-tethers', 'showTethers', () => updateForceVectorsAndTethers());
  bindCheckbox('chk-layer-hills', 'showHillSpheres', (v) => hillSphereMeshes.forEach(h => h.mesh.visible = v));
  bindCheckbox('chk-layer-spacetime', 'showSpacetime', (v) => { if (spacetimeMesh) spacetimeMesh.visible = v; });
  bindCheckbox('chk-layer-trails', 'showTrails', (v) => trailLines.forEach(l => l.visible = v));
  bindCheckbox('chk-layer-grid', 'showGrid', (v) => { if (eclipticGrid) eclipticGrid.visible = v; });
  bindCheckbox('chk-layer-basins', 'showBasins', (v) => { if (gravitationalBasinsGroup) gravitationalBasinsGroup.visible = v; });
  bindCheckbox('chk-layer-vector-grid', 'showVectorGrid', (v) => { if (spatialVectorGridGroup) spatialVectorGridGroup.visible = v; });
  bindCheckbox('chk-layer-equipotentials', 'showEquipotentials', (v) => { if (equipotentialContoursGroup) equipotentialContoursGroup.visible = v; });
  bindCheckbox('chk-layer-test-particles', 'showTestParticles', (v) => {
    state.testParticles.forEach(p => {
      if (p.mesh) p.mesh.visible = v;
      if (p.orbitLine) p.orbitLine.visible = v && !state.isolateSelectedOrbit;
    });
  });
  bindCheckbox('chk-layer-visviva', 'visVivaColoring');
  bindCheckbox('chk-layer-shadow-cone', 'showShadowCone', (v) => {
    if (shadowConeGroup) shadowConeGroup.visible = v && (state.activeCentricBody !== 'Sun');
  });

  // Raycasting for planet & satellite selection
  const raycaster = new THREE.Raycaster();
  const mouse = new THREE.Vector2();

  window.addEventListener('click', (e) => {
    if (e.target.closest('.panel') || e.target.closest('header') || e.target.closest('#gmap-place-card') || e.target.closest('#bottom-toolbar') || e.target.closest('#gmap-layers-widget')) return;

    mouse.x = (e.clientX / window.innerWidth) * 2 - 1;
    mouse.y = -(e.clientY / window.innerHeight) * 2 + 1;

    // Check satellite proximity (within 20 px)
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

    // Check celestial body raycasting
    raycaster.setFromCamera(mouse, camera);
    const hits = raycaster.intersectObjects(bodyMeshes.filter(m => m.visible));
    if (hits.length > 0) {
      const idx = hits[0].object.userData.bodyIndex;
      state.selectedBodyIndex = idx;
      openBodyPlaceCard(state.bodies[idx], state);
      updateForceVectorsAndTethers();
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

  const perturbersFlat = getCentricPerturbersFlat(state);
  openSatelliteInspector(tp, getActiveSwarmEngine(), sunRel, sun ? sun.radius * 1e3 : 696340e3, perturbersFlat);

  if (tp.mesh) {
    controls.target.copy(tp.mesh.position);
    camera.position.set(tp.mesh.position.x + 0.9, tp.mesh.position.y + 0.6, tp.mesh.position.z + 1.2);
    controls.update();
  }
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

      const scPos = toSceneCoords(state.bodies[i].pos);
      if (bodyMeshes[i]) {
        bodyMeshes[i].position.copy(scPos);
      }
      if (vectorArrows[i]) {
        vectorArrows[i].position.copy(scPos);
        const vMag = Math.hypot(state.bodies[i].vel[0], state.bodies[i].vel[1], state.bodies[i].vel[2]);
        if (vMag > 0) {
          vectorArrows[i].setDirection(new THREE.Vector3(state.bodies[i].vel[0] / vMag, state.bodies[i].vel[1] / vMag, state.bodies[i].vel[2] / vMag));
        }
      }
    }

    // Sync Hill Sphere positions
    hillSphereMeshes.forEach(h => {
      if (state.bodies[h.bodyIndex]) {
        h.mesh.position.copy(toSceneCoords(state.bodies[h.bodyIndex].pos));
      }
    });

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

    // 7. Update Force Vectors & Tethers
    updateForceVectorsAndTethers();
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
      const perturbersFlat = getCentricPerturbersFlat(state);
      updateSatelliteInspectorCard(selSat, getActiveSwarmEngine(), sunRel, sun ? sun.radius * 1e3 : 696340e3, perturbersFlat);
    }
  }

  // Update HUDs
  const hudTime = document.getElementById('hud-time');
  if (hudTime) {
    const days = (state.time_s / DAY_S).toFixed(2);
    hudTime.innerText = `T + ${days} days`;
  }
  updateClassicHUD();

  // Update Controls & Render
  controls.update();
  renderer.render(scene, camera);
}

// Bootstrap Application on Window Load
window.addEventListener('DOMContentLoaded', () => {
  initApp().catch(err => {
    console.error('Fatal initialization error:', err);
    document.title = 'ERR: ' + (err && err.message ? err.message : String(err));
  });
});
