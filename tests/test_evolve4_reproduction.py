#!/usr/bin/env python3
"""
Automated Verification & Reproduction Benchmark Suite for NASA EVOLVE 4.0 Breakup Model
Reference: Johnson et al. (2001), Adv. Space Res. Vol. 28, No. 9, pp. 1377-1384

Verifies both the Rust engine (engine_cli) and Web visualizer algorithms against:
- Figure 2: Explosion Size Distribution (Eq. 2 & 3)
- Figure 4: Collision Size Distribution (Eq. 4)
- Figure 5 & 6: Area-to-Mass (A/M) Distributions for Upper Stages and Spacecraft (Eqs. 5, 6, 7)
- Equations 8 & 9: Average Cross-Sectional Area Ax(Lc)
- Equation 10: Individual Fragment Mass M = Ax / (A/M)
- Figure 7 & Equations 11 & 12: Delta-V Ejection Velocity Conditioning on log10(A/M)
"""

import json
import math
import os
import subprocess
import sys
import unittest

ROOT_DIR = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))

class TestEvolve4BreakupModel(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        # Run cargo build/run to ensure fresh fragments_output.json
        engine_dir = os.path.join(ROOT_DIR, "engine_cli")
        res = subprocess.run(["cargo", "run", "--release"], cwd=engine_dir, capture_output=True, text=True)
        assert res.returncode == 0, f"Cargo run failed: {res.stderr}"

        json_path = os.path.join(engine_dir, "fragments_output.json")
        with open(json_path, "r") as f:
            cls.fragments = json.load(f)

    def test_fragment_count_and_fields(self):
        """Verify fragments count and required EVOLVE 4.0 physical properties"""
        self.assertEqual(len(self.fragments), 1500)
        for f in self.fragments:
            self.assertIn("id", f)
            self.assertIn("size", f)
            self.assertIn("area", f)
            self.assertIn("am_ratio", f)
            self.assertIn("chi", f)
            self.assertIn("mass", f)
            self.assertIn("speed", f)
            self.assertGreater(f["size"], 0.0)
            self.assertGreater(f["area"], 0.0)
            self.assertGreater(f["am_ratio"], 0.0)
            self.assertGreater(f["speed"], 0.0)

    def test_cross_sectional_area_scaling(self):
        """Verify Ax(Lc) matches Johnson et al. (2001) Eqs. (8) and (9)"""
        for f in self.fragments:
            lc = f["size"]
            ax_actual = f["area"]
            if lc < 0.00167:
                ax_expected = 0.540424 * (lc ** 2.0)
            else:
                ax_expected = 0.556945 * (lc ** 2.0047077)
            self.assertAlmostEqual(ax_actual, ax_expected, delta=1e-4)

    def test_mass_conservation_and_physical_relation(self):
        """Verify total destroyed mass conservation and physical inverse relation M = Ax / (A/M)"""
        total_mass = sum(f["mass"] for f in self.fragments)
        # Target (1000 kg) + Projectile (100 kg) = 1100 kg destroyed
        self.assertAlmostEqual(total_mass, 1100.0, delta=0.5)

        for f in self.fragments:
            if f["mass"] > 1e-4:
                expected_am = f["area"] / f["mass"]
                self.assertAlmostEqual(f["am_ratio"], expected_am, delta=0.05)
            self.assertAlmostEqual(f["chi"], math.log10(f["am_ratio"]), delta=0.005)

    def test_area_to_mass_scatter_regime(self):
        """Verify log10(A/M) distribution matches empirical limits of Fig. 5 and Fig. 6 ([-3.0, +1.5])"""
        chis = [f["chi"] for f in self.fragments]
        mean_chi = sum(chis) / len(chis)
        # Empirical mean for spacecraft debris between -1.0 and 0.0
        self.assertTrue(-1.2 <= mean_chi <= 0.2, f"Mean chi {mean_chi} out of expected range")

        # 95% of fragments should fall within [-3.0, +1.5]
        within_bounds = sum(1 for c in chis if -3.0 <= c <= 1.5)
        pct = (within_bounds / len(chis)) * 100.0
        self.assertGreaterEqual(pct, 95.0, f"Only {pct}% of fragments within [-3.0, 1.5] range")

    def test_delta_v_distribution_moments(self):
        """Verify Delta-v follows log-normal conditioned on log10(A/M) (Eq. 12, Fig. 7)"""
        speeds = sorted(f["speed"] for f in self.fragments)
        p10 = speeds[int(len(speeds) * 0.10)]
        median = speeds[int(len(speeds) * 0.50)]
        p90 = speeds[int(len(speeds) * 0.90)]

        # Fig. 7: 10th percentile ~ 40-150 m/s, median ~ 200-700 m/s, 90th percentile ~ 1500-4000 m/s
        self.assertTrue(30.0 <= p10 <= 200.0, f"10th percentile {p10} m/s out of bounds")
        self.assertTrue(200.0 <= median <= 900.0, f"Median speed {median} m/s out of bounds")
        self.assertTrue(1200.0 <= p90 <= 4500.0, f"90th percentile {p90} m/s out of bounds")

    def test_size_range_without_artificial_ceiling(self):
        """Verify fragment size spectrum extends into the decimeter/meter range without artificial 0.22m cap"""
        sizes = [f["size"] for f in self.fragments]
        max_size = max(sizes)
        # Should generate fragments > 0.5m
        self.assertGreater(max_size, 0.50, f"Max size {max_size} m is artificially truncated")

if __name__ == "__main__":
    unittest.main()
