/* tslint:disable */
/* eslint-disable */
/**
 * Request a viewport reset (called from JavaScript)
 */
export function reset_viewport(): void;
/**
 * Get current viewport offset X (called from JavaScript for URL updates)
 */
export function get_viewport_x(): number;
/**
 * Get current viewport offset Y (called from JavaScript for URL updates)
 */
export function get_viewport_y(): number;
/**
 * Get current cell size (called from JavaScript for URL updates)
 */
export function get_cell_size(): number;
/**
 * Set initial viewport state from URL parameters (called from JavaScript)
 */
export function set_initial_viewport(offset_x: number, offset_y: number, cell_size: number): void;
/**
 * Initialize the web application with default settings
 * This function is exported to JavaScript and can be called to start the app
 */
export function start(): Promise<void>;
/**
 * Start the application with specific parameters
 * Called from JavaScript with values from the UI form
 */
export function start_with_params(rule: number, width: number, height: number, cell_size: number, cache_tiles: number, tile_size: number, initial_state: string | null | undefined, zoom_min: number, zoom_max: number): Promise<void>;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly get_viewport_x: () => number;
  readonly get_viewport_y: () => number;
  readonly get_cell_size: () => number;
  readonly start: () => any;
  readonly start_with_params: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number) => any;
  readonly set_initial_viewport: (a: number, b: number, c: number) => void;
  readonly reset_viewport: () => void;
  readonly __externref_table_alloc: () => number;
  readonly __wbindgen_export_1: WebAssembly.Table;
  readonly __wbindgen_exn_store: (a: number) => void;
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
  readonly __wbindgen_free: (a: number, b: number, c: number) => void;
  readonly __wbindgen_export_6: WebAssembly.Table;
  readonly closure522_externref_shim: (a: number, b: number, c: any) => void;
  readonly wasm_bindgen__convert__closures_____invoke__h83a3ad27906d3133: (a: number, b: number) => void;
  readonly closure1173_externref_shim: (a: number, b: number, c: any) => void;
  readonly closure524_externref_shim: (a: number, b: number, c: any, d: any) => void;
  readonly closure3019_externref_shim: (a: number, b: number, c: any, d: any) => void;
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
