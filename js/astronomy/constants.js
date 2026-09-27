/**
 * Mathematical, Physical, and Visual Astronomical Constants.
 */

// Universal Physical Constants
export const G_STANDARD = 6.67430e-11;
export const SPEED_OF_LIGHT = 299792458.0;
export const AU_M = 149597870700.0;
export const DAY_S = 86400.0;
export const YEAR_S = 365.25 * DAY_S;

// High-Precision Integrator Weights (Yoshida 4th & 6th order)
export const Y4_W1 = 1.3512071919596578;
export const Y4_W0 = -1.7024143839193153;
export const Y4_C1 = Y4_W1 / 2.0;
export const Y4_C2 = (Y4_W0 + Y4_W1) / 2.0;
export const Y4_C = [Y4_C1, Y4_C2, Y4_C2, Y4_C1];
export const Y4_D = [Y4_W1, Y4_W0, Y4_W1, 0.0];

export const Y6_WEIGHTS = [
  0.784513610477560,
  0.235573213359357,
  -1.17767998417887,
  1.315186320683906,
  -1.17767998417887,
  0.235573213359357,
  0.784513610477560
];

// Perceptual Planetary Visual Radii (scene units)
export const PERCEPTUAL_PLANET_RADII = {
  Sun: 1.4,
  Mercury: 0.16,
  Venus: 0.26,
  Earth: 0.28,
  Moon: 0.09,
  Mars: 0.20,
  Jupiter: 0.85,
  Saturn: 0.70,
  Uranus: 0.45,
  Neptune: 0.44,
  Pluto: 0.10,
  Io: 0.08,
  Europa: 0.07,
  Ganymede: 0.11,
  Callisto: 0.10,
  Default: 0.15
};

// Body Emojis
export const BODY_EMOJIS = {
  Sun: '☀️',
  Mercury: '🟤',
  Venus: '🟡',
  Earth: '🌍',
  Moon: '🌕',
  Mars: '🔴',
  Jupiter: '🪐',
  Saturn: '🪐',
  Uranus: '🔵',
  Neptune: '🔷',
  Pluto: '⚪',
  Io: '🟠',
  Europa: '⚪',
  Ganymede: '🟤',
  Callisto: '🌑'
};

export function getBodyEmoji(name) {
  return BODY_EMOJIS[name] || '🪐';
}
