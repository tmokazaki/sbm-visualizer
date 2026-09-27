/**
 * Keplerian Swarm Particle Management with WebAssembly Symplectic Integration.
 * Renders satellite meshes, closed Keplerian ellipses, and Vis-Viva kinetic thermal grading.
 */
import { createSwarmEngine } from './wasm_bridge.js';
import { getCentricFrameConfig } from './centric_config.js';
import { AU_M } from './constants.js';

let activeSwarmEngine = null;
let swarmParticleObjects = [];

export function getVisVivaColorRGB(tau) {
  const tClamped = Math.max(0.0, Math.min(1.0, tau));
  if (tClamped < 0.25) {
    const f = tClamped / 0.25;
    return [0.0, 0.4 + 0.6 * f, 1.0];
  } else if (tClamped < 0.5) {
    const f = (tClamped - 0.25) / 0.25;
    return [0.0, 1.0, 1.0 - f];
  } else if (tClamped < 0.75) {
    const f = (tClamped - 0.5) / 0.25;
    return [f, 1.0, 0.0];
  } else {
    const f = (tClamped - 0.75) / 0.25;
    return [1.0, 1.0 - 0.7 * f, 0.0];
  }
}

export function clearSwarm(scene, state) {
  if (activeSwarmEngine) {
    try {
      activeSwarmEngine.free();
    } catch (_) {}
    activeSwarmEngine = null;
  }

  swarmParticleObjects.forEach(p => {
    if (p.mesh) {
      scene.remove(p.mesh);
      if (p.mesh.geometry) p.mesh.geometry.dispose();
      if (p.mesh.material) p.mesh.material.dispose();
    }
    if (p.trailLine) {
      scene.remove(p.trailLine);
      if (p.trailGeo) p.trailGeo.dispose();
      if (p.trailLine.material) p.trailLine.material.dispose();
    }
  });
  swarmParticleObjects = [];
  state.testParticles = [];
  state.selectedSatelliteId = null;
  state.isTrackingSatellite = false;
  state.isolateSelectedOrbit = false;

  const vvLegend = document.getElementById('visviva-legend');
  if (vvLegend) vvLegend.style.display = 'none';

  const hudSunlit = document.getElementById('hud-sunlit-count');
  if (hudSunlit) hudSunlit.innerText = '☀️ Sunlit: 0';
  const hudUmbra = document.getElementById('hud-umbra-count');
  if (hudUmbra) hudUmbra.innerText = '🌑 Eclipse: 0';
}

export function spawnSwarm(scene, anchorName, preset, count, state) {
  clearSwarm(scene, state);

  // Instantiate WebAssembly Keplerian Swarm Engine
  activeSwarmEngine = createSwarmEngine(anchorName, preset, count);
  const numParticles = activeSwarmEngine.particle_count();
  const cfg = getCentricFrameConfig(anchorName);
  const span = cfg.spanScene || 3.6;
  const halfSpan = cfg.halfSpanPhys_m || 1_200_000e3;

  const sphereGeo = new THREE.SphereGeometry(0.016, 8, 8);
  const basePalette = ['#38bdf8', '#818cf8', '#c084fc', '#f472b6', '#34d399', '#fbbf24', '#f87171'];

  for (let i = 0; i < numParticles; i++) {
    const satId = `sat_${i + 1}`;
    const satName = `SAT-${String(i + 1).padStart(3, '0')}`;
    const colorHex = basePalette[i % basePalette.length];

    // Create Particle Dot Mesh
    const mat = new THREE.MeshBasicMaterial({ color: new THREE.Color(colorHex) });
    const mesh = new THREE.Mesh(sphereGeo, mat);
    mesh.name = satName;
    mesh.userData = { isTestParticle: true, particleId: satId, satIndex: i };
    scene.add(mesh);

    // Fetch closed Keplerian ellipse orbit vertices directly from WASM
    const trailFlat = activeSwarmEngine.get_orbit_trail_flat(i);
    const trailPoints = [];
    const trailColors = [];
    const ptsCount = trailFlat.length / 3;

    for (let ptIdx = 0; ptIdx < ptsCount; ptIdx++) {
      const rx = trailFlat[ptIdx * 3];
      const ry = trailFlat[ptIdx * 3 + 1];
      const rz = trailFlat[ptIdx * 3 + 2];
      trailPoints.push(new THREE.Vector3(
        (rx / halfSpan) * span,
        (ry / halfSpan) * span,
        (rz / halfSpan) * span
      ));
      // Base coloring
      const c = new THREE.Color(colorHex);
      trailColors.push(c.r, c.g, c.b);
    }

    const trailGeo = new THREE.BufferGeometry().setFromPoints(trailPoints);
    trailGeo.setAttribute('color', new THREE.Float32BufferAttribute(trailColors, 3));
    const trailMat = new THREE.LineBasicMaterial({
      vertexColors: true,
      transparent: true,
      opacity: 0.65,
      depthWrite: false
    });
    const trailLine = new THREE.Line(trailGeo, trailMat);
    scene.add(trailLine);

    const ptObj = {
      id: satId,
      index: i,
      name: satName,
      anchorBodyName: anchorName,
      mesh,
      trailLine,
      trailGeo,
      trailFlat,
      colorHex,
      eclipseState: 'sunlit',
      relPos: [0, 0, 0]
    };

    swarmParticleObjects.push(ptObj);
    state.testParticles.push(ptObj);
  }

  const vvLegend = document.getElementById('visviva-legend');
  if (vvLegend) {
    vvLegend.style.display = (state.visVivaColoring !== false && state.showTestParticles !== false) ? 'flex' : 'none';
  }

  return activeSwarmEngine;
}

export function stepSwarm(dt, subSteps, state, toSceneCoords) {
  if (!activeSwarmEngine || swarmParticleObjects.length === 0) return;

  // 1. Advance Swarm in WebAssembly
  activeSwarmEngine.step(dt, subSteps);

  // 2. Fetch Flat State Buffers from WebAssembly Memory
  const posFlat = activeSwarmEngine.get_positions_flat();
  const kineticsFlat = activeSwarmEngine.get_vis_viva_kinetic_flat();

  // Evaluate Solar Eclipse States via Anchor Body Shadow Cone in WASM
  const sun = state.bodies.find(b => b.name === 'Sun');
  const anchor = state.bodies.find(b => b.name === state.activeCentricBody);
  let eclipseStates = null;
  if (sun && anchor) {
    const sunRel = new Float64Array([
      sun.pos[0] - anchor.pos[0],
      sun.pos[1] - anchor.pos[1],
      sun.pos[2] - anchor.pos[2]
    ]);
    eclipseStates = activeSwarmEngine.get_eclipse_states_flat(sunRel, sun.radius ? sun.radius * 1e3 : 696340e3);
  }

  const anchorScPos = (anchor && anchor.name !== 'Sun') ? toSceneCoords(anchor.pos) : new THREE.Vector3(0, 0, 0);
  const cfg = getCentricFrameConfig(state.activeCentricBody || 'Earth');
  const span = cfg.spanScene || 3.6;
  const halfSpan = cfg.halfSpanPhys_m || 1_200_000e3;

  let sunlit = 0, umbra = 0, penumbra = 0;
  const isVisible = (state.showTestParticles !== false);
  const showTrails = (state.showTrails !== false) && isVisible;
  const isIsolating = state.isolateSelectedOrbit && state.selectedSatelliteId;

  for (let i = 0; i < swarmParticleObjects.length; i++) {
    const p = swarmParticleObjects[i];
    p.mesh.visible = isVisible && (!isIsolating || p.id === state.selectedSatelliteId);
    p.trailLine.visible = showTrails && (!isIsolating || p.id === state.selectedSatelliteId);

    if (!p.mesh.visible && !p.trailLine.visible) continue;

    const rx = posFlat[i * 3];
    const ry = posFlat[i * 3 + 1];
    const rz = posFlat[i * 3 + 2];
    p.relPos = [rx, ry, rz];

    const scX = anchorScPos.x + (rx / halfSpan) * span;
    const scY = anchorScPos.y + (ry / halfSpan) * span;
    const scZ = anchorScPos.z + (rz / halfSpan) * span + 0.005;

    p.mesh.position.set(scX, scY, scZ);
    p.trailLine.position.set(anchorScPos.x, anchorScPos.y, anchorScPos.z);

    // Illumination state
    const eState = eclipseStates ? eclipseStates[i] : 0;
    if (eState === 2) {
      p.eclipseState = 'umbra';
      umbra++;
    } else if (eState === 1) {
      p.eclipseState = 'penumbra';
      penumbra++;
    } else {
      p.eclipseState = 'sunlit';
      sunlit++;
    }

    // Color interpolation
    if (state.visVivaColoring !== false && kineticsFlat && kineticsFlat.length > i) {
      const tau = kineticsFlat[i];
      const [cr, cg, cb] = getVisVivaColorRGB(tau);
      p.mesh.material.color.setRGB(cr, cg, cb);
    } else if (p.eclipseState === 'umbra') {
      p.mesh.material.color.setHex(0x60a5fa);
    } else {
      p.mesh.material.color.set(p.colorHex);
    }
  }

  // Update HUD
  const hudSunlit = document.getElementById('hud-sunlit-count');
  if (hudSunlit) hudSunlit.innerText = `☀️ Sunlit: ${sunlit}`;
  const hudUmbra = document.getElementById('hud-umbra-count');
  if (hudUmbra) hudUmbra.innerText = `🌑 Eclipse: ${umbra + penumbra}`;

  state.sunlitCount = sunlit;
  state.umbraCount = umbra;
  state.penumbraCount = penumbra;
}

export function getActiveSwarmEngine() {
  return activeSwarmEngine;
}
