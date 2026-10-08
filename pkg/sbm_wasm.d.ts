/* tslint:disable */
/* eslint-disable */

/**
 * Swarm particle propagation engine running in pure WebAssembly.
 */
export class WasmCentricSwarmEngine {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Evaluates eclipse state (0 = Sunlit, 1 = Penumbra, 2 = Umbra) for all particles.
     */
    get_eclipse_states_flat(sun_rel_pos: Float64Array, sun_radius_m: number): Int32Array;
    /**
     * Returns flat array of the closed Keplerian ellipse orbit vertices for particle `index`.
     */
    get_orbit_trail_flat(index: number): Float64Array;
    /**
     * Gets the active perturbation mode.
     */
    get_perturbation_mode(): number;
    /**
     * Returns flat array of particle relative positions: [x0, y0, z0, x1, y1, z1, ...].
     */
    get_positions_flat(): Float64Array;
    /**
     * Computes full satellite telemetry (Kepler elements, altitude, speed, period, eclipse, acceleration breakdown).
     */
    get_satellite_telemetry(index: number, sun_rel_pos: Float64Array, sun_radius_m: number, perturbers_flat: Float64Array): any;
    /**
     * Returns flat array of normalized Vis-Viva kinetic parameters: tau in [0.0, 1.0].
     */
    get_vis_viva_kinetic_flat(): Float64Array;
    /**
     * Generates Keplerian swarm orbits around the specified anchor body.
     */
    constructor(anchor_name: string, preset: string, count: number);
    /**
     * Returns the particle count in the swarm.
     */
    particle_count(): number;
    /**
     * Sets the active perturbation model:
     * 0 = TwoBody (pure Keplerian)
     * 1 = ThirdBody (central + lunisolar/planetary third-body tidal & reflex)
     * 2 = FullPerturbed (central + third-body + J2 oblateness)
     */
    set_perturbation_mode(mode: number): void;
    /**
     * Numerically integrates all particles in the non-inertial relative centric frame.
     *
     * Supports pure two-body, lunisolar third-body, and oblate J2 zonal gravitational perturbations.
     */
    step(dt: number, sub_steps: number, perturbers_flat: Float64Array): void;
}

/**
 * High-performance N-Body gravitational dynamics simulation engine.
 */
export class WasmNBodyEngine {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Returns bodies serialized to a JavaScript array of objects.
     */
    get_bodies(): any;
    /**
     * Evaluates total mechanical energy, momentum, and barycenter conservation metrics.
     */
    get_conservation_metrics(): any;
    /**
     * Evaluates osculating Keplerian orbital elements of body `body_index` relative to `primary_index`.
     */
    get_osculating_elements(body_index: number, primary_index: number): any;
    /**
     * Evaluates pairwise gravitational forces on the body at `focus_index`.
     */
    get_pairwise_forces(focus_index: number): any;
    /**
     * Returns flat array of body positions: [x0, y0, z0, x1, y1, z1, ...].
     */
    get_positions_flat(): Float64Array;
    /**
     * Returns the preset identifier string.
     */
    get_preset_name(): string;
    /**
     * Returns flat array of body velocities: [vx0, vy0, vz0, vx1, vy1, vz1, ...].
     */
    get_velocities_flat(): Float64Array;
    /**
     * Initializes an N-Body system from a canonical astronomical preset.
     */
    constructor(preset_id: string);
    /**
     * Advances the N-body system by time `dt` seconds across `sub_steps` integration increments.
     */
    step(dt: number, sub_steps: number, integrator: string, enable_gr: boolean): void;
}

/**
 * Evaluates shadow cone geometry in pure WebAssembly.
 */
export function wasm_compute_shadow_cone(body_x: number, body_y: number, body_z: number, body_radius_m: number, sun_x: number, sun_y: number, sun_z: number, sun_radius_m: number): any;

/**
 * Computes spatial gravitational field point and tidal tensor in pure WebAssembly.
 */
export function wasm_compute_spatial_field_point(engine: WasmNBodyEngine, px: number, py: number, pz: number): any;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_wasmcentricswarmengine_free: (a: number, b: number) => void;
    readonly __wbg_wasmnbodyengine_free: (a: number, b: number) => void;
    readonly wasm_compute_shadow_cone: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => [number, number, number];
    readonly wasm_compute_spatial_field_point: (a: number, b: number, c: number, d: number) => [number, number, number];
    readonly wasmcentricswarmengine_get_eclipse_states_flat: (a: number, b: number, c: number, d: number) => any;
    readonly wasmcentricswarmengine_get_orbit_trail_flat: (a: number, b: number) => any;
    readonly wasmcentricswarmengine_get_perturbation_mode: (a: number) => number;
    readonly wasmcentricswarmengine_get_positions_flat: (a: number) => any;
    readonly wasmcentricswarmengine_get_satellite_telemetry: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => [number, number, number];
    readonly wasmcentricswarmengine_get_vis_viva_kinetic_flat: (a: number) => any;
    readonly wasmcentricswarmengine_new: (a: number, b: number, c: number, d: number, e: number) => [number, number, number];
    readonly wasmcentricswarmengine_particle_count: (a: number) => number;
    readonly wasmcentricswarmengine_set_perturbation_mode: (a: number, b: number) => void;
    readonly wasmcentricswarmengine_step: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly wasmnbodyengine_get_bodies: (a: number) => [number, number, number];
    readonly wasmnbodyengine_get_conservation_metrics: (a: number) => [number, number, number];
    readonly wasmnbodyengine_get_osculating_elements: (a: number, b: number, c: number) => [number, number, number];
    readonly wasmnbodyengine_get_pairwise_forces: (a: number, b: number) => [number, number, number];
    readonly wasmnbodyengine_get_positions_flat: (a: number) => any;
    readonly wasmnbodyengine_get_preset_name: (a: number) => [number, number];
    readonly wasmnbodyengine_get_velocities_flat: (a: number) => any;
    readonly wasmnbodyengine_new: (a: number, b: number) => [number, number, number];
    readonly wasmnbodyengine_step: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
