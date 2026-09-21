#!/usr/bin/env python3
"""
AutoOrbit Model Training Script in PyTorch.

Trains the AutoOrbit Fourier Neural Operator (FNO1d) with joint data loss
and acceleration-level physics loss (Eqs. 8-9).

Usage:
    python scripts/autoorbit/train_autoorbit.py --epochs 10 --lr 1e-3 --hidden 32
"""

import argparse
import math
import os
import torch
import torch.nn as nn
import torch.optim as optim
from fno_pytorch import AutoOrbitFNO1d, PhysicsInformedLoss
from export_weights import export_model_to_json


def generate_synthetic_orbit_dataset(num_samples: int = 500, seq_len: int = 128, cadence_s: float = 10.0):
    """Generates synthetic LEO orbit residual sequences for training validation."""
    torch.manual_seed(42)
    # LEO orbital frequency ~ 0.001 rad/s (period ~ 98 min)
    omega = 0.00107
    t = torch.arange(seq_len) * cadence_s

    # Dominant orbital harmonic residuals + stochastic perturbations
    residuals = []
    for _ in range(num_samples):
        phase = torch.rand(1) * 2 * math.pi
        amp_pos = 50.0 + 20.0 * torch.rand(3, 1)
        amp_vel = 0.05 + 0.02 * torch.rand(3, 1)

        r_res = amp_pos * torch.sin(omega * t + phase) + 2.0 * torch.randn(3, seq_len)
        v_res = amp_vel * torch.cos(omega * t + phase) + 0.005 * torch.randn(3, seq_len)

        sample = torch.cat([r_res, v_res], dim=0)  # [6, seq_len]
        residuals.append(sample)

    return torch.stack(residuals, dim=0)  # [num_samples, 6, seq_len]


def train(args):
    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    print(f"Training AutoOrbit FNO on device: {device}")

    # Build model
    model = AutoOrbitFNO1d(
        in_channels=6,
        out_channels=6,
        hidden_channels=args.hidden,
        n_modes=args.modes,
        n_layers=args.layers,
        input_length=args.input_len,
        output_length=args.output_len,
    ).to(device)

    # Losses & Optimizer
    criterion_data = nn.MSELoss()
    criterion_physics = PhysicsInformedLoss(cadence_s=args.cadence)
    optimizer = optim.Adam(model.parameters(), lr=args.lr)
    scheduler = optim.lr_scheduler.ExponentialLR(optimizer, gamma=0.97)

    # Data
    dataset = generate_synthetic_orbit_dataset(
        num_samples=args.samples,
        seq_len=args.input_len + args.output_len,
        cadence_s=args.cadence
    )
    dataloader = torch.utils.data.DataLoader(dataset, batch_size=args.batch_size, shuffle=True)

    print(f"Starting training for {args.epochs} epochs...")
    for epoch in range(1, args.epochs + 1):
        model.train()
        total_loss = 0.0
        total_data_loss = 0.0
        total_phy_loss = 0.0

        for batch in dataloader:
            batch = batch.to(device)
            x_in = batch[:, :, :args.input_len]
            y_target = batch[:, :, args.input_len:args.input_len + args.output_len]

            optimizer.zero_grad()
            pred = model(x_in)

            loss_data = criterion_data(pred, y_target)
            loss_phy = criterion_physics(pred)

            loss = loss_data + args.phy_weight * loss_phy
            loss.backward()
            optimizer.step()

            total_loss += loss.item() * len(batch)
            total_data_loss += loss_data.item() * len(batch)
            total_phy_loss += loss_phy.item() * len(batch)

        scheduler.step()
        n = len(dataset)
        print(f"Epoch {epoch:02d}/{args.epochs:02d} | "
              f"Loss: {total_loss/n:.6f} | Data: {total_data_loss/n:.6f} | "
              f"Phy: {total_phy_loss/n:.6e} | LR: {scheduler.get_last_lr()[0]:.2e}")

    # Export trained weights
    os.makedirs(os.path.dirname(args.output_weights), exist_ok=True)
    export_model_to_json(model, args.output_weights)
    print(f"AutoOrbit training completed. Exported model to {args.output_weights}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Train AutoOrbit FNO Model")
    parser.add_argument("--epochs", type=int, default=5)
    parser.add_argument("--lr", type=float, default=1e-3)
    parser.add_argument("--hidden", type=int, default=32)
    parser.add_argument("--modes", type=int, default=8)
    parser.add_argument("--layers", type=int, default=4)
    parser.add_argument("--input-len", type=int, default=64)
    parser.add_argument("--output-len", type=int, default=16)
    parser.add_argument("--cadence", type=float, default=10.0)
    parser.add_argument("--samples", type=int, default=100)
    parser.add_argument("--batch-size", type=int, default=16)
    parser.add_argument("--phy-weight", type=float, default=0.1)
    parser.add_argument("--output-weights", type=str, default="crates/sbm_core/models/autoorbit_trained.json")

    train(parser.parse_args())
