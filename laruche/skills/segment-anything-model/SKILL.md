---
type: skill
name: segment-anything-model
description: "Zero-shot image segmentation via points, boxes, or auto-masks (SAM)."
version: 1.1.0
author: Orchestra Research
license: MIT
dependencies: [segment-anything, transformers>=4.30.0, torch>=1.7.0]
platforms: [linux, macos, windows]
tools: [execute_code, shell_exec, file_write]
metadata:
  laruche:
    tags: [Multimodal, Image Segmentation, Computer Vision, SAM, Zero-Shot]

---

# Segment Anything Model (SAM)

Meta AI's SAM segments any object in images without task-specific training. Use `execute_code` or `shell_exec` to run the snippets below inside LaRuche.

**When to use SAM:** point/box-prompted annotation, generating training masks, zero-shot domain transfer (medical, satellite, etc.).
**Prefer alternatives for:** real-time detection with class labels (YOLO), semantic/panoptic segmentation (Mask2Former), text-prompted segmentation (GroundingDINO+SAM), video (SAM 2).

---

## 1. Install

```bash
pip install git+https://github.com/facebookresearch/segment-anything.git
pip install opencv-python pycocotools matplotlib
# or HuggingFace route:
pip install transformers
```

## 2. Download a checkpoint

| Model | Key | Size | Notes |
|-------|-----|------|-------|
| ViT-H | `vit_h` | 2.4 GB | Most accurate |
| ViT-L | `vit_l` | 1.2 GB | Balanced |
| ViT-B | `vit_b` | 375 MB | Fastest, use when VRAM is limited |

```bash
wget https://dl.fbaipublicfiles.com/segment_anything/sam_vit_h_4b8939.pth
wget https://dl.fbaipublicfiles.com/segment_anything/sam_vit_l_0b3195.pth
wget https://dl.fbaipublicfiles.com/segment_anything/sam_vit_b_01ec64.pth
```

## 3. Prompted segmentation (SamPredictor)

```python
import cv2, numpy as np
from segment_anything import sam_model_registry, SamPredictor

sam = sam_model_registry["vit_h"](checkpoint="sam_vit_h_4b8939.pth")
sam.to(device="cuda")
predictor = SamPredictor(sam)

image = cv2.cvtColor(cv2.imread("image.jpg"), cv2.COLOR_BGR2RGB)
predictor.set_image(image)  # Encode once; reuse for multiple prompts

# --- Point prompt ---
masks, scores, logits = predictor.predict(
    point_coords=np.array([[500, 375]]),  # (x, y)
    point_labels=np.array([1]),            # 1=foreground, 0=background
    multimask_output=True                  # returns 3 candidates
)
best_mask = masks[np.argmax(scores)]

# --- Box prompt ---
masks, scores, logits = predictor.predict(
    box=np.array([425, 600, 700, 875]),    # [x1, y1, x2, y2]
    multimask_output=False
)

# --- Combined box + point ---
masks, scores, logits = predictor.predict(
    point_coords=np.array([[500, 375]]),
    point_labels=np.array([1]),
    box=np.array([400, 300, 700, 600]),
    multimask_output=False
)

# --- Iterative refinement (use previous mask logits) ---
masks, scores, logits = predictor.predict(
    point_coords=np.array([[500, 375], [550, 400]]),
    point_labels=np.array([1, 0]),
    mask_input=logits[np.argmax(scores)][None],
    multimask_output=False
)
```

## 4. Automatic mask generation

```python
from segment_anything import SamAutomaticMaskGenerator

mask_generator = SamAutomaticMaskGenerator(
    model=sam,
    points_per_side=32,           # Grid density; lower = faster
    pred_iou_thresh=0.88,
    stability_score_thresh=0.95,
    min_mask_region_area=100,     # Drops tiny noise masks
)
masks = mask_generator.generate(image)

# Each mask dict keys: segmentation (H×W bool), bbox [x,y,w,h],
# area (px), predicted_iou (0-1), stability_score (0-1)

# Useful filters:
high_quality = [m for m in masks if m["predicted_iou"] > 0.9]
large_first   = sorted(masks, key=lambda m: m["area"], reverse=True)
```

## 5. HuggingFace route

```python
import torch
from PIL import Image
from transformers import SamModel, SamProcessor

model = SamModel.from_pretrained("facebook/sam-vit-huge").to("cuda")
processor = SamProcessor.from_pretrained("facebook/sam-vit-huge")

image = Image.open("image.jpg")
inputs = processor(image, input_points=[[[450, 600]]], return_tensors="pt")
inputs = {k: v.to("cuda") for k, v in inputs.items()}

with torch.no_grad():
    outputs = model(**inputs)

masks = processor.image_processor.post_process_masks(
    outputs.pred_masks.cpu(),
    inputs["original_sizes"].cpu(),
    inputs["reshaped_input_sizes"].cpu()
)
```

## 6. Object extraction (RGBA cutout)

```python
def extract_object(image, point):
    predictor.set_image(image)
    masks, scores, _ = predictor.predict(
        point_coords=np.array([point]),
        point_labels=np.array([1]),
        multimask_output=True
    )
    best = masks[np.argmax(scores)]
    rgba = np.zeros((*image.shape[:2], 4), dtype=np.uint8)
    rgba[:, :, :3] = image
    rgba[:, :, 3] = best * 255
    return rgba
```

## 7. ONNX export (edge / browser deployment)

```bash
python scripts/export_onnx_model.py \
    --checkpoint sam_vit_h_4b8939.pth \
    --model-type vit_h \
    --output sam_onnx.onnx \
    --return-single-mask
```

```python
import onnxruntime, numpy as np
ort = onnxruntime.InferenceSession("sam_onnx.onnx")
masks = ort.run(None, {
    "image_embeddings": image_embeddings,
    "point_coords": point_coords,
    "point_labels": point_labels,
    "mask_input": np.zeros((1, 1, 256, 256), dtype=np.float32),
    "has_mask_input": np.array([0], dtype=np.float32),
    "orig_im_size": np.array([h, w], dtype=np.float32)
})
```

## 8. Performance tips & pitfalls

| Issue | Fix |
|-------|-----|
| Out of VRAM | Switch to ViT-B; add `sam = sam.half()` |
| Slow auto-generation | Lower `points_per_side` (e.g. 16) |
| Poor mask quality | Combine box + point prompts |
| Edge artifacts | Filter on `stability_score > 0.95` |
| Small objects missed | Increase `points_per_side`; add `crop_n_layers=1` |
| Grayscale input | Convert first: `cv2.cvtColor(img, cv2.COLOR_GRAY2RGB)` |

**Memory:** call `torch.cuda.empty_cache()` between large batches.
**Efficiency:** `predictor.set_image()` runs the encoder once - loop prompts without re-encoding.

## Resources

- GitHub: https://github.com/facebookresearch/segment-anything
- Paper: https://arxiv.org/abs/2304.02643
- HuggingFace: https://huggingface.co/facebook/sam-vit-huge
- SAM 2 (video): https://github.com/facebookresearch/segment-anything-2
