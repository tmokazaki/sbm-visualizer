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

export function openSatelliteInspector(tp, wasmSwarmEngine, sunRelPos, sunRadiusM) {
  const card = document.getElementById('gmap-place-card');
  if (!card) return;

  card.dataset.isSatellite = 'true';
  card.dataset.isSpacePoint = 'false';

  const planetActions = document.getElementById('planet-actions-row');
  const spaceActions = document.getElementById('space-actions-row');
  const satActions = document.getElementById('satellite-actions-row');
  if (planetActions) planetActions.style.display = 'none';
  if (spaceActions) spaceActions.style.display = 'none';
  if (satActions) satActions.style.display = 'grid';

  const tidalCard = document.getElementById('place-tidal-card');
  if (tidalCard) tidalCard.style.display = 'none';
  const tugCard = document.getElementById('place-tug-card');
  if (tugCard) tugCard.style.display = 'none';

  document.getElementById('place-icon').innerText = '🛰️';
  document.getElementById('place-title').innerText = `${tp.name} (${tp.anchorBodyName} Centric)`;

  const heading = document.getElementById('place-specs-heading');
  if (heading) heading.innerText = 'Orbital Mechanics & Telemetry (Rust / WASM)';

  updateSatelliteInspectorCard(tp, wasmSwarmEngine, sunRelPos, sunRadiusM);
  card.style.display = 'block';
}

export function updateSatelliteInspectorCard(tp, wasmSwarmEngine, sunRelPos, sunRadiusM) {
  const card = document.getElementById('gmap-place-card');
  if (!card || card.dataset.isSatellite !== 'true' || card.style.display === 'none' || !tp) return;

  let telem = null;
  if (wasmSwarmEngine && wasmSwarmEngine.get_satellite_telemetry) {
    try {
      const sunRel = sunRelPos ? new Float64Array(sunRelPos) : new Float64Array([0, AU_M, 0]);
      telem = wasmSwarmEngine.get_satellite_telemetry(tp.index || 0, sunRel, sunRadiusM || 696340e3);
    } catch (_) {}
  }

  const altKm = telem ? telem.altitude_km : 420;
  const speedKmS = telem ? telem.speed_km_s : 7.66;
  const smaKm = telem ? telem.semi_major_axis_km : 6791;
  const ecc = telem ? telem.eccentricity.toFixed(4) : '0.0012';
  const inc = telem ? telem.inclination_deg.toFixed(1) + '°' : '51.6°';
  const hpKm = telem ? telem.periapsis_alt_km : 415;
  const haKm = telem ? telem.apoapsis_alt_km : 425;
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
    `;
  }
}

export function openBodyPlaceCard(body, state) {
  const card = document.getElementById('gmap-place-card');
  if (!card || !body) return;

  card.dataset.isSatellite = 'false';
  card.dataset.isSpacePoint = 'false';

  const planetActions = document.getElementById('planet-actions-row');
  const spaceActions = document.getElementById('space-actions-row');
  const satActions = document.getElementById('satellite-actions-row');
  if (planetActions) planetActions.style.display = 'grid';
  if (spaceActions) spaceActions.style.display = 'none';
  if (satActions) satActions.style.display = 'none';

  document.getElementById('place-icon').innerText = getBodyEmoji(body.name);
  document.getElementById('place-title').innerText = body.name;
  document.getElementById('place-subtitle').innerText = `Celestial Body &bull; Mass: ${body.mass.toExponential(3)} kg`;

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

  card.style.display = 'block';
}

export function setupUIInteractions(state, callbacks) {
  // Gravitational Centric Frame Dropdown
  const selCentric = document.getElementById('select-gravitational-frame');
  if (selCentric) {
    selCentric.addEventListener('change', (e) => {
      state.gravitationalFrameMode = 'manual';
      callbacks.onCentricChange(e.target.value);
    });
  }

  // Centric Chip Click (cycle frames)
  const centricChip = document.getElementById('chip-centric-indicator') || document.getElementById('centric-view-chip');
  if (centricChip) {
    centricChip.addEventListener('click', () => {
      const frames = ['Sun', 'Earth', 'Moon', 'Jupiter', 'Venus', 'Mars'];
      const curIdx = frames.indexOf(state.activeCentricBody);
      const nextFrame = frames[(curIdx + 1) % frames.length];
      callbacks.onCentricChange(nextFrame);
    });
  }

  // Quick Chips in Omnibox row
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

  // Satellite Inspector Actions
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

  // Planet card Fly To button
  const btnPlaceFly = document.getElementById('btn-place-flyto');
  if (btnPlaceFly) {
    btnPlaceFly.addEventListener('click', () => {
      if (callbacks.onFlyToBody) callbacks.onFlyToBody(state.selectedBodyIndex);
    });
  }

  // Omnibox Search input
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
