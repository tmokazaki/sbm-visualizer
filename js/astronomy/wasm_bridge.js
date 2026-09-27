/**
 * WebAssembly Bridge for SBM Astronomical Mechanics & Dynamics.
 * Connects high-performance Rust WASM numerical kernels to browser rendering.
 */
import initWasm, {
  WasmNBodyEngine,
  WasmCentricSwarmEngine,
  wasm_compute_shadow_cone,
  wasm_compute_spatial_field_point
} from '/pkg/sbm_wasm.js';

let wasmReady = false;
let wasmInitPromise = null;

export async function initWasmEngine() {
  if (wasmReady) return true;
  if (wasmInitPromise) return wasmInitPromise;

  wasmInitPromise = (async () => {
    try {
      await initWasm();
      wasmReady = true;
      console.log('✅ SBM WebAssembly Physics Engine initialized successfully.');
      return true;
    } catch (err) {
      console.error('❌ Failed to initialize SBM WebAssembly Engine:', err);
      throw err;
    }
  })();

  return wasmInitPromise;
}

export function isWasmReady() {
  return wasmReady;
}

export function createNBodyEngine(presetId) {
  if (!wasmReady) {
    throw new Error('WebAssembly Engine not initialized. Call await initWasmEngine() first.');
  }
  return new WasmNBodyEngine(presetId);
}

export function createSwarmEngine(anchorName, preset, count) {
  if (!wasmReady) {
    throw new Error('WebAssembly Engine not initialized. Call await initWasmEngine() first.');
  }
  return new WasmCentricSwarmEngine(anchorName, preset, count);
}

export function computeShadowCone(bodyPos, bodyRadius, sunPos, sunRadius) {
  if (!wasmReady) return null;
  return wasm_compute_shadow_cone(
    bodyPos[0], bodyPos[1], bodyPos[2], bodyRadius,
    sunPos[0], sunPos[1], sunPos[2], sunRadius
  );
}

export function computeSpatialFieldPoint(nbodyEngine, px, py, pz) {
  if (!wasmReady || !nbodyEngine) return null;
  return wasm_compute_spatial_field_point(nbodyEngine, px, py, pz);
}
