"""What OpenCV's own face classes find in pictures, for `real_faces_match_opencv` (faces.rs).

    python reference_faces.py <folder of pictures> <folder of OpenCV Zoo files> <output folder> [count]

Writes <output folder>/reference.json and the pictures as given to OpenCV (PNG, longest side at
most 1024 px), so the test reads exactly the same pixels. Needs `opencv-python` and `numpy`.
The output holds face vectors of the people in those pictures: keep it to yourself, and
delete it after the check.
"""
import sys, os, json, glob, random, cv2

def main(src, zoo, out, count=40):
    os.makedirs(out, exist_ok=True)
    files = [f for f in glob.glob(os.path.join(src, "**", "*"), recursive=True) if f.lower().endswith((".jpg", ".jpeg", ".png"))]
    random.seed(1)
    random.shuffle(files)
    rec = cv2.FaceRecognizerSF.create(os.path.join(zoo, "face_recognition_sface_2021dec.onnx"), "")
    pictures = []
    for f in files:
        if len(pictures) >= int(count):
            break
        img = cv2.imread(f)
        if img is None:
            continue
        k = min(1.0, 1024 / max(img.shape[:2]))
        if k < 1:
            img = cv2.resize(img, None, fx=k, fy=k, interpolation=cv2.INTER_AREA)
        h, w = img.shape[:2]
        det = cv2.FaceDetectorYN.create(os.path.join(zoo, "face_detection_yunet_2023mar.onnx"), "", (w, h), 0.6, 0.3, 5000)
        _, found = det.detect(img)
        if found is None:
            continue
        path = os.path.join(out, f"{len(pictures):03}.png")
        cv2.imwrite(path, img)
        faces = []
        for d in found:
            v = rec.feature(rec.alignCrop(img, d)).flatten()
            faces.append({"rect": [float(x) for x in d[:4]], "landmarks": [[float(d[4 + 2 * n]), float(d[5 + 2 * n])] for n in range(5)], "score": float(d[14]), "vector": [float(x) for x in v]})
        pictures.append({"path": os.path.abspath(path), "width": w, "height": h, "faces": faces})
    with open(os.path.join(out, "reference.json"), "w") as o:
        json.dump(pictures, o)
    print(len(pictures), "pictures,", sum(len(p["faces"]) for p in pictures), "faces")

if __name__ == "__main__":
    main(*sys.argv[1:5])
