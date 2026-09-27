/**
 * Gravitational Centric View Frame Configurations & Filtering.
 */
import { AU_M } from './constants.js';

export function getCentricFrameConfig(bodyName) {
  if (bodyName === 'Earth') {
    return {
      bodyName: 'Earth',
      frameName: 'Geocentric',
      desc: 'Earth Dominance Basin (Solar gravity ignored · Cislunar Earth-Moon gravity well)',
      spanScene: 3.6,
      halfSpanPhys_m: 1_200_000e3, // 1.2M km covers Chebotarev (259k km), Moon orbit (384k km), Hill (1.5M km)
      gridSteps: 32, // 32x32 = 1,024 vectors (fine micro-granularity: ~77,419 km step)
      baseArrowLen: 0.13,
      arrowColor: 0x38bdf8,
      sensitivity: 50.0,
      contourRadiiScene: [0.25, 0.5, 0.9, 1.4, 1.9, 2.7]
    };
  } else if (bodyName === 'Moon') {
    return {
      bodyName: 'Moon',
      frameName: 'Selenocentric',
      desc: 'Moon Dominance Basin (Solar gravity ignored · Lunar gravity well)',
      spanScene: 1.4,
      halfSpanPhys_m: 100_000e3, // 100,000 km covers lunar surface, neutral boundary, and L1
      gridSteps: 32,
      baseArrowLen: 0.065,
      arrowColor: 0xffffff,
      sensitivity: 100.0,
      contourRadiiScene: [0.15, 0.3, 0.55, 0.8, 1.1]
    };
  } else if (bodyName === 'Jupiter') {
    return {
      bodyName: 'Jupiter',
      frameName: 'Jovicentric',
      desc: 'Jovian Dominance Basin (Solar gravity ignored · Jovian system gravity well)',
      spanScene: 7.5,
      halfSpanPhys_m: 32_000_000e3, // 32M km
      gridSteps: 30,
      baseArrowLen: 0.28,
      arrowColor: 0xfb923c,
      sensitivity: 20.0,
      contourRadiiScene: [0.8, 1.8, 3.2, 4.8, 6.2]
    };
  } else if (bodyName === 'Venus') {
    return {
      bodyName: 'Venus',
      frameName: 'Cytherocentric',
      desc: 'Venus Dominance Basin (Solar gravity ignored · Venusian gravity well)',
      spanScene: 2.8,
      halfSpanPhys_m: 600_000e3,
      gridSteps: 28,
      baseArrowLen: 0.12,
      arrowColor: 0xfef08a,
      sensitivity: 50.0,
      contourRadiiScene: [0.3, 0.7, 1.2, 1.8, 2.4]
    };
  } else if (bodyName === 'Mars') {
    return {
      bodyName: 'Mars',
      frameName: 'Areocentric',
      desc: 'Mars Dominance Basin (Solar gravity ignored · Martian gravity well)',
      spanScene: 2.5,
      halfSpanPhys_m: 500_000e3,
      gridSteps: 28,
      baseArrowLen: 0.12,
      arrowColor: 0xf87171,
      sensitivity: 50.0,
      contourRadiiScene: [0.25, 0.6, 1.1, 1.6, 2.2]
    };
  } else {
    // Sun / Heliocentric (Default)
    return {
      bodyName: 'Sun',
      frameName: 'Heliocentric',
      desc: 'Interplanetary Solar System (Sun Dominant Gravitational Expanse)',
      spanScene: 32.0,
      halfSpanPhys_m: 5.5 * AU_M,
      gridSteps: 22,
      baseArrowLen: 0.80,
      arrowColor: 0xfbbf24,
      sensitivity: 500.0,
      contourRadiiScene: [4.0, 8.0, 14.0, 20.0, 26.0, 32.0]
    };
  }
}

export function isBodyRelevantToCentricMode(bodyName, centricBodyName) {
  if (!centricBodyName || centricBodyName === 'Sun') {
    return true; // Heliocentric mode includes all celestial bodies
  }
  if (centricBodyName === 'Earth') {
    return bodyName === 'Earth' || bodyName === 'Moon';
  }
  if (centricBodyName === 'Moon') {
    return bodyName === 'Moon' || bodyName === 'Earth';
  }
  if (centricBodyName === 'Jupiter') {
    return bodyName === 'Jupiter' || bodyName === 'Io' || bodyName === 'Europa' || bodyName === 'Ganymede' || bodyName === 'Callisto';
  }
  if (centricBodyName === 'Venus') {
    return bodyName === 'Venus';
  }
  if (centricBodyName === 'Mars') {
    return bodyName === 'Mars';
  }
  return bodyName === centricBodyName;
}

let toastTimeout = null;
export function showCentricToast(cfg) {
  const toast = document.getElementById('gravitational-centric-toast');
  const title = document.getElementById('centric-toast-title');
  const desc = document.getElementById('centric-toast-desc');
  if (!toast) return;

  if (title) title.innerText = `CENTRIC VIEW: ${cfg.frameName.toUpperCase()} (${cfg.bodyName.toUpperCase()})`;
  if (desc) desc.innerText = cfg.desc;

  toast.style.borderColor = `#${cfg.arrowColor.toString(16).padStart(6, '0')}`;
  toast.classList.remove('hidden');

  if (toastTimeout) clearTimeout(toastTimeout);
  toastTimeout = setTimeout(() => {
    toast.classList.add('hidden');
  }, 4500);
}
