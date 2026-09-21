"""
Export PyTorch AutoOrbit FNO weights to JSON format for the Rust inference engine.
"""

import json
import os
import torch
from fno_pytorch import AutoOrbitFNO1d


def export_model_to_json(model: AutoOrbitFNO1d, output_path: str):
    """Serializes an AutoOrbitFNO1d PyTorch model into a portable JSON file."""
    data = {
        "model_name": "AutoOrbitFNO1d",
        "in_channels": model.lifting.in_channels,
        "out_channels": model.projection.out_channels,
        "hidden_channels": model.hidden_channels,
        "n_modes": model.n_modes,
        "input_length": model.input_length,
        "output_length": model.output_length,
        "lifting": {
            "weight": model.lifting.weight.squeeze(-1).detach().cpu().numpy().tolist(),
            "bias": model.lifting.bias.detach().cpu().numpy().tolist(),
        },
        "encoder_layers": [],
        "decoder_layers": [],
        "projection": {
            "weight": model.projection.weight.squeeze(-1).detach().cpu().numpy().tolist(),
            "bias": model.projection.bias.detach().cpu().numpy().tolist(),
        }
    }

    # Export encoder layers
    for layer in model.encoder:
        layer_dict = {
            "spectral": {
                "n_modes": layer.spectral_conv.n_modes,
                "weights_real": layer.spectral_conv.weights_real.detach().cpu().numpy().tolist(),
                "weights_imag": layer.spectral_conv.weights_imag.detach().cpu().numpy().tolist(),
            },
            "skip": {
                "weight": layer.skip_conv.weight.squeeze(-1).detach().cpu().numpy().tolist(),
                "bias": layer.skip_conv.bias.detach().cpu().numpy().tolist(),
            },
            "activation": "GELU" if isinstance(layer.activation, torch.nn.GELU) else "ReLU"
        }
        data["encoder_layers"].append(layer_dict)

    # Export decoder layers
    for layer in model.decoder:
        layer_dict = {
            "spectral": {
                "n_modes": layer.spectral_conv.n_modes,
                "weights_real": layer.spectral_conv.weights_real.detach().cpu().numpy().tolist(),
                "weights_imag": layer.spectral_conv.weights_imag.detach().cpu().numpy().tolist(),
            },
            "skip": {
                "weight": layer.skip_conv.weight.squeeze(-1).detach().cpu().numpy().tolist(),
                "bias": layer.skip_conv.bias.detach().cpu().numpy().tolist(),
            },
            "activation": "GELU" if isinstance(layer.activation, torch.nn.GELU) else "ReLU"
        }
        data["decoder_layers"].append(layer_dict)

    os.makedirs(os.path.dirname(os.path.abspath(output_path)), exist_ok=True)
    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(data, f, indent=2)

    print(f"Exported model weights successfully to {output_path}")


if __name__ == "__main__":
    # Test export of a calibrated AutoOrbit model
    model = AutoOrbitFNO1d(
        in_channels=6,
        out_channels=6,
        hidden_channels=32,
        n_modes=8,
        n_layers=4,
        input_length=128,
        output_length=60
    )
    export_model_to_json(model, "crates/sbm_core/models/autoorbit_s1a.json")
