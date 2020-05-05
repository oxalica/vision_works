#!/usr/bin/env python3
import sys
from pathlib import Path
from typing import Tuple, List
import cv2 as cv
import numpy as np

DISPLAY_MAX = 800
VK_SPACE = 32

def main():
    args = sys.argv[1:]
    quiet = args[0] == '-q'
    if quiet:
        args = args[1:]

    if len(args) < 4 or args[0] not in ('chessboard', 'circles'):
        print(f'''
USAGE: {sys.argv[0]} [-q] {{chessboard|circles}} <grid_w> <grid_h> <result_path> <image_paths...>
''')
        return

    method = args[0]
    grid_size = tuple(map(int, args[1:3]))
    assert grid_size[0] != grid_size[1], 'Grid should not be symmentric'
    result_path = Path(args[3])
    img_paths = list(map(Path, args[4:]))

    mtx, dist = calibrate_camera(grid_size, img_paths, method, quiet)
    with open(result_path, 'w') as fout:
        assert mtx.shape, (3, 3)
        fout.write(f'{" ".join(map(str, mtx.flatten()))}\n')
        assert dist.shape, (1, 5)
        fout.write(f'{" ".join(map(str, dist.flatten()))}\n')

def gen_circle_grid_obj_pts(w, h):
    ret = [[(2 * j) + i % 2, i, 0] for i in range(w) for j in range(h)]
    return np.array(ret, dtype=np.float32)

def gen_chessboard_grid_obj_pts(w, h):
    ret = [[i, j, 0] for i in range(w) for j in range(h)]
    return np.array(ret, dtype=np.float32)

def calibrate_camera(grid_size: Tuple[int, int], img_paths: List[Path], method: str, quiet: bool):
    if method == 'chessboard':
        expect_obj_pts = gen_chessboard_grid_obj_pts(*grid_size)
    else:
        expect_obj_pts = gen_circle_grid_obj_pts(*grid_size)

    img_size = None
    img_pts = []
    for img_path in img_paths:
        print(f'Loading {img_path.name}...')
        img_orig = cv.imread(str(img_path))
        img = cv.cvtColor(img_orig, cv.COLOR_BGR2GRAY)

        size = img.shape[:2][::-1]
        if img_size is None:
            img_size = size
        else:
            assert img_size == size, 'Size mismatch'

        if method == 'chessboard':
            ok, corners = cv.findChessboardCorners(img, grid_size, None)
            if not ok:
                print(f'Chessboard not found.')
                continue
            criteria = (cv.TERM_CRITERIA_EPS + cv.TERM_CRITERIA_MAX_ITER, 30, 0.001)
            corners = cv.cornerSubPix(img, corners, (11, 11), (-1, -1), criteria)
            img_pts.append(corners)

            if not quiet:
                cv.drawChessboardCorners(img_orig, grid_size, corners, True)
                display_img(img_orig, img_path.name)

        else:
            ok, centers = cv.findCirclesGrid(
                img,
                grid_size,
                flags=cv.CALIB_CB_ASYMMETRIC_GRID | cv.CALIB_CB_CLUSTERING,
            )
            if not ok:
                print(f'Circle grid not found.')
                continue
            centers = centers.reshape((centers.shape[0], 2))
            img_pts.append(centers)

            if not quiet:
                fst_center = centers[0][0], centers[0][1]
                cv.drawMarker(img_orig, fst_center, (255, 255, 255), markerSize=16, thickness=4)
                for (x1, y1), (x2, y2) in zip(centers, centers[1:]):
                    cv.line(img_orig, (x1, y1), (x2, y2), (127, 127, 127), thickness=4)
                display_img(img_orig, img_path.name)

    print(f'Available images: {len(img_pts)}')
    assert img_size is not None
    print(f'Image size: {img_size[0]}x{img_size[1]}')

    obj_pts = [expect_obj_pts] * len(img_pts)
    ok, mtx, dist, _rvecs, _tvecs = cv.calibrateCamera(
        np.array(obj_pts, dtype=np.float32),
        np.array(img_pts, dtype=np.float32),
        img_size,
        None,
        None,
    )
    assert ok, 'Failed to calibrate cameta'
    return mtx, dist

def display_img(img, title):
    # Scale before display.
    h, w = img.shape[:2]
    if max(h, w) > DISPLAY_MAX:
        if h > w:
            # (w, h)
            dsize = w * DISPLAY_MAX // h, DISPLAY_MAX
        else:
            dsize = DISPLAY_MAX, h * DISPLAY_MAX // w
        img = cv.resize(img, dsize)

    cv.imshow(title, img)
    if cv.waitKey() != VK_SPACE:
        print('User canceled')
        exit(1)
    cv.destroyWindow(title)

if __name__ == '__main__':
    main()
