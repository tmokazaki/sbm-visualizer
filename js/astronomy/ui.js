/**
 * UI Controls, Telemetry Cards, Quick Chips, Omnibox, and Mouse Interaction Handlers.
 */
import { AU_M, DAY_S, YEAR_S, getBodyEmoji } from './constants.js';
import { getCentricFrameConfig, showCentricToast } from './centric_config.js';

let flightAnimation = null;

export function flyTo(camera, controls, targetPos, targetDist = 12, durationMs = 1200) {
  if (flightAnimation) cancelAnimationFrame(flightAnimation);
  const startPos = camera.position.clone();
  const startTarget = controls.target.clone();
  const endTarget = new THREE.Vector3(targetPos.x, targetPos.y, targetPos.z);

  let dir = startPos.clone().sub(startTarget);
  if (dir.lengthSq() < 0.001) {
    dir.set(0, -0.85, 0.52);
  }
  dir.normalize();
  const endPos = endTarget.clone().add(dir.multiplyScalar(targetDist));

  const startTime = performance.now();
  controls.enabled = false;

  function step(now) {
    const elapsed = now - startTime;
    const progress = Math.min(1.0, elapsed / durationMs);
    const ease = progress < 0.5 ? 4 * progress * progress * progress : 1 - Math.pow(-2 * progress + 2, 3) / 2;

    camera.position.lerpVectors(startPos, endPos, ease);
    controls.target.lerpVectors(startTarget, endTarget, ease);

    if (progress < 1.0) {
      flightAnimation = requestAnimationFrame(step);
    } else {
      controls.enabled = true;
      flightAnimation = null;
    }
  }
  flightAnimation = requestAnimationFrame(step);
}

export function openSatelliteInspector(tp, wasmSwarmEngine, sunRelPos, sunRadiusM, perturbersFlat) {
  const card = document.getElementById('gmap-place-card');
  if (!card) return;

  card.dataset.isSatellite = 'true';
  card.dataset.isSpacePoint = 'false';

  const planetActions = document.getElementById('planet-actions-row');
  const satActions = document.getElementById('satellite-actions-row');
  if (planetActions) planetActions.style.display = 'none';
  if (satActions) satActions.style.display = 'grid';

  const tidalCard = document.getElementById('place-tidal-card');
  if (tidalCard) tidalCard.style.display = 'none';
  const tugCard = document.getElementById('place-tug-card');
  if (tugCard) tugCard.style.display = 'none';

  document.getElementById('place-icon').innerText = '🛰️';
  document.getElementById('place-title').innerText = `${tp.name} (${tp.anchorBodyName} Centric)`;

  const heading = document.getElementById('place-specs-heading');
  if (heading) heading.innerText = 'Orbital Mechanics & Telemetry (Rust / WASM)';

  updateSatelliteInspectorCard(tp, wasmSwarmEngine, sunRelPos, sunRadiusM, perturbersFlat);
  card.style.display = 'block';
}

export function updateSatelliteInspectorCard(tp, wasmSwarmEngine, sunRelPos, sunRadiusM, perturbersFlat) {
  const card = document.getElementById('gmap-place-card');
  if (!card || card.dataset.isSatellite !== 'true' || card.style.display === 'none' || !tp) return;

  let telem = null;
  if (wasmSwarmEngine && wasmSwarmEngine.get_satellite_telemetry) {
    try {
      const sunRel = sunRelPos ? new Float64Array(sunRelPos) : new Float64Array([0, AU_M, 0]);
      const pertFlat = perturbersFlat || new Float64Array(0);
      telem = wasmSwarmEngine.get_satellite_telemetry(tp.index || 0, sunRel, sunRadiusM || 696340e3, pertFlat);
    } catch (_) {}
  }

  const altKm = telem ? (telem.current_altitude_m / 1e3) : 420;
  const speedKmS = telem ? (telem.current_speed_m_s / 1e3) : 7.66;
  const smaKm = telem ? (telem.semi_major_axis_m / 1e3) : 6791;
  const ecc = telem ? telem.eccentricity.toFixed(4) : '0.0012';
  const inc = telem ? telem.inclination_deg.toFixed(1) + '°' : '51.6°';
  const hpKm = telem ? (telem.periapsis_altitude_m / 1e3) : 415;
  const haKm = telem ? (telem.apoapsis_altitude_m / 1e3) : 425;
  const eclState = telem ? telem.eclipse_state : (tp.eclipseState || 'sunlit');

  let periodStr = 'N/A';
  if (telem && telem.orbital_period_s && isFinite(telem.orbital_period_s)) {
    const sec = telem.orbital_period_s;
    const h = Math.floor(sec / 3600);
    const m = Math.floor((sec % 3600) / 60);
    const s = Math.floor(sec % 60);
    periodStr = `${h}h ${m}m ${s}s`;
  }

  let eclipseHtml = '<span style="color: #facc15; font-weight: 700;">☀️ Sunlit (1,361 W/m²)</span>';
  let subtitleText = 'Direct Sunlight &bull; Nominal Solar Power';
  if (eclState === 'Umbra' || eclState === 'umbra') {
    eclipseHtml = '<span style="color: #60a5fa; font-weight: 700;">🌑 Total Umbral Eclipse (0 W/m²)</span>';
    subtitleText = `Occulted by ${tp.anchorBodyName} &bull; Battery Discharge`;
  } else if (eclState === 'Penumbra' || eclState === 'penumbra') {
    eclipseHtml = '<span style="color: #fb923c; font-weight: 700;">🌗 Penumbral Twilight (Partial)</span>';
    subtitleText = 'Penumbral Fringe &bull; Partial Solar Obscuration';
  }
  document.getElementById('place-subtitle').innerHTML = subtitleText;

  let accelRows = '';
  if (telem && telem.acceleration) {
    const acc = telem.acceleration;
    const formatAccel = (val) => {
      if (val >= 1.0) return `${val.toFixed(3)} m/s²`;
      if (val >= 1e-3) return `${(val * 1e3).toFixed(2)} mm/s²`;
      return `${(val * 1e6).toFixed(2)} µm/s²`;
    };

    const g0Str = formatAccel(acc.a_central_mps2);
    const j2Str = formatAccel(acc.a_j2_mps2);
    const thirdStr = `${formatAccel(acc.a_third_body_mps2)} (${acc.dominant_perturber_name})`;
    const totStr = formatAccel(acc.a_total_mps2);

    accelRows = `
      <tr><td class="label" style="color: #c084fc; font-weight: 700; border-top: 1px solid rgba(255,255,255,0.1); padding-top: 6px;">Central Gravity (g₀)</td><td class="val" style="color: #c084fc; font-weight: 700; border-top: 1px solid rgba(255,255,255,0.1); padding-top: 6px;">${g0Str}</td></tr>
      <tr><td class="label" style="color: #38bdf8;">J₂ Oblateness Perturbation</td><td class="val" style="color: #38bdf8;">${j2Str}</td></tr>
      <tr><td class="label" style="color: #f59e0b;">3rd-Body Perturbation</td><td class="val" style="color: #f59e0b;">${thirdStr}</td></tr>
      <tr><td class="label" style="color: #34d399; font-weight: 700;">Net Acceleration (a_net)</td><td class="val" style="color: #34d399; font-weight: 700;">${totStr}</td></tr>
    `;
  }

  const specs = document.getElementById('place-specs-table');
  if (specs) {
    specs.innerHTML = `
      <tr><td class="label">Illumination State</td><td class="val">${eclipseHtml}</td></tr>
      <tr><td class="label">Current Altitude</td><td class="val" style="color: #38bdf8; font-weight: 700;">${altKm.toLocaleString(undefined, {maximumFractionDigits: 1})} km</td></tr>
      <tr><td class="label">Orbital Speed (v)</td><td class="val" style="color: #f43f5e; font-weight: 700;">${speedKmS.toFixed(2)} km/s</td></tr>
      <tr><td class="label">Semi-Major Axis (a)</td><td class="val">${smaKm.toLocaleString(undefined, {maximumFractionDigits: 0})} km</td></tr>
      <tr><td class="label">Eccentricity (e)</td><td class="val">${ecc}</td></tr>
      <tr><td class="label">Inclination (i)</td><td class="val">${inc}</td></tr>
      <tr><td class="label">Periapsis Altitude (hp)</td><td class="val" style="color: #ef4444;">${hpKm.toLocaleString(undefined, {maximumFractionDigits: 0})} km</td></tr>
      <tr><td class="label">Apoapsis Altitude (ha)</td><td class="val" style="color: #3b82f6;">${haKm.toLocaleString(undefined, {maximumFractionDigits: 0})} km</td></tr>
      <tr><td class="label">Orbital Period (T)</td><td class="val">${periodStr}</td></tr>
      ${accelRows}
    `;
  }
}

export function openBodyPlaceCard(body, state) {
  const card = document.getElementById('gmap-place-card');
  if (!card || !body) return;

  card.dataset.isSatellite = 'false';
  card.dataset.isSpacePoint = 'false';

  const planetActions = document.getElementById('planet-actions-row');
  const satActions = document.getElementById('satellite-actions-row');
  if (planetActions) planetActions.style.display = 'grid';
  if (satActions) satActions.style.display = 'none';

  document.getElementById('place-icon').innerText = getBodyEmoji(body.name);
  document.getElementById('place-title').innerText = body.name;
  document.getElementById('place-subtitle').innerText = `Celestial Body &bull; Mass: ${body.mass.toExponential(3)} kg`;

  const heading = document.getElementById('place-specs-heading');
  if (heading) heading.innerText = 'Celestial Specifications';

  const specs = document.getElementById('place-specs-table');
  if (specs) {
    const speed = Math.hypot(body.vel[0], body.vel[1], body.vel[2]) / 1e3;
    const distSun = Math.hypot(body.pos[0], body.pos[1], body.pos[2]) / AU_M;
    specs.innerHTML = `
      <tr><td class="label">Speed</td><td class="val">${speed.toFixed(2)} km/s</td></tr>
      <tr><td class="label">Solar Distance</td><td class="val">${distSun.toFixed(3)} AU</td></tr>
      <tr><td class="label">Radius</td><td class="val">${(body.radius || 0).toLocaleString()} km</td></tr>
      <tr><td class="label">Color Code</td><td class="val"><span style="color: ${body.color};">●</span> ${body.color}</td></tr>
    `;
  }

  updatePlaceTugOfWar(body, state);
  card.style.display = 'block';
}

export function updatePlaceTugOfWar(b, state) {
  const table = document.getElementById('place-tug-table');
  const badge = document.getElementById('place-tug-badge');
  const note = document.getElementById('place-tug-insight');
  if (!table || !b || !state.bodies || state.bodies.length < 2) return;

  const G_STANDARD = 6.67430e-11;
  const forces = [];

  for (let j = 0; j < state.bodies.length; j++) {
    const other = state.bodies[j];
    if (other.name === b.name) continue;
    const dx = other.pos[0] - b.pos[0];
    const dy = other.pos[1] - b.pos[1];
    const dz = other.pos[2] - b.pos[2];
    const r2 = dx * dx + dy * dy + dz * dz;
    if (r2 <= 1e-12) continue;
    const fMag = (G_STANDARD * b.mass * other.mass) / r2;
    forces.push({ name: other.name, color: other.color, fMag });
  }

  forces.sort((a, b) => b.fMag - a.fMag);
  const totalF = forces.reduce((sum, f) => sum + f.fMag, 0);

  table.innerHTML = '';
  if (badge) badge.innerText = b.name;

  forces.slice(0, 4).forEach(f => {
    const pct = totalF > 0 ? ((f.fMag / totalF) * 100).toFixed(1) : '0.0';
    const row = document.createElement('tr');
    row.innerHTML = `
      <td class="label" style="color: ${f.color}; font-weight: 600;">${f.name} Pull</td>
      <td class="val">${f.fMag.toExponential(2)} N <span style="color: var(--text-muted); font-size: 10px;">(${pct}%)</span></td>
    `;
    table.appendChild(row);
  });

  if (note) {
    if (b.name === 'Moon') {
      note.innerHTML = `<strong>Tug-of-War Insight:</strong> Sun pull (${forces[0]?.fMag.toExponential(2)} N) is <strong>2.20&times;</strong> Earth pull, yet Moon stays stably bound within Earth's Hill sphere (61,500 km)!`;
    } else if (b.name === 'Earth') {
      note.innerHTML = `<strong>Gravitational Anchor:</strong> Sun accounts for 99.47% of pull. Jupiter exerts secular ~1.46 &times; 10¹⁸ N perturbation.`;
    } else {
      note.innerHTML = `Resultant dominant pull: ${forces[0]?.name || 'N/A'} (${((forces[0]?.fMag / (totalF || 1)) * 100).toFixed(1)}%).`;
    }
  }
}

export function setupUIInteractions(state, callbacks) {
  // 1. Google Maps Layers Widget (Open / Close Menu)
  const layersBtn = document.getElementById('btn-gmap-layers');
  const layersMenu = document.getElementById('gmap-layers-menu');
  if (layersBtn && layersMenu) {
    layersBtn.addEventListener('click', (e) => {
      e.stopPropagation();
      const isVisible = layersMenu.style.display === 'flex';
      layersMenu.style.display = isVisible ? 'none' : 'flex';
    });
    document.addEventListener('click', (e) => {
      if (!e.target.closest('#gmap-layers-widget')) {
        layersMenu.style.display = 'none';
      }
    });
  }

  // 2. Gravitational Centric Frame Dropdown (Inside Layers Menu)
  const selCentric = document.getElementById('select-gravitational-frame');
  if (selCentric) {
    selCentric.addEventListener('change', (e) => {
      state.gravitationalFrameMode = 'manual';
      callbacks.onCentricChange(e.target.value);
    });
  }

  // 3. Centric Chip Click (cycle frames)
  const centricChip = document.getElementById('chip-centric-indicator') || document.getElementById('centric-view-chip');
  if (centricChip) {
    centricChip.addEventListener('click', () => {
      const frames = ['Sun', 'Earth', 'Moon', 'Jupiter', 'Venus', 'Mars'];
      const curIdx = frames.indexOf(state.activeCentricBody);
      const nextFrame = frames[(curIdx + 1) % frames.length];
      callbacks.onCentricChange(nextFrame);
    });
  }

  // 4. Quick Chips in Omnibox row
  document.querySelectorAll('.gmap-chip[data-centric]').forEach(chip => {
    chip.addEventListener('click', () => {
      state.gravitationalFrameMode = 'manual';
      callbacks.onCentricChange(chip.dataset.centric);
    });
  });

  document.querySelectorAll('.gmap-chip[data-body]').forEach(chip => {
    chip.addEventListener('click', () => {
      const bName = chip.dataset.body;
      const bIdx = state.bodies.findIndex(b => b.name === bName);
      if (bIdx >= 0 && callbacks.onSelectBody) {
        callbacks.onSelectBody(bIdx);
      }
    });
  });

  const chipGenOrbits = document.getElementById('chip-generate-orbits');
  if (chipGenOrbits) {
    chipGenOrbits.addEventListener('click', () => {
      if (callbacks.onGenerateOrbits) callbacks.onGenerateOrbits('swarm', 300);
    });
  }

  const chipClearOrbits = document.getElementById('chip-clear-orbits');
  if (chipClearOrbits) {
    chipClearOrbits.addEventListener('click', () => {
      if (callbacks.onClearOrbits) callbacks.onClearOrbits();
    });
  }

  const chipPerturb = document.getElementById('chip-perturbation-mode');
  if (chipPerturb) {
    chipPerturb.addEventListener('click', () => {
      if (callbacks.onCyclePerturbationMode) callbacks.onCyclePerturbationMode();
    });
  }

  // 5. Satellite Inspector Actions
  const btnTrack = document.getElementById('btn-sat-track');
  if (btnTrack) {
    btnTrack.addEventListener('click', () => {
      state.isTrackingSatellite = !state.isTrackingSatellite;
      btnTrack.classList.toggle('active', state.isTrackingSatellite);
    });
  }

  const btnIsolate = document.getElementById('btn-sat-isolate');
  if (btnIsolate) {
    btnIsolate.addEventListener('click', () => {
      state.isolateSelectedOrbit = !state.isolateSelectedOrbit;
      btnIsolate.classList.toggle('active', state.isolateSelectedOrbit);
    });
  }

  const btnFlyTo = document.getElementById('btn-sat-focus') || document.getElementById('btn-sat-flyto');
  if (btnFlyTo) {
    btnFlyTo.addEventListener('click', () => {
      if (callbacks.onFlyToSatellite) callbacks.onFlyToSatellite();
    });
  }

  const btnRelease = document.getElementById('btn-sat-release');
  if (btnRelease) {
    btnRelease.addEventListener('click', () => {
      if (callbacks.onDeselectSatellite) callbacks.onDeselectSatellite();
    });
  }

  // Close place card button
  const btnCloseCard = document.getElementById('btn-place-close') || document.getElementById('btn-close-place-card');
  if (btnCloseCard) {
    btnCloseCard.addEventListener('click', () => {
      const card = document.getElementById('gmap-place-card');
      if (card) card.style.display = 'none';
      if (callbacks.onDeselectSatellite) callbacks.onDeselectSatellite();
    });
  }

  // Planet card Fly To & Tug-of-War buttons
  const btnPlaceFly = document.getElementById('btn-place-flyto');
  if (btnPlaceFly) {
    btnPlaceFly.addEventListener('click', () => {
      if (callbacks.onFlyToBody) callbacks.onFlyToBody(state.selectedBodyIndex);
    });
  }

  const btnPlaceTug = document.getElementById('btn-place-tug');
  if (btnPlaceTug) {
    btnPlaceTug.addEventListener('click', () => {
      const tugCard = document.getElementById('place-tug-card');
      if (tugCard) {
        tugCard.style.display = (tugCard.style.display === 'none' || !tugCard.style.display) ? 'block' : 'none';
      }
    });
  }

  const btnPlaceHill = document.getElementById('btn-place-hill');
  if (btnPlaceHill) {
    btnPlaceHill.addEventListener('click', () => {
      if (callbacks.onFocusHillSphere) callbacks.onFocusHillSphere();
    });
  }

  // 6. View Mode Toggle (Google Maps vs Full Cockpit)
  const setViewMode = (mode) => {
    state.viewMode = mode;
    const leftPanel = document.getElementById('left-panel');
    const rightPanel = document.getElementById('right-panel');
    const bottomPanel = document.getElementById('bottom-panel');
    const bottomToolbar = document.getElementById('bottom-toolbar');
    const gmapSearch = document.getElementById('gmap-search-container');
    const gmapControls = document.getElementById('gmap-controls-stack');
    const gmapScale = document.getElementById('gmap-scale-container');
    const gmapLayers = document.getElementById('gmap-layers-widget');
    const gmapStatusBar = document.getElementById('gmap-status-bar');
    const toggleBtn = document.getElementById('btn-toggle-view-mode');

    if (mode === 'google_maps') {
      if (leftPanel) leftPanel.style.display = 'none';
      if (rightPanel) rightPanel.style.display = 'none';
      if (bottomPanel) bottomPanel.style.display = 'none';
      if (bottomToolbar) bottomToolbar.style.display = 'none';
      if (gmapSearch) gmapSearch.style.display = 'flex';
      if (gmapControls) gmapControls.style.display = 'flex';
      if (gmapScale) gmapScale.style.display = 'flex';
      if (gmapLayers) gmapLayers.style.display = 'block';
      if (gmapStatusBar) gmapStatusBar.style.display = 'block';
      const legend = document.getElementById('visviva-legend');
      if (legend && state.activeCentricBody !== 'Sun') legend.style.display = 'flex';
      if (toggleBtn) {
        toggleBtn.innerText = '🗺️ Google Maps View';
        toggleBtn.classList.add('active');
      }
    } else {
      if (leftPanel) leftPanel.style.display = 'block';
      if (rightPanel) rightPanel.style.display = 'block';
      if (bottomPanel) bottomPanel.style.display = 'flex';
      if (bottomToolbar) bottomToolbar.style.display = 'flex';
      if (gmapSearch) gmapSearch.style.display = 'none';
      if (gmapControls) gmapControls.style.display = 'none';
      if (gmapScale) gmapScale.style.display = 'none';
      if (gmapLayers) gmapLayers.style.display = 'none';
      if (gmapStatusBar) gmapStatusBar.style.display = 'none';
      const legend = document.getElementById('visviva-legend');
      if (legend) legend.style.display = 'none';
      const card = document.getElementById('gmap-place-card');
      if (card) card.style.display = 'none';
      if (toggleBtn) {
        toggleBtn.innerText = '🎛️ Full Cockpit';
        toggleBtn.classList.remove('active');
      }
      if (callbacks.onCockpitOpen) callbacks.onCockpitOpen();
    }
  };

  const toggleViewBtn = document.getElementById('btn-toggle-view-mode');
  if (toggleViewBtn) {
    toggleViewBtn.addEventListener('click', () => {
      const newMode = (state.viewMode === 'google_maps' || !state.viewMode) ? 'cockpit' : 'google_maps';
      setViewMode(newMode);
    });
  }

  const btnPlaceCockpit = document.getElementById('btn-place-cockpit-toggle');
  if (btnPlaceCockpit) {
    btnPlaceCockpit.addEventListener('click', () => {
      setViewMode('cockpit');
    });
  }

  // 7. Google Maps Floating Controls Stack
  const btnZoomIn = document.getElementById('btn-gmap-zoom-in');
  if (btnZoomIn) {
    btnZoomIn.addEventListener('click', () => {
      if (callbacks.onZoom) callbacks.onZoom(0.7);
    });
  }
  const btnZoomOut = document.getElementById('btn-gmap-zoom-out');
  if (btnZoomOut) {
    btnZoomOut.addEventListener('click', () => {
      if (callbacks.onZoom) callbacks.onZoom(1.4);
    });
  }

  const btnTilt = document.getElementById('btn-gmap-tilt');
  if (btnTilt) {
    btnTilt.addEventListener('click', () => {
      if (callbacks.onToggleTilt) callbacks.onToggleTilt();
    });
  }

  const btnRecenter = document.getElementById('btn-gmap-recenter');
  if (btnRecenter) {
    btnRecenter.addEventListener('click', () => {
      if (callbacks.onRecenter) callbacks.onRecenter();
    });
  }

  const btnCompass = document.getElementById('btn-gmap-compass');
  if (btnCompass) {
    btnCompass.addEventListener('click', () => {
      if (callbacks.onCompass) callbacks.onCompass();
    });
  }

  // 8. Top Header Simulation Controls
  const selPreset = document.getElementById('preset-select');
  if (selPreset) {
    selPreset.addEventListener('change', (e) => {
      if (callbacks.onPresetChange) callbacks.onPresetChange(e.target.value);
    });
  }

  const btnPlay = document.getElementById('btn-play');
  if (btnPlay) {
    btnPlay.addEventListener('click', () => {
      state.isPlaying = !state.isPlaying;
      btnPlay.innerText = state.isPlaying ? 'Pause' : 'Play';
      btnPlay.classList.toggle('active', state.isPlaying);
    });
  }

  const btnReset = document.getElementById('btn-reset');
  if (btnReset) {
    btnReset.addEventListener('click', () => {
      if (callbacks.onReset) callbacks.onReset();
    });
  }

  const btnScale = document.getElementById('btn-scale-mode');
  if (btnScale) {
    btnScale.addEventListener('click', () => {
      if (callbacks.onToggleScaleMode) callbacks.onToggleScaleMode();
    });
  }

  const selFocus = document.getElementById('camera-focus-select');
  if (selFocus) {
    selFocus.addEventListener('change', (e) => {
      if (callbacks.onCameraFocus) callbacks.onCameraFocus(e.target.value);
    });
  }

  // 9. Classic Cockpit Left Panel Controls
  const selInt = document.getElementById('integrator-select');
  if (selInt) {
    selInt.addEventListener('change', (e) => {
      state.integrator = e.target.value;
      const badge = document.getElementById('integrator-badge');
      if (badge) {
        const names = { yoshida4: 'Yoshida 4th', yoshida6: 'Yoshida 6th', leapfrog: 'Leapfrog', hermite4: 'Hermite 4th' };
        badge.innerText = names[state.integrator] || state.integrator;
      }
    });
  }

  const speedSlider = document.getElementById('time-speed-slider');
  const speedVal = document.getElementById('time-speed-val');
  if (speedSlider) {
    speedSlider.addEventListener('input', (e) => {
      state.timeMultiplier = parseFloat(e.target.value);
      if (speedVal) speedVal.innerText = `${state.timeMultiplier}x`;
    });
  }

  const substepsSlider = document.getElementById('substeps-slider');
  const substepsVal = document.getElementById('substeps-val');
  if (substepsSlider) {
    substepsSlider.addEventListener('input', (e) => {
      state.subSteps = parseInt(e.target.value, 10);
      if (substepsVal) substepsVal.innerText = `${state.subSteps} steps`;
    });
  }

  const btnToggleForces = document.getElementById('btn-toggle-forces');
  if (btnToggleForces) {
    btnToggleForces.addEventListener('click', () => {
      state.showForces = !state.showForces;
      btnToggleForces.innerText = `Forces: ${state.showForces ? 'ON' : 'OFF'}`;
      btnToggleForces.classList.toggle('active', state.showForces);
      if (callbacks.onToggleForces) callbacks.onToggleForces(state.showForces);
    });
  }

  const magSlider = document.getElementById('magnifier-slider');
  const magVal = document.getElementById('magnifier-val');
  if (magSlider) {
    magSlider.addEventListener('input', (e) => {
      state.perturbationMagnifier = parseFloat(e.target.value);
      if (magVal) magVal.innerText = `${state.perturbationMagnifier.toLocaleString()}x`;
      if (callbacks.onUpdateMagnifier) callbacks.onUpdateMagnifier(state.perturbationMagnifier);
    });
  }

  const btnToggleTethers = document.getElementById('btn-toggle-tethers');
  if (btnToggleTethers) {
    btnToggleTethers.addEventListener('click', () => {
      state.showTethers = !state.showTethers;
      btnToggleTethers.innerText = `Tethers: ${state.showTethers ? 'ON' : 'OFF'}`;
      btnToggleTethers.classList.toggle('active', state.showTethers);
      if (callbacks.onToggleTethers) callbacks.onToggleTethers(state.showTethers);
    });
  }

  const btnToggleSpacetime = document.getElementById('btn-toggle-spacetime');
  if (btnToggleSpacetime) {
    btnToggleSpacetime.addEventListener('click', () => {
      state.showSpacetime = !state.showSpacetime;
      btnToggleSpacetime.innerText = `Spacetime: ${state.showSpacetime ? 'ON' : 'OFF'}`;
      btnToggleSpacetime.classList.toggle('active', state.showSpacetime);
      if (callbacks.onToggleSpacetime) callbacks.onToggleSpacetime(state.showSpacetime);
    });
  }

  const btnToggleHills = document.getElementById('btn-toggle-hills');
  if (btnToggleHills) {
    btnToggleHills.addEventListener('click', () => {
      state.showHillSpheres = !state.showHillSpheres;
      btnToggleHills.innerText = `Hill Spheres: ${state.showHillSpheres ? 'ON' : 'OFF'}`;
      btnToggleHills.classList.toggle('active', state.showHillSpheres);
      if (callbacks.onToggleHills) callbacks.onToggleHills(state.showHillSpheres);
    });
  }

  const btnToggleGr = document.getElementById('btn-toggle-gr');
  if (btnToggleGr) {
    btnToggleGr.addEventListener('click', () => {
      state.enable_gr = !state.enable_gr;
      btnToggleGr.innerText = `1PN GR: ${state.enable_gr ? 'ON' : 'OFF'}`;
      btnToggleGr.classList.toggle('active', state.enable_gr);
    });
  }

  const btnToggleTrails = document.getElementById('btn-toggle-trails');
  if (btnToggleTrails) {
    btnToggleTrails.addEventListener('click', () => {
      state.showTrails = !state.showTrails;
      btnToggleTrails.innerText = `Trails: ${state.showTrails ? 'ON' : 'OFF'}`;
      btnToggleTrails.classList.toggle('active', state.showTrails);
      if (callbacks.onToggleTrails) callbacks.onToggleTrails(state.showTrails);
    });
  }

  const btnToggleVel = document.getElementById('btn-toggle-vel');
  if (btnToggleVel) {
    btnToggleVel.addEventListener('click', () => {
      state.showVectors = !state.showVectors;
      btnToggleVel.innerText = `Vectors: ${state.showVectors ? 'ON' : 'OFF'}`;
      btnToggleVel.classList.toggle('active', state.showVectors);
      if (callbacks.onToggleVectors) callbacks.onToggleVectors(state.showVectors);
    });
  }

  const btnToggleGrid = document.getElementById('btn-toggle-grid');
  if (btnToggleGrid) {
    btnToggleGrid.addEventListener('click', () => {
      state.showGrid = !state.showGrid;
      btnToggleGrid.innerText = `Grid: ${state.showGrid ? 'ON' : 'OFF'}`;
      btnToggleGrid.classList.toggle('active', state.showGrid);
      if (callbacks.onToggleGrid) callbacks.onToggleGrid(state.showGrid);
    });
  }

  // 10. Classic Cockpit Right Panel Body Selector
  const selBody = document.getElementById('body-selector-dropdown');
  if (selBody) {
    selBody.addEventListener('change', (e) => {
      const idx = parseInt(e.target.value, 10);
      if (callbacks.onSelectBody) callbacks.onSelectBody(idx);
    });
  }

  // 11. Classic Cockpit Bottom Toolbar Camera Views
  const btnCamTop = document.getElementById('btn-camera-top');
  if (btnCamTop) {
    btnCamTop.addEventListener('click', () => {
      if (callbacks.onCameraView) callbacks.onCameraView('top');
    });
  }
  const btnCamSide = document.getElementById('btn-camera-side');
  if (btnCamSide) {
    btnCamSide.addEventListener('click', () => {
      if (callbacks.onCameraView) callbacks.onCameraView('side');
    });
  }
  const btnCamIso = document.getElementById('btn-camera-iso');
  if (btnCamIso) {
    btnCamIso.addEventListener('click', () => {
      if (callbacks.onCameraView) callbacks.onCameraView('iso');
    });
  }

  // 12. Omnibox Search input
  const searchInput = document.getElementById('gmap-search-input');
  const searchClear = document.getElementById('gmap-search-clear');
  const searchDropdown = document.getElementById('gmap-search-dropdown');

  if (searchInput && searchDropdown) {
    searchInput.addEventListener('input', (e) => {
      const q = e.target.value.trim().toLowerCase();
      if (!q) {
        searchDropdown.style.display = 'none';
        return;
      }

      const matches = [];
      state.bodies.forEach((b, idx) => {
        if (b.name.toLowerCase().includes(q)) {
          matches.push({ type: 'body', index: idx, name: b.name, emoji: getBodyEmoji(b.name) });
        }
      });
      state.testParticles.slice(0, 10).forEach(p => {
        if (p.name.toLowerCase().includes(q) || p.anchorBodyName.toLowerCase().includes(q)) {
          matches.push({ type: 'satellite', id: p.id, name: p.name, emoji: '🛰️', sub: `${p.anchorBodyName} Centric` });
        }
      });

      if (matches.length === 0) {
        searchDropdown.innerHTML = '<div style="padding: 10px; font-size: 11px; color: #94a3b8;">No bodies or satellites found</div>';
      } else {
        searchDropdown.innerHTML = matches.map(m => `
          <div class="gmap-search-item" data-type="${m.type}" data-val="${m.index !== undefined ? m.index : m.id}" style="padding: 8px 12px; cursor: pointer; display: flex; align-items: center; gap: 8px; font-size: 12px; border-bottom: 1px solid rgba(255,255,255,0.06);">
            <span>${m.emoji}</span>
            <div><strong>${m.name}</strong> ${m.sub ? `<span style="font-size: 10px; color: #94a3b8;">(${m.sub})</span>` : ''}</div>
          </div>
        `).join('');

        searchDropdown.querySelectorAll('.gmap-search-item').forEach(el => {
          el.addEventListener('click', () => {
            searchDropdown.style.display = 'none';
            searchInput.value = '';
            if (el.dataset.type === 'body') {
              const idx = parseInt(el.dataset.val, 10);
              if (callbacks.onSelectBody) callbacks.onSelectBody(idx);
            } else if (el.dataset.type === 'satellite') {
              if (callbacks.onSelectSatelliteById) callbacks.onSelectSatelliteById(el.dataset.val);
            }
          });
        });
      }
      searchDropdown.style.display = 'block';
    });

    if (searchClear) {
      searchClear.addEventListener('click', () => {
        searchInput.value = '';
        searchDropdown.style.display = 'none';
      });
    }
  }
}
