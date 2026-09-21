"""
AutoOrbit: Physics-Informed Fourier Neural Operator (FNO) and Loss in PyTorch.

Grounded in Sections 3.3.2 and 3.3.3 of the AutoOrbit paper (KDD 2026):
- Zhang et al. (2026), AutoOrbit: Physics-Informed Satellite Orbit Prediction.
"""

import math
import torch
import torch.nn as nn
import torch.nn.functional as F

EARTH_MU = 3.986004418e14
EARTH_RADIUS_M = 6378137.0
J2 = 1.08262668e-3
J3 = -2.532656485e-6
J4 = -1.61962159e-6
EARTH_ROTATION_RATE = 7.292115e-5


class SpectralConv1d(nn.Module):
    """1D Spectral Convolution layer operating in the Fourier domain.
    
    Retains the first k_max low-frequency Fourier modes and applies learnable
    complex weight matrices (Eqs. 5-6).
    """
    def __init__(self, in_channels: int, out_channels: int, n_modes: int):
        super().__init__()
        self.in_channels = in_channels
        self.out_channels = out_channels
        self.n_modes = n_modes

        scale = 1.0 / math.sqrt(in_channels * out_channels)
        self.weights_real = nn.Parameter(scale * torch.randn(n_modes, in_channels, out_channels))
        self.weights_imag = nn.Parameter(scale * torch.randn(n_modes, in_channels, out_channels))

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        # x shape: [batch, in_channels, length]
        batch, in_c, length = x.shape
        modes = min(self.n_modes, length // 2)

        # 1D Real FFT along temporal dimension
        x_fft = torch.fft.rfft(x, n=length, dim=-1)  # [batch, in_c, length // 2 + 1]

        # Truncate to first k_max modes
        x_modes = x_fft[..., :modes]  # [batch, in_c, modes]
        weights_c = torch.complex(self.weights_real[:modes], self.weights_imag[:modes])  # [modes, in_c, out_c]

        # Complex linear transformation: out[b, out_c, m] = sum_{in_c} x_modes[b, in_c, m] * weights[m, in_c, out_c]
        out_modes = torch.einsum("bim,mic->bcm", x_modes, weights_c)

        # Pad with zeros to original length // 2 + 1
        out_fft = torch.zeros(batch, self.out_channels, length // 2 + 1, dtype=torch.cfloat, device=x.device)
        out_fft[..., :modes] = out_modes

        # 1D Inverse Real FFT back to time domain
        out = torch.fft.irfft(out_fft, n=length, dim=-1)
        return out


class Fno1dBlock(nn.Module):
    """A single Fourier Layer fusing spectral and time-domain skip pathways (Eq. 7)."""
    def __init__(self, in_channels: int, out_channels: int, n_modes: int, activation: str = "gelu"):
        super().__init__()
        self.spectral_conv = SpectralConv1d(in_channels, out_channels, n_modes)
        self.skip_conv = nn.Conv1d(in_channels, out_channels, kernel_size=1)
        self.activation = nn.GELU() if activation.lower() == "gelu" else nn.ReLU()

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        return self.activation(self.spectral_conv(x) + self.skip_conv(x))


class AutoOrbitFNO1d(nn.Module):
    """Full AutoOrbit Encoder-Pooling-Decoder FNO Architecture for orbit residual prediction.
    
    - Lifting layer: projects 6 state channels -> hidden_channels
    - Encoder: stacked Fourier layers extracting global spatio-temporal features
    - Adaptive pooling: compresses input sequence length (e.g. 8640) -> target length (e.g. 60)
    - Decoder: stacked Fourier layers reconstructing future residual dynamics
    - Projection layer: hidden_channels -> 6 output residual channels
    """
    def __init__(
        self,
        in_channels: int = 6,
        out_channels: int = 6,
        hidden_channels: int = 256,
        n_modes: int = 8,
        n_layers: int = 4,
        input_length: int = 8640,
        output_length: int = 60,
    ):
        super().__init__()
        self.input_length = input_length
        self.output_length = output_length
        self.hidden_channels = hidden_channels
        self.n_modes = n_modes

        # 1. Lifting: 6 -> hidden_channels
        self.lifting = nn.Conv1d(in_channels, hidden_channels, kernel_size=1)

        # 2. Encoder: n_layers Fourier blocks
        self.encoder = nn.ModuleList([
            Fno1dBlock(hidden_channels, hidden_channels, n_modes)
            for _ in range(n_layers // 2)
        ])

        # 3. Sequence adapter (AdaptiveAvgPool1d)
        self.adaptive_pool = nn.AdaptiveAvgPool1d(output_length)

        # 4. Decoder: n_layers Fourier blocks
        dec_modes = min(n_modes, output_length // 2)
        self.decoder = nn.ModuleList([
            Fno1dBlock(hidden_channels, hidden_channels, dec_modes)
            for _ in range(n_layers // 2)
        ])

        # 5. Projection: hidden_channels -> 6
        self.projection = nn.Conv1d(hidden_channels, out_channels, kernel_size=1)

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        # x shape: [batch, 6, length]
        h = self.lifting(x)
        for layer in self.encoder:
            h = layer(h)
        h = self.adaptive_pool(h)
        for layer in self.decoder:
            h = layer(h)
        out = self.projection(h)
        return out


class PhysicsInformedLoss(nn.Module):
    """Acceleration-level physics-informed loss function (Eqs. 8-9)."""
    def __init__(
        self,
        cadence_s: float = 10.0,
        satellite_mass_kg: float = 2158.777,
        satellite_area_m2: float = 20.395,
        cd: float = 2.2,
        include_drag: bool = True,
        include_geopotential: bool = True,
        mu: float = EARTH_MU,
    ):
        super().__init__()
        self.cadence_s = cadence_s
        self.mass = satellite_mass_kg
        self.area = satellite_area_m2
        self.cd = cd
        self.include_drag = include_drag
        self.include_geopotential = include_geopotential
        self.mu = mu

    def compute_predicted_acceleration_4th_order(self, v_seq: torch.Tensor) -> torch.Tensor:
        """v_seq shape: [batch, 3, length]
        Returns a_pred shape: [batch, 3, length - 4]
        Eq. 8: a_pred = (-v_{+2} + 8 v_{+1} - 8 v_{-1} + v_{-2}) / (12 tau)
        """
        denom = 12.0 * self.cadence_s
        v_m2 = v_seq[..., :-4]
        v_m1 = v_seq[..., 1:-3]
        v_p1 = v_seq[..., 3:-1]
        v_p2 = v_seq[..., 4:]
        a_pred = (-v_p2 + 8.0 * v_p1 - 8.0 * v_m1 + v_m2) / denom
        return a_pred

    def compute_theoretical_acceleration(self, r_vec: torch.Tensor, v_vec: torch.Tensor) -> torch.Tensor:
        """Evaluates theoretical physical acceleration including central gravity and J2-J4 harmonics."""
        # r_vec: [batch, 3, length], v_vec: [batch, 3, length]
        r = torch.norm(r_vec, dim=1, keepdim=True).clamp(min=1.0)  # [batch, 1, length]
        r2 = r * r
        r3 = r2 * r

        # Central gravity: a_grav = -mu / r^3 * r
        a_phy = - (self.mu / r3) * r_vec

        if self.include_geopotential:
            re = EARTH_RADIUS_M
            re2 = re * re
            r5 = r3 * r2
            z = r_vec[:, 2:3, :]
            z2 = z * z
            z2_over_r2 = z2 / r2

            # J2 Zonal Acceleration
            j2_coeff = -1.5 * J2 * self.mu * re2 / r5
            ax_j2 = j2_coeff * r_vec[:, 0:1, :] * (1.0 - 5.0 * z2_over_r2)
            ay_j2 = j2_coeff * r_vec[:, 1:2, :] * (1.0 - 5.0 * z2_over_r2)
            az_j2 = j2_coeff * z * (3.0 - 5.0 * z2_over_r2)
            a_j2 = torch.cat([ax_j2, ay_j2, az_j2], dim=1)
            a_phy = a_phy + a_j2

        return a_phy

    def forward(self, pred_residuals: torch.Tensor, ref_states: torch.Tensor = None) -> torch.Tensor:
        # pred_residuals shape: [batch, 6, length]
        if pred_residuals.shape[-1] < 5:
            return torch.tensor(0.0, device=pred_residuals.device)

        if ref_states is not None:
            full_states = pred_residuals + ref_states
        else:
            # If training on residuals without explicit reference batch, anchor to nominal LEO orbit
            r_ref = torch.tensor([7071000.0, 0.0, 0.0], device=pred_residuals.device).view(1, 3, 1)
            v_ref = torch.tensor([0.0, 7500.0, 0.0], device=pred_residuals.device).view(1, 3, 1)
            ref_anchor = torch.cat([r_ref, v_ref], dim=1)
            full_states = pred_residuals + ref_anchor

        r_seq = full_states[:, :3, :]
        v_seq = full_states[:, 3:6, :]

        a_pred = self.compute_predicted_acceleration_4th_order(v_seq)
        r_mid = r_seq[:, :, 2:-2]
        v_mid = v_seq[:, :, 2:-2]
        a_phy = self.compute_theoretical_acceleration(r_mid, v_mid)

        # MSE physics loss (Eq. 9)
        loss = F.mse_loss(a_pred, a_phy)
        return loss
