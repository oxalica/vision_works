#!/usr/bin/env python3
import sys
from pathlib import Path
import cv2 as cv
import numpy as np

def main():
    args = sys.argv[1:]
    if len(args) != 3:
        print(f'''
Usage: {sys.argv[0]} <camera_parameter_file> <image> <output_path>
''')
        exit(1)
    param_path, img_path, out_path = map(Path, args)

    with open(param_path, 'r') as fin:
        def read():
            return np.array(list(map(float, fin.readline().split())), dtype=np.float32)
        mtx = read().reshape((3, 3))
        dist = read().reshape((1, 5))

    img = cv.imread(str(img_path))
    h, w = img.shape[:2]
    new_mtx, (x, y, w, h) = cv.getOptimalNewCameraMatrix(mtx, dist, (w, h), 1, (w, h))

    img_undist = cv.undistort(img, mtx, dist, None, new_mtx)
    # Crop
    img_undist = img_undist[y:(y + h), x:(x + w)]

    cv.imwrite(str(out_path), img_undist)

if __name__ == '__main__':
    main()
