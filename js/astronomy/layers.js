/**
 * Three.js Visual Layers: Spacetime Curvature, Vector Fields, Shadow Cones, Basins, and Reticles.
 * Pure rendering components that consume WASM physics data.
 */
import { AU_M, PERCEPTUAL_PLANET_RADII } from './constants.js';
import { getCentricFrameConfig, isBodyRelevantToCentricMode } from './centric_config.js';

// --- Spacetime Curvature Grid ---
export function createSpacetimeMesh() {
  const size = 60;
  const divisions = 60;
  const geometry = new THREE.PlaneGeometry(size, size, divisions, divisions);
  geometry.rotateX(-Math.PI / 2);

  const material = new THREE.MeshBasicMaterial({
    color: 0x1e3a8a,
    wireframe: true,
    transparent: true,
    opacity: 0.25
  });

  const mesh = new THREE.Mesh(geometry, material);
  mesh.name = 'spacetimeMesh';
  mesh.position.y = -0.5;
  return mesh;
}

export function updateSpacetimeMesh(mesh, bodies, toSceneCoords) {
  if (!mesh || !mesh.visible || !bodies || bodies.length === 0) return;
  const posAttr = mesh.geometry.attributes.position;
  const count = posAttr.count;

  for (let i = 0; i < count; i++) {
    const vx = posAttr.getX(i);
    const vz = posAttr.getZ(i);
    let totalWell = 0;

    for (let j = 0; j < bodies.length; j++) {
      const b = bodies[j];
      const sc = toSceneCoords(b.pos);
      const dx = vx - sc.x;
      const dz = vz - sc.y;
      const distSq = dx * dx + dz * dz + 0.15;
      const massScale = Math.cbrt(b.mass / 1e24);
      totalWell += massScale * 0.08 / Math.sqrt(distSq);
    }
    posAttr.setY(i, -Math.min(totalWell, 3.5));
  }
  posAttr.needsUpdate = true;
}

// --- Shadow Cone Geometry (Umbra & Penumbra) ---
export function createShadowConeGroup() {
  const group = new THREE.Group();
  group.name = 'shadowConeGroup';
  return group;
}

export function buildShadowConeMeshes(group, anchorName, scaleMode, sceneScale) {
  while (group.children.length > 0) {
    const obj = group.children[0];
    group.remove(obj);
    if (obj.geometry) obj.geometry.dispose();
    if (obj.material) obj.material.dispose();
  }

  if (anchorName === 'Sun') {
    group.visible = false;
    return;
  }

  const cfg = getCentricFrameConfig(anchorName);
  const span = cfg.spanScene || 3.6;
  const halfSpan = cfg.halfSpanPhys_m || 1_200_000e3;

  let rPlanetScene = 0.28;
  let umbraLengthScene = 4.14;
  if (anchorName === 'Earth') {
    rPlanetScene = (scaleMode === 'perceptual') ? 0.28 : (6371e3 * sceneScale);
    umbraLengthScene = (scaleMode === 'perceptual') ? (1.38e9 / halfSpan) * span : (1.38e9 * sceneScale);
  } else if (anchorName === 'Moon') {
    rPlanetScene = (scaleMode === 'perceptual') ? 0.09 : (1737e3 * sceneScale);
    umbraLengthScene = (scaleMode === 'perceptual') ? (374e6 / halfSpan) * span : (374e6 * sceneScale);
  } else if (anchorName === 'Jupiter') {
    rPlanetScene = (scaleMode === 'perceptual') ? 0.55 : (69911e3 * sceneScale);
    umbraLengthScene = (scaleMode === 'perceptual') ? 14.0 : (86.8e9 * sceneScale);
  } else {
    rPlanetScene = 0.25;
    umbraLengthScene = 4.0;
  }

  const coneHeight = Math.max(1.0, umbraLengthScene);

  // 1. Umbra Core Cone
  const umbraGeo = new THREE.ConeGeometry(rPlanetScene, coneHeight, 32, 1, true);
  umbraGeo.translate(0, coneHeight / 2, 0);
  const umbraMat = new THREE.MeshBasicMaterial({
    color: 0x071529,
    transparent: true,
    opacity: 0.32,
    depthWrite: false,
    side: THREE.DoubleSide
  });
  const umbraMesh = new THREE.Mesh(umbraGeo, umbraMat);
  umbraMesh.name = 'umbraConeMesh';
  group.add(umbraMesh);

  // 2. Umbra Outer Wireframe Boundary Rim
  const umbraRimGeo = new THREE.ConeGeometry(rPlanetScene * 1.008, coneHeight, 16, 1, true);
  umbraRimGeo.translate(0, coneHeight / 2, 0);
  const umbraRimMat = new THREE.MeshBasicMaterial({
    color: 0x38bdf8,
    wireframe: true,
    transparent: true,
    opacity: 0.16,
    depthWrite: false
  });
  group.add(new THREE.Mesh(umbraRimGeo, umbraRimMat));

  // 3. Distance Cross-Section Range Rings
  const ringFractions = [0.25, 0.50, 0.75];
  ringFractions.forEach(frac => {
    const ringY = frac * coneHeight;
    const ringR = rPlanetScene * (1.0 - frac);
    const ringPts = [];
    const segs = 32;
    for (let s = 0; s <= segs; s++) {
      const phi = (s / segs) * 2 * Math.PI;
      ringPts.push(new THREE.Vector3(ringR * Math.cos(phi), ringY, ringR * Math.sin(phi)));
    }
    const ringGeo = new THREE.BufferGeometry().setFromPoints(ringPts);
    const ringMat = new THREE.LineBasicMaterial({
      color: 0x38bdf8,
      transparent: true,
      opacity: 0.22,
      depthWrite: false
    });
    group.add(new THREE.Line(ringGeo, ringMat));
  });

  // 4. Solar Ray Beam Vector
  const rayLen = rPlanetScene * 4.5;
  const rayPoints = [
    new THREE.Vector3(0, 0, 0),
    new THREE.Vector3(0, -rayLen, 0)
  ];
  const rayGeo = new THREE.BufferGeometry().setFromPoints(rayPoints);
  const rayMat = new THREE.LineBasicMaterial({
    color: 0xfbbf24,
    transparent: true,
    opacity: 0.85
  });
  group.add(new THREE.Line(rayGeo, rayMat));

  // Solar Ray Arrowhead
  const arrowGeo = new THREE.ConeGeometry(rPlanetScene * 0.18, rPlanetScene * 0.38, 12);
  arrowGeo.translate(0, -rayLen, 0);
  arrowGeo.rotateX(Math.PI);
  group.add(new THREE.Mesh(arrowGeo, new THREE.MeshBasicMaterial({ color: 0xfbbf24 })));
}

export function updateShadowConeTransform(group, anchorBody, sunBody, toSceneCoords) {
  if (!group || !anchorBody || !sunBody) {
    if (group) group.visible = false;
    return;
  }

  const anchorScPos = (anchorBody.name === 'Sun') ? new THREE.Vector3(0, 0, 0) : toSceneCoords(anchorBody.pos);
  group.position.copy(anchorScPos);

  const dx = anchorBody.pos[0] - sunBody.pos[0];
  const dy = anchorBody.pos[1] - sunBody.pos[1];
  const dz = anchorBody.pos[2] - sunBody.pos[2];
  const d = Math.hypot(dx, dy, dz) || 1.0;
  const shadowDir = new THREE.Vector3(dx / d, dy / d, dz / d).normalize();

  group.quaternion.setFromUnitVectors(new THREE.Vector3(0, 1, 0), shadowDir);
}

// --- Satellite Targeting Reticle & Periapsis / Apoapsis Markers ---
export function createSatelliteReticleLayer(scene) {
  const group = new THREE.Group();
  group.name = 'satelliteReticleGroup';

  const ringRadius = 0.048;
  const ringGeo = new THREE.RingGeometry(ringRadius * 0.85, ringRadius, 32);
  const ringMat = new THREE.MeshBasicMaterial({
    color: 0x38bdf8,
    side: THREE.DoubleSide,
    transparent: true,
    opacity: 0.85,
    depthWrite: false
  });
  group.add(new THREE.Mesh(ringGeo, ringMat));

  const tickGeo = new THREE.BufferGeometry();
  const tickLen = ringRadius * 0.45;
  const tickPts = [
    new THREE.Vector3(0, ringRadius, 0), new THREE.Vector3(0, ringRadius + tickLen, 0),
    new THREE.Vector3(0, -ringRadius, 0), new THREE.Vector3(0, -ringRadius - tickLen, 0),
    new THREE.Vector3(ringRadius, 0, 0), new THREE.Vector3(ringRadius + tickLen, 0, 0),
    new THREE.Vector3(-ringRadius, 0, 0), new THREE.Vector3(-ringRadius - tickLen, 0, 0),
  ];
  tickGeo.setFromPoints(tickPts);
  const tickMat = new THREE.LineBasicMaterial({ color: 0x38bdf8, transparent: true, opacity: 0.9 });
  group.add(new THREE.LineSegments(tickGeo, tickMat));
  group.visible = false;
  scene.add(group);

  // Periapsis Marker
  const periapsisMesh = new THREE.Group();
  periapsisMesh.name = 'periapsisMarker';
  const pDot = new THREE.Mesh(
    new THREE.SphereGeometry(0.035, 16, 16),
    new THREE.MeshBasicMaterial({ color: 0xef4444 })
  );
  const pRing = new THREE.Mesh(
    new THREE.RingGeometry(0.048, 0.058, 32),
    new THREE.MeshBasicMaterial({ color: 0xef4444, side: THREE.DoubleSide, transparent: true, opacity: 0.9 })
  );
  periapsisMesh.add(pDot);
  periapsisMesh.add(pRing);
  periapsisMesh.visible = false;
  scene.add(periapsisMesh);

  // Apoapsis Marker
  const apoapsisMesh = new THREE.Group();
  apoapsisMesh.name = 'apoapsisMarker';
  const aDot = new THREE.Mesh(
    new THREE.SphereGeometry(0.035, 16, 16),
    new THREE.MeshBasicMaterial({ color: 0x3b82f6 })
  );
  const aRing = new THREE.Mesh(
    new THREE.RingGeometry(0.048, 0.058, 32),
    new THREE.MeshBasicMaterial({ color: 0x3b82f6, side: THREE.DoubleSide, transparent: true, opacity: 0.9 })
  );
  apoapsisMesh.add(aDot);
  apoapsisMesh.add(aRing);
  apoapsisMesh.visible = false;
  scene.add(apoapsisMesh);

  return { reticleGroup: group, periapsisMarker: periapsisMesh, apoapsisMarker: apoapsisMesh };
}

// --- Gravitational Basins Layer ---
export function createGravitationalBasinsLayer(scene) {
  const group = new THREE.Group();
  group.name = 'gravitationalBasinsGroup';

  const sunGeo = new THREE.CircleGeometry(48, 64);
  const sunMat = new THREE.MeshBasicMaterial({
    color: 0xfbbf24,
    transparent: true,
    opacity: 0.06,
    side: THREE.DoubleSide
  });
  const sunDisc = new THREE.Mesh(sunGeo, sunMat);
  sunDisc.name = 'sunBasinDisc';
  sunDisc.position.z = -0.01;
  group.add(sunDisc);

  scene.add(group);
  return group;
}

export function buildPlanetBasins(group, bodies, toSceneCoords, sceneScale) {
  // Clear any existing planet basin meshes (leaving sunDisc intact)
  for (let i = group.children.length - 1; i >= 0; i--) {
    const c = group.children[i];
    if (c.name !== 'sunBasinDisc') {
      group.remove(c);
      if (c.geometry) c.geometry.dispose();
      if (c.material) c.material.dispose();
    }
  }

  for (let i = 0; i < bodies.length; i++) {
    const b = bodies[i];
    if (b.name === 'Sun') continue;

    // Chebotarev Gravitational Sphere of Influence Radius: R_c = a * (m / M_sun)^(2/5)
    const distSun = Math.hypot(b.pos[0], b.pos[1], b.pos[2]);
    const rCheb_m = distSun * Math.pow(b.mass / 1.98847e30, 0.4);
    const rChebScene = Math.max(0.4, rCheb_m * sceneScale);

    const geo = new THREE.RingGeometry(rChebScene * 0.98, rChebScene, 48);
    const mat = new THREE.MeshBasicMaterial({
      color: new THREE.Color(b.color),
      side: THREE.DoubleSide,
      transparent: true,
      opacity: 0.28
    });
    const ring = new THREE.Mesh(geo, mat);
    ring.userData = { bodyIndex: i };
    const sc = toSceneCoords(b.pos);
    ring.position.set(sc.x, sc.y, -0.005);
    group.add(ring);
  }
}

export function updatePlanetBasinPositions(group, bodies, toSceneCoords) {
  if (!group || !group.visible) return;
  group.children.forEach(child => {
    if (child.userData && child.userData.bodyIndex !== undefined) {
      const b = bodies[child.userData.bodyIndex];
      if (b) {
        const sc = toSceneCoords(b.pos);
        child.position.x = sc.x;
        child.position.y = sc.y;
      }
    }
  });
}

// --- Spatial Vector Grid Layer ---
export function createSpatialVectorGridLayer(scene) {
  const group = new THREE.Group();
  group.name = 'spatialVectorGridGroup';
  scene.add(group);
  return group;
}

// --- Equipotential Contours Layer ---
export function createEquipotentialContoursLayer(scene) {
  const group = new THREE.Group();
  group.name = 'equipotentialContoursGroup';
  scene.add(group);
  return group;
}

export function buildEquipotentialContours(group, anchorBody, toSceneCoords) {
  while (group.children.length > 0) {
    const obj = group.children[0];
    group.remove(obj);
    if (obj.geometry) obj.geometry.dispose();
    if (obj.material) obj.material.dispose();
  }

  if (!anchorBody) return;
  const cfg = getCentricFrameConfig(anchorBody.name);
  const radii = cfg.contourRadiiScene || [0.5, 1.0, 1.8, 2.5];
  const sc = (anchorBody.name === 'Sun') ? new THREE.Vector3(0, 0, 0) : toSceneCoords(anchorBody.pos);

  radii.forEach(r => {
    const geo = new THREE.RingGeometry(r * 0.995, r, 64);
    const mat = new THREE.MeshBasicMaterial({
      color: cfg.arrowColor,
      side: THREE.DoubleSide,
      transparent: true,
      opacity: 0.18,
      depthWrite: false
    });
    const ring = new THREE.Mesh(geo, mat);
    ring.position.set(sc.x, sc.y, -0.006);
    group.add(ring);
  });
}

// --- Space Probe Layer ---
export function createSpaceProbeLayer(scene) {
  const group = new THREE.Group();
  group.name = 'spaceProbeGroup';

  const reticleGeo = new THREE.RingGeometry(0.08, 0.095, 32);
  const reticleMat = new THREE.MeshBasicMaterial({
    color: 0x38bdf8,
    side: THREE.DoubleSide,
    transparent: true,
    opacity: 0.9
  });
  const reticleMesh = new THREE.Mesh(reticleGeo, reticleMat);
  group.add(reticleMesh);

  group.visible = false;
  scene.add(group);
  return group;
}
