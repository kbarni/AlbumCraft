"""Prepares the face networks AlbumCraft uses, from the OpenCV Zoo files.

    python prepare_faces.py <folder with the OpenCV Zoo files> <output folder>

Input files (https://github.com/opencv/opencv_zoo, also https://huggingface.co/opencv):
  face_detection_yunet_2023mar.onnx          (YuNet, MIT)
  face_recognition_sface_2021dec.onnx        (SFace, Apache 2.0)
  face_recognition_sface_2021dec_int8.onnx   (SFace quantised, Apache 2.0)

What is changed, nothing else:
- YuNet: the input's height and width, and the outputs' number of cells, are made free
  (the file says 640 x 640, but the network is fully convolutional and OpenCV runs it at any
  size that is a multiple of 32).
- SFace: the weights are taken out of the list of graph inputs (an old way of exporting that
  makes every weight look like an input the caller must give); they stay as initialisers.

Needs the `onnx` Python package.
"""
import sys, pathlib, onnx

def free_dims(model, input_dims, output_dims):
    g = model.graph
    for i in g.input:
        dims = i.type.tensor_type.shape.dim
        for k, name in input_dims.items():
            dims[k].ClearField("dim_value")
            dims[k].dim_param = name
    for o in g.output:
        dims = o.type.tensor_type.shape.dim
        for k, name in output_dims.items():
            dims[k].ClearField("dim_value")
            dims[k].dim_param = name + "_" + o.name.split("_")[-1]
    # shapes inside the graph were inferred for 640 x 640: drop them
    del g.value_info[:]
    return model

def drop_weight_inputs(model):
    g = model.graph
    weights = {t.name for t in g.initializer}
    keep = [i for i in g.input if i.name not in weights]
    del g.input[:]
    g.input.extend(keep)
    return model

def main(src, out):
    src, out = pathlib.Path(src), pathlib.Path(out)
    out.mkdir(parents=True, exist_ok=True)
    yunet = free_dims(onnx.load(src / "face_detection_yunet_2023mar.onnx"), {2: "height", 3: "width"}, {1: "cells"})
    onnx.checker.check_model(yunet)
    onnx.save(yunet, out / "yunet-2023mar-1.onnx")
    for name, dest in [("face_recognition_sface_2021dec.onnx", "sface-1.onnx"), ("face_recognition_sface_2021dec_int8.onnx", "sface-2021dec-int8-1.onnx")]:
        m = drop_weight_inputs(onnx.load(src / name))
        onnx.checker.check_model(m)
        onnx.save(m, out / dest)

if __name__ == "__main__":
    main(*sys.argv[1:3])
