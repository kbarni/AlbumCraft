# Networks built into the program

- `yunet-2023mar-1.onnx` — the YuNet face detector (2023mar) of the OpenCV Zoo, by Shiqi Yu
  and others, MIT licence (`LICENSE-YuNet`). About 0.2 MB, so it is part of the program; the
  face recogniser (SFace, Apache 2.0) is the downloadable *Faces* pack (`../models.json`).

`prepare_faces.py` makes both from the OpenCV Zoo files (see its first lines): YuNet's input
size is made free (the published file says 640 × 640) and SFace's weights are taken out of its
input list. Nothing else changes; the outputs equal OpenCV's own `FaceDetectorYN` and
`FaceRecognizerSF` (checked on 265 faces: boxes and landmarks to the pixel, vectors to a cosine
of 1.0). The int8 SFace of the Zoo is not used: its vectors differ from the full one's
(cosine 0.96 on average, 0.80 at worst).
