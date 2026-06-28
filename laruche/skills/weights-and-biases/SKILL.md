---
type: skill
name: weights-and-biases
description: "Log ML experiments, sweeps, artifacts, and model registry via W&B."
version: 1.0.1
author: Orchestra Research
license: MIT
dependencies: [wandb]
platforms: [linux, macos, windows]
tools: [execute_code, shell_exec]
metadata:
  laruche:
    tags: [MLOps, WandB, ExperimentTracking, HyperparameterTuning, ModelRegistry, PyTorch, TensorFlow, HuggingFace]
    homepage: https://docs.wandb.ai

---

# Weights & Biases: ML Experiment Tracking

## Prerequisites

```bash
pip install wandb
# Authenticate - WANDB_API_KEY is injected from LaRuche secrets vault
export WANDB_API_KEY=${WANDB_API_KEY}
wandb login --relogin   # optional: verify auth
```

> Use `execute_code` for Python snippets; use `shell_exec` for CLI commands.

---

## Core Pattern: Instrument a Training Run

```python
import wandb

run = wandb.init(
    project="my-project",
    name="resnet50-lr0.001-bs32",   # descriptive name beats "run1"
    tags=["baseline", "resnet"],
    group="resnet-experiments",      # group related runs
    job_type="train",
    config={
        "learning_rate": 0.001,
        "batch_size": 32,
        "epochs": 50,
        "architecture": "ResNet50",
    },
)

for epoch in range(run.config.epochs):
    train_loss = train_epoch()
    val_loss, val_acc = validate()

    wandb.log({
        "epoch": epoch,
        "train/loss": train_loss,
        "val/loss": val_loss,
        "val/accuracy": val_acc,
    })

wandb.finish()
print(f"Run URL: {run.url}")
```

**Failure handling:**
- If `wandb.init` hangs, set `WANDB_MODE=offline` and sync later with `wandb sync <run_dir>`.
- On unstable connections: `os.environ["WANDB_MODE"] = "offline"` before `init`.

---

## Metric & Media Logging

```python
# Scalars with explicit step
wandb.log({"loss": loss, "lr": current_lr}, step=global_step)

# Images, histograms, tables
wandb.log({
    "examples": [wandb.Image(img) for img in images],
    "grad_hist": wandb.Histogram(gradients),
    "conf_mat": wandb.plot.confusion_matrix(
        y_true=ground_truth, preds=predictions, class_names=class_names
    ),
})

# Prediction table
table = wandb.Table(columns=["id", "pred", "truth"], data=rows)
wandb.log({"predictions": table})
```

---

## Hyperparameter Sweeps

```python
sweep_config = {
    "method": "bayes",          # "grid" | "random" | "bayes" (recommended)
    "metric": {"name": "val/accuracy", "goal": "maximize"},
    "parameters": {
        "learning_rate": {"distribution": "log_uniform", "min": 1e-5, "max": 1e-1},
        "batch_size":    {"values": [16, 32, 64, 128]},
        "optimizer":     {"values": ["adam", "sgd", "rmsprop"]},
        "dropout":       {"distribution": "uniform", "min": 0.1, "max": 0.5},
    },
}

sweep_id = wandb.sweep(sweep_config, project="my-project")

def train():
    run = wandb.init()
    cfg = wandb.config
    model = build_model(cfg)
    for epoch in range(NUM_EPOCHS):
        loss = train_epoch(model, cfg.learning_rate, cfg.batch_size)
        acc  = validate(model)
        wandb.log({"train/loss": loss, "val/accuracy": acc})

wandb.agent(sweep_id, function=train, count=50)
```

---

## Artifacts (Datasets & Models)

```python
# --- Log a dataset artifact ---
artifact = wandb.Artifact("training-dataset", type="dataset",
                           metadata={"size": "1.2M", "split": "train"})
artifact.add_file("data/train.csv")
artifact.add_dir("data/images/")
wandb.log_artifact(artifact)

# --- Consume an artifact ---
run = wandb.init(project="my-project")
artifact = run.use_artifact("training-dataset:latest")
artifact_dir = artifact.download()

# --- Save a model to the registry ---
model_artifact = wandb.Artifact("resnet50-model", type="model",
                                 metadata={"accuracy": 0.95})
model_artifact.add_file("model.pth")
wandb.log_artifact(model_artifact, aliases=["best", "production"])
run.link_artifact(model_artifact, "model-registry/production-models")
```

---

## Framework Integrations (One-liners)

**HuggingFace Trainer** - add `report_to="wandb"` to `TrainingArguments`.

**PyTorch Lightning** - pass `logger=WandbLogger(project="...", log_model=True)` to `Trainer`.

**Keras/TensorFlow** - add `WandbCallback()` to `model.fit(..., callbacks=[...])`.

---

## Key Gotchas

| Situation | Fix |
|---|---|
| No API key at runtime | Ensure `${WANDB_API_KEY}` is set in LaRuche secrets vault |
| Run hangs on slow network | `WANDB_MODE=offline`; sync later: `wandb sync ./wandb/` |
| Too many sweep trials | Use `count=N` on `wandb.agent` to cap |
| Duplicate metric keys | Use namespaced keys: `train/loss`, `val/loss` |
| Large artifact uploads | Use `artifact.add_reference("s3://...")` instead of `add_file` |

---

## Resources

- Docs: https://docs.wandb.ai
- GitHub: https://github.com/wandb/wandb
- Examples: https://github.com/wandb/examples
