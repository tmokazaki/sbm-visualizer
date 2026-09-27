/* @ts-self-types="./sbm_wasm.d.ts" */

/**
 * Swarm particle propagation engine running in pure WebAssembly.
 */
export class WasmCentricSwarmEngine {
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WasmCentricSwarmEngineFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmcentricswarmengine_free(ptr, 0);
    }
    /**
     * Evaluates eclipse state (0 = Sunlit, 1 = Penumbra, 2 = Umbra) for all particles.
     * @param {Float64Array} sun_rel_pos
     * @param {number} sun_radius_m
     * @returns {Int32Array}
     */
    get_eclipse_states_flat(sun_rel_pos, sun_radius_m) {
        const ptr0 = passArrayF64ToWasm0(sun_rel_pos, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmcentricswarmengine_get_eclipse_states_flat(this.__wbg_ptr, ptr0, len0, sun_radius_m);
        return ret;
    }
    /**
     * Returns flat array of the closed Keplerian ellipse orbit vertices for particle `index`.
     * @param {number} index
     * @returns {Float64Array}
     */
    get_orbit_trail_flat(index) {
        const ret = wasm.wasmcentricswarmengine_get_orbit_trail_flat(this.__wbg_ptr, index);
        return ret;
    }
    /**
     * Gets the active perturbation mode.
     * @returns {number}
     */
    get_perturbation_mode() {
        const ret = wasm.wasmcentricswarmengine_get_perturbation_mode(this.__wbg_ptr);
        return ret;
    }
    /**
     * Returns flat array of particle relative positions: [x0, y0, z0, x1, y1, z1, ...].
     * @returns {Float64Array}
     */
    get_positions_flat() {
        const ret = wasm.wasmcentricswarmengine_get_positions_flat(this.__wbg_ptr);
        return ret;
    }
    /**
     * Computes full satellite telemetry (Kepler elements, altitude, speed, period, eclipse, acceleration breakdown).
     * @param {number} index
     * @param {Float64Array} sun_rel_pos
     * @param {number} sun_radius_m
     * @param {Float64Array} perturbers_flat
     * @returns {any}
     */
    get_satellite_telemetry(index, sun_rel_pos, sun_radius_m, perturbers_flat) {
        const ptr0 = passArrayF64ToWasm0(sun_rel_pos, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passArrayF64ToWasm0(perturbers_flat, wasm.__wbindgen_malloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.wasmcentricswarmengine_get_satellite_telemetry(this.__wbg_ptr, index, ptr0, len0, sun_radius_m, ptr1, len1);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * Returns flat array of normalized Vis-Viva kinetic parameters: tau in [0.0, 1.0].
     * @returns {Float64Array}
     */
    get_vis_viva_kinetic_flat() {
        const ret = wasm.wasmcentricswarmengine_get_vis_viva_kinetic_flat(this.__wbg_ptr);
        return ret;
    }
    /**
     * Generates Keplerian swarm orbits around the specified anchor body.
     * @param {string} anchor_name
     * @param {string} preset
     * @param {number} count
     */
    constructor(anchor_name, preset, count) {
        const ptr0 = passStringToWasm0(anchor_name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(preset, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.wasmcentricswarmengine_new(ptr0, len0, ptr1, len1, count);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        this.__wbg_ptr = ret[0];
        WasmCentricSwarmEngineFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * Returns the particle count in the swarm.
     * @returns {number}
     */
    particle_count() {
        const ret = wasm.wasmcentricswarmengine_particle_count(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Sets the active perturbation model:
     * 0 = TwoBody (pure Keplerian)
     * 1 = ThirdBody (central + lunisolar/planetary third-body tidal & reflex)
     * 2 = FullPerturbed (central + third-body + J2 oblateness)
     * @param {number} mode
     */
    set_perturbation_mode(mode) {
        wasm.wasmcentricswarmengine_set_perturbation_mode(this.__wbg_ptr, mode);
    }
    /**
     * Numerically integrates all particles in the non-inertial relative centric frame.
     *
     * Supports pure two-body, lunisolar third-body, and oblate J2 zonal gravitational perturbations.
     * @param {number} dt
     * @param {number} sub_steps
     * @param {Float64Array} perturbers_flat
     */
    step(dt, sub_steps, perturbers_flat) {
        const ptr0 = passArrayF64ToWasm0(perturbers_flat, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.wasmcentricswarmengine_step(this.__wbg_ptr, dt, sub_steps, ptr0, len0);
    }
}
if (Symbol.dispose) WasmCentricSwarmEngine.prototype[Symbol.dispose] = WasmCentricSwarmEngine.prototype.free;

/**
 * High-performance N-Body gravitational dynamics simulation engine.
 */
export class WasmNBodyEngine {
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WasmNBodyEngineFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmnbodyengine_free(ptr, 0);
    }
    /**
     * Returns bodies serialized to a JavaScript array of objects.
     * @returns {any}
     */
    get_bodies() {
        const ret = wasm.wasmnbodyengine_get_bodies(this.__wbg_ptr);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * Evaluates total mechanical energy, momentum, and barycenter conservation metrics.
     * @returns {any}
     */
    get_conservation_metrics() {
        const ret = wasm.wasmnbodyengine_get_conservation_metrics(this.__wbg_ptr);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * Evaluates pairwise gravitational forces on the body at `focus_index`.
     * @param {number} focus_index
     * @returns {any}
     */
    get_pairwise_forces(focus_index) {
        const ret = wasm.wasmnbodyengine_get_pairwise_forces(this.__wbg_ptr, focus_index);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * Returns flat array of body positions: [x0, y0, z0, x1, y1, z1, ...].
     * @returns {Float64Array}
     */
    get_positions_flat() {
        const ret = wasm.wasmnbodyengine_get_positions_flat(this.__wbg_ptr);
        return ret;
    }
    /**
     * Returns the preset identifier string.
     * @returns {string}
     */
    get_preset_name() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmnbodyengine_get_preset_name(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Returns flat array of body velocities: [vx0, vy0, vz0, vx1, vy1, vz1, ...].
     * @returns {Float64Array}
     */
    get_velocities_flat() {
        const ret = wasm.wasmnbodyengine_get_velocities_flat(this.__wbg_ptr);
        return ret;
    }
    /**
     * Initializes an N-Body system from a canonical astronomical preset.
     * @param {string} preset_id
     */
    constructor(preset_id) {
        const ptr0 = passStringToWasm0(preset_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmnbodyengine_new(ptr0, len0);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        this.__wbg_ptr = ret[0];
        WasmNBodyEngineFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * Advances the N-body system by time `dt` seconds across `sub_steps` integration increments.
     * @param {number} dt
     * @param {number} sub_steps
     * @param {string} integrator
     * @param {boolean} enable_gr
     */
    step(dt, sub_steps, integrator, enable_gr) {
        const ptr0 = passStringToWasm0(integrator, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.wasmnbodyengine_step(this.__wbg_ptr, dt, sub_steps, ptr0, len0, enable_gr);
    }
}
if (Symbol.dispose) WasmNBodyEngine.prototype[Symbol.dispose] = WasmNBodyEngine.prototype.free;

/**
 * Evaluates shadow cone geometry in pure WebAssembly.
 * @param {number} body_x
 * @param {number} body_y
 * @param {number} body_z
 * @param {number} body_radius_m
 * @param {number} sun_x
 * @param {number} sun_y
 * @param {number} sun_z
 * @param {number} sun_radius_m
 * @returns {any}
 */
export function wasm_compute_shadow_cone(body_x, body_y, body_z, body_radius_m, sun_x, sun_y, sun_z, sun_radius_m) {
    const ret = wasm.wasm_compute_shadow_cone(body_x, body_y, body_z, body_radius_m, sun_x, sun_y, sun_z, sun_radius_m);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Computes spatial gravitational field point and tidal tensor in pure WebAssembly.
 * @param {WasmNBodyEngine} engine
 * @param {number} px
 * @param {number} py
 * @param {number} pz
 * @returns {any}
 */
export function wasm_compute_spatial_field_point(engine, px, py, pz) {
    _assertClass(engine, WasmNBodyEngine);
    const ret = wasm.wasm_compute_spatial_field_point(engine.__wbg_ptr, px, py, pz);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}
function __wbg_get_imports() {
    const import0 = {
        __proto__: null,
        __wbg_Error_30c8987f7c2ed4e2: function(arg0, arg1) {
            const ret = Error(getStringFromWasm0(arg0, arg1));
            return ret;
        },
        __wbg_String_8564e559799eccda: function(arg0, arg1) {
            const ret = String(arg1);
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg___wbindgen_throw_41e9ee4f547fc59a: function(arg0, arg1) {
            throw new Error(getStringFromWasm0(arg0, arg1));
        },
        __wbg_new_617a8cdb8bb1130e: function() {
            const ret = new Object();
            return ret;
        },
        __wbg_new_c0cfdc72bf7dee4d: function(arg0) {
            const ret = new Float64Array(arg0);
            return ret;
        },
        __wbg_new_ee2291f50781bf1d: function() {
            const ret = new Array();
            return ret;
        },
        __wbg_new_from_slice_911f717853b222c7: function(arg0, arg1) {
            const ret = new Int32Array(getArrayI32FromWasm0(arg0, arg1));
            return ret;
        },
        __wbg_new_from_slice_f545fd22ddc142b8: function(arg0, arg1) {
            const ret = new Float64Array(getArrayF64FromWasm0(arg0, arg1));
            return ret;
        },
        __wbg_set_6be42768c690e380: function(arg0, arg1, arg2) {
            arg0[arg1] = arg2;
        },
        __wbg_set_bea140a88be9b277: function(arg0, arg1, arg2) {
            arg0[arg1 >>> 0] = arg2;
        },
        __wbindgen_generic_0000000000000001: function(arg0) {
            // Cast intrinsic for `F64 -> Externref`.
            const ret = arg0;
            return ret;
        },
        __wbindgen_generic_0000000000000002: function(arg0, arg1) {
            // Cast intrinsic for `Ref(String) -> Externref`.
            const ret = getStringFromWasm0(arg0, arg1);
            return ret;
        },
        __wbindgen_generic_0000000000000003: function(arg0) {
            // Cast intrinsic for `U64 -> Externref`.
            const ret = BigInt.asUintN(64, arg0);
            return ret;
        },
        __wbindgen_init_externref_table: function() {
            const table = wasm.__wbindgen_externrefs;
            const offset = table.grow(4);
            table.set(0, undefined);
            table.set(offset + 0, undefined);
            table.set(offset + 1, null);
            table.set(offset + 2, true);
            table.set(offset + 3, false);
        },
    };
    return {
        __proto__: null,
        "./sbm_wasm_bg.js": import0,
    };
}

const WasmCentricSwarmEngineFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmcentricswarmengine_free(ptr, 1));
const WasmNBodyEngineFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmnbodyengine_free(ptr, 1));

function _assertClass(instance, klass) {
    if (!(instance instanceof klass)) {
        throw new Error(`expected instance of ${klass.name}`);
    }
}

function getArrayF64FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getFloat64ArrayMemory0().subarray(ptr / 8, ptr / 8 + len);
}

function getArrayI32FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getInt32ArrayMemory0().subarray(ptr / 4, ptr / 4 + len);
}

let cachedDataViewMemory0 = null;
function getDataViewMemory0() {
    if (cachedDataViewMemory0 === null || cachedDataViewMemory0.buffer.detached === true || (cachedDataViewMemory0.buffer.detached === undefined && cachedDataViewMemory0.buffer !== wasm.memory.buffer)) {
        cachedDataViewMemory0 = new DataView(wasm.memory.buffer);
    }
    return cachedDataViewMemory0;
}

let cachedFloat64ArrayMemory0 = null;
function getFloat64ArrayMemory0() {
    if (cachedFloat64ArrayMemory0 === null || cachedFloat64ArrayMemory0.byteLength === 0) {
        cachedFloat64ArrayMemory0 = new Float64Array(wasm.memory.buffer);
    }
    return cachedFloat64ArrayMemory0;
}

let cachedInt32ArrayMemory0 = null;
function getInt32ArrayMemory0() {
    if (cachedInt32ArrayMemory0 === null || cachedInt32ArrayMemory0.byteLength === 0) {
        cachedInt32ArrayMemory0 = new Int32Array(wasm.memory.buffer);
    }
    return cachedInt32ArrayMemory0;
}

function getStringFromWasm0(ptr, len) {
    return decodeText(ptr >>> 0, len);
}

let cachedUint8ArrayMemory0 = null;
function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

function passArrayF64ToWasm0(arg, malloc) {
    const ptr = malloc(arg.length * 8, 8) >>> 0;
    getFloat64ArrayMemory0().set(arg, ptr / 8);
    WASM_VECTOR_LEN = arg.length;
    return ptr;
}

function passStringToWasm0(arg, malloc, realloc) {
    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length, 1) >>> 0;
        getUint8ArrayMemory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len, 1) >>> 0;

    const mem = getUint8ArrayMemory0();

    let offset = 0;

    for (; offset < len; offset++) {
        const code = arg.charCodeAt(offset);
        if (code > 0x7F) break;
        mem[ptr + offset] = code;
    }
    if (offset !== len) {
        if (offset !== 0) {
            arg = arg.slice(offset);
        }
        ptr = realloc(ptr, len, len = offset + arg.length * 3, 1) >>> 0;
        const view = getUint8ArrayMemory0().subarray(ptr + offset, ptr + len);
        const ret = cachedTextEncoder.encodeInto(arg, view);

        offset += ret.written;
        ptr = realloc(ptr, len, offset, 1) >>> 0;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

function takeFromExternrefTable0(idx) {
    const value = wasm.__wbindgen_externrefs.get(idx);
    wasm.__externref_table_dealloc(idx);
    return value;
}

let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
cachedTextDecoder.decode();
const MAX_SAFARI_DECODE_BYTES = 2146435072;
let numBytesDecoded = 0;
function decodeText(ptr, len) {
    numBytesDecoded += len;
    if (numBytesDecoded >= MAX_SAFARI_DECODE_BYTES) {
        cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
        cachedTextDecoder.decode();
        numBytesDecoded = len;
    }
    return cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr, ptr + len));
}

const cachedTextEncoder = new TextEncoder();

if (!('encodeInto' in cachedTextEncoder)) {
    cachedTextEncoder.encodeInto = function (arg, view) {
        const buf = cachedTextEncoder.encode(arg);
        view.set(buf);
        return {
            read: arg.length,
            written: buf.length
        };
    };
}

let WASM_VECTOR_LEN = 0;

let wasmModule, wasmInstance, wasm;
function __wbg_finalize_init(instance, module) {
    wasmInstance = instance;
    wasm = instance.exports;
    wasmModule = module;
    cachedDataViewMemory0 = null;
    cachedFloat64ArrayMemory0 = null;
    cachedInt32ArrayMemory0 = null;
    cachedUint8ArrayMemory0 = null;
    wasm.__wbindgen_start();
    return wasm;
}

async function __wbg_load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (!module.ok) {
            throw new Error(`failed to fetch Wasm: ${module.status} ${module.statusText} fetching '${module.url}'`);
        }

        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);
            } catch (e) {
                const validResponse = expectedResponseType(module.type);

                if (validResponse && module.headers.get('Content-Type') !== 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve Wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else { throw e; }
            }
        }

        const bytes = await module.arrayBuffer();
        return await WebAssembly.instantiate(bytes, imports);
    } else {
        const instance = await WebAssembly.instantiate(module, imports);

        if (instance instanceof WebAssembly.Instance) {
            return { instance, module };
        } else {
            return instance;
        }
    }

    function expectedResponseType(type) {
        switch (type) {
            case 'basic': case 'cors': case 'default': return true;
        }
        return false;
    }
}

function initSync(module) {
    if (wasm !== undefined) return wasm;


    if (module !== undefined) {
        if (Object.getPrototypeOf(module) === Object.prototype) {
            ({module} = module)
        } else {
            console.warn('using deprecated parameters for `initSync()`; pass a single object instead')
        }
    }

    const imports = __wbg_get_imports();
    if (!(module instanceof WebAssembly.Module)) {
        module = new WebAssembly.Module(module);
    }
    const instance = new WebAssembly.Instance(module, imports);
    return __wbg_finalize_init(instance, module);
}

async function __wbg_init(module_or_path) {
    if (wasm !== undefined) return wasm;


    if (module_or_path !== undefined) {
        if (Object.getPrototypeOf(module_or_path) === Object.prototype) {
            ({module_or_path} = module_or_path)
        } else {
            console.warn('using deprecated parameters for the initialization function; pass a single object instead')
        }
    }

    if (module_or_path === undefined) {
        module_or_path = new URL('sbm_wasm_bg.wasm', import.meta.url);
    }
    const imports = __wbg_get_imports();

    if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {
        module_or_path = fetch(module_or_path);
    }

    const { instance, module } = await __wbg_load(await module_or_path, imports);

    return __wbg_finalize_init(instance, module);
}

export { initSync, __wbg_init as default };
