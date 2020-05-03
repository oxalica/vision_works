#!/usr/bin/env python3
import math
import subprocess
import sys
from tempfile import NamedTemporaryFile
from pathlib import Path
from typing import List, Tuple

MOTION_TRACKING_EXEC = Path(__file__).with_name('motion_tracking.py').absolute()
TRACKING_METHODS = 'tld kcf goturn'.split()
BBox = Tuple[float, float, float, float]

def main():
    args = sys.argv[1:]
    if len(args) not in (3, 4):
        print(
            f'Usage: {sys.argv[0]} <otb_dataset_dir> <vot_dataset_dir> <result_dir> [filter]',
            file=sys.stderr,
        )
        exit(1)
    otb_dir, vot_dir, result_dir = map(Path, args[:3])
    filter = args[3] if len(args) == 4 else ''
    assert otb_dir.is_dir() and vot_dir.is_dir(), 'Invalid path'
    result_dir.mkdir(exist_ok=True)

    # Initialize
    subprocess.check_call([MOTION_TRACKING_EXEC, 'init-goturn'])

    for dir in otb_dir.iterdir():
        for method in TRACKING_METHODS:
            name = f'OTB-{dir.name}-{method}'
            output = result_dir / f'{name}.txt'
            if filter in name and not output.exists():
                print(f'Testing {name}')
                with open(dir / 'groundtruth_rect.txt', 'r') as fin:
                    bbox = [otb_parse_bbox(line) for line in fin]
                run_test(method, dir / 'img', output, bbox)

    for dir in vot_dir.iterdir():
        for method in TRACKING_METHODS:
            name = f'VOT-{dir.name}-{method}'
            output = result_dir / f'{name}.txt'
            if filter in name and not output.exists():
                print(f'Testing {name}')
                with open(dir / 'groundtruth.txt', 'r') as fin:
                    bbox = [vot_parse_bbox(line) for line in fin]
                run_test(method, dir, output, bbox)

def otb_parse_bbox(line: str) -> BBox:
    line = line.strip()
    if ',' in line:
        return tuple(map(float, line.split(',')))
    elif '\t' in line:
        return tuple(map(float, line.split('\t')))
    else:
        raise ValueError(f'Unknow bbox format: `{line}`')

def vot_parse_bbox(line: str) -> BBox:
    pts = list(map(float, line.split(',')))
    xs, ys = pts[::2], pts[1::2]
    x1, y1, x2, y2 = min(xs), min(ys), max(xs), max(ys)
    return x1, y1, x2 - x1, y2 - y1

def run_test(method: str, input: Path, output: Path, bbox: List[BBox]):
    init_bbox = ','.join(map(str, bbox[0]))
    result = subprocess.check_output([
        MOTION_TRACKING_EXEC,
        'run', method,
        '-b', init_bbox,
        '-t', '1',
        input,
    ])
    result = [tuple(map(float, line.split(','))) for line in result.decode().splitlines()]
    assert len(bbox) == len(result), f'Length mismatch. Input: {len(bbox)}, output: {len(result)}'

    with open(output, 'w') as fout:
        for expect, got in zip(bbox, result):
            dist, cover = bbox_diff(expect, got)
            fout.write(f'{dist},{cover}\n')

# Return: (center_distance, cover_ratio)
def bbox_diff(a: BBox, b: BBox) -> Tuple[float, float]:
    x1, y1, x2, y2 = a[0], a[1], a[0] + a[2], a[1] + a[3]
    x3, y3, x4, y4 = b[0], b[1], b[0] + b[2], b[1] + b[3]
    center_dist = math.hypot((x1 + x2) - (x3 + x4), (y1 + y2) - (y3 + y4)) / 2

    ix1, iy1 = max(x1, x3), max(y1, y3)
    ix2, iy2 = min(x2, x4), min(y2, y4)
    if ix1 < ix2 and iy1 < iy2:
        intersect_area = (ix2 - ix1) * (iy2 - iy1)
    else:
        # No intersection.
        intersect_area = 0
    sum_area = (x2 - x1) * (y2 - y1) + (x4 - x3) * (y4 - y3)
    union_area = sum_area - intersect_area
    return center_dist, intersect_area / union_area

if __name__ == '__main__':
    main()
