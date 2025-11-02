/**
 * Universal config that matches notch-config.json structure
 * 
 * Usage:
 * ```typescript
 * import { loadConfig, getConfig } from '$lib/config';
 * 
 * // Load config once at app startup
 * const config = await loadConfig();
 * const width = config.dimensions.expanded_width.value;
 * 
 * // Or get cached config (returns defaults if not loaded)
 * const config = getConfig();
 * ```
 * 
 * You can also get config from Rust backend:
 * ```typescript
 * import { invoke } from '@tauri-apps/api/core';
 * const config = await invoke('get_notch_config');
 * ```
 */
export interface NotchConfig {
	animation: {
		expand_duration: ConfigValue<number>;
		collapse_duration: ConfigValue<number>;
		expand_timing: ConfigValue<number[]>;
		collapse_timing: ConfigValue<number[]>;
	};
	dimensions: {
		corner_radius: ConfigValue<number>;
		collapsed_width: ConfigValue<number>;
		collapsed_height: ConfigValue<number>;
		expanded_width: ConfigValue<number>;
		expanded_height: ConfigValue<number>;
	};
	hover: {
		collapsed_zone_width: ConfigValue<number>;
		collapsed_zone_height: ConfigValue<number>;
		expanded_zone_width: ConfigValue<number>;
		expanded_zone_height: ConfigValue<number>;
		expand_delay_ms: ConfigValue<number>;
		collapse_delay_ms: ConfigValue<number>;
		poll_interval_ms: ConfigValue<number>;
	};
	window: {
		level_offset: ConfigValue<number>;
	};
}

export interface ConfigValue<T> {
	value: T;
	description: string;
}

let cachedConfig: NotchConfig | null = null;

/**
 * Load the notch config from JSON file
 * This reads the same config file used by Rust and Swift
 */
export async function loadConfig(): Promise<NotchConfig> {
	if (cachedConfig) {
		return cachedConfig;
	}

	try {
		// In production, the config will be bundled
		// In dev, fetch from root
		const response = await fetch('/notch-config.json');
		if (!response.ok) {
			throw new Error(`Failed to load config: ${response.statusText}`);
		}
		cachedConfig = await response.json();
		return cachedConfig!;
	} catch (error) {
		console.error('Failed to load config, using defaults:', error);
		return getDefaultConfig();
	}
}

/**
 * Get cached config or default if not loaded
 */
export function getConfig(): NotchConfig {
	return cachedConfig || getDefaultConfig();
}

/**
 * Default config matching the JSON file defaults
 */
function getDefaultConfig(): NotchConfig {
	return {
		animation: {
			expand_duration: {
				value: 0.5,
				description: 'Duration in seconds for the expand animation - slower gives more fluid spring feel'
			},
			collapse_duration: {
				value: 0.35,
				description: 'Duration in seconds for the collapse animation - smooth and gentle'
			},
			expand_timing: {
				value: [0.16, 1.0, 0.3, 1.0],
				description: 'Cubic bezier control points for expand animation (fluid spring curve)'
			},
			collapse_timing: {
				value: [0.25, 0.1, 0.25, 1.0],
				description: 'Cubic bezier control points for collapse animation (smooth ease out)'
			}
		},
		dimensions: {
			corner_radius: {
				value: 12,
				description: 'Corner radius in points for the notch capsule rounded corners'
			},
			collapsed_width: {
				value: 460.0,
				description: 'Width in points when the notch is collapsed (matches notch size)'
			},
			collapsed_height: {
				value: 50.0,
				description: 'Height in points when the notch is collapsed'
			},
			expanded_width: {
				value: 700.0,
				description: 'Width in points when the notch is fully expanded'
			},
			expanded_height: {
				value: 200.0,
				description: 'Height in points when the notch is fully expanded'
			}
		},
		hover: {
			collapsed_zone_width: {
				value: 460.0,
				description: 'Width in points of the hover detection zone when collapsed'
			},
			collapsed_zone_height: {
				value: 50.0,
				description: 'Height in points of the hover detection zone when collapsed'
			},
			expanded_zone_width: {
				value: 700.0,
				description: 'Width in points of the hover detection zone when expanded'
			},
			expanded_zone_height: {
				value: 200.0,
				description: 'Height in points of the hover detection zone when expanded'
			},
			expand_delay_ms: {
				value: 250,
				description: 'Milliseconds to wait before expanding when hovering over the notch area'
			},
			collapse_delay_ms: {
				value: 150,
				description: 'Milliseconds to wait before collapsing when leaving the hover area'
			},
			poll_interval_ms: {
				value: 50,
				description: 'How often in milliseconds to check mouse position (polling fallback)'
			}
		},
		window: {
			level_offset: {
				value: 3,
				description: 'How many levels above main menu to place the window (higher = more on top)'
			}
		}
	};
}
