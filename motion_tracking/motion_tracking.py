#!/usr/bin/env python3
import argparse
import sys
from pathlib import Path
import cv2 as cv

VK_ESC = 27

GOTURN_FILES = Path(__file__).with_name('goturn-files')
GOTURL_PROTOTXT = Path(__file__).with_name('goturn.prototxt')
GOTURN_CAFFEMODEL = Path(__file__).with_name('goturn.caffemodel')

TRACKERS = {
    'tld': cv.TrackerTLD_create,
    'kcf': cv.TrackerKCF_create,
    'goturn': cv.TrackerGOTURN_create,
}

cv.setUseOptimized(True)

def main():
    def tup4(s: str):
        try:
            a, b, c, d = map(int, s.split(','))
            return a, b, c, d
        except ValueError:
            raise argparse.ArgumentTypeError('Expecting `x,y,w,h`')
    def int_pos(s: str):
        try:
            v = int(s)
            assert v > 0
            return v
        except (ValueError, AssertionError):
            raise argparse.ArgumentTypeError('Expecting positive integer')

    parser = argparse.ArgumentParser()
    parser.add_argument('tracker', choices=list(TRACKERS.keys()) + ['init-goturn'], help='Tracker')
    parser.add_argument('input', type=Path, help='Video path or directory of images')
    parser.add_argument('-o', '--output', type=Path, default=None, help='Output file to store result')
    parser.add_argument('-t', '--delay', type=int_pos, default=10, help='Delay between frames in ms')
    parser.add_argument('-b', '--init_bbox', type=tup4, default=None, help='Initial bounding box, in format x,y,h,w')
    args = parser.parse_args()

    if args.tracker == 'init-goturn':
        init_goturn_model()
        return

    if args.tracker == 'goturn':
        assert GOTURN_CAFFEMODEL.exists(), 'Run with argument `init-goturn` to initialize GOTURN model first'

    tracker = TRACKERS[args.tracker]()
    runner = TrackerRunner(tracker)
    if args.input.is_dir():
        runner.open_image_dir(args.input)
    else:
        video = cv.VideoCapture(str(args.input))
        assert video.isOpened(), 'Cannot open video file'
        runner.open_video(video)
    runner.run(args.delay, args.init_bbox)

    if args.output is not None:
        with open(args.output, 'w') as fout:
            for b in runner.result:
                if b is not None:
                    fout.write(f'{b[0]},{b[1]},{b[2]},{b[3]}\n')
                else:
                    fout.write('0,0,0,0\n')

def init_goturn_model():
    from zipfile import ZipFile
    from tempfile import TemporaryFile

    with TemporaryFile('wb+') as zipfp:
        for path in sorted(GOTURN_FILES.glob('goturn.caffemodel.zip.*')):
            zipfp.write(path.read_bytes())
        with ZipFile(zipfp, 'r') as zip:
            data = zip.read('goturn.caffemodel')
    GOTURN_CAFFEMODEL.write_bytes(data)
    with open(GOTURN_FILES / 'goturn.prototxt', 'rb') as fin:
        GOTURL_PROTOTXT.write_bytes(fin.read())

class TrackerRunner(object):
    def __init__(self, tracker: cv.Tracker):
        self.tracker = tracker
        self.frames = None
        self.result = []

    def open_video(self, video: cv.VideoCapture):
        def gen(video):
            while True:
                ok, frame = video.read()
                if not ok:
                    break
                yield frame
        self.frames = gen(video)

    def open_image_dir(self, dir: Path):
        def gen(dir):
            for path in sorted(dir.glob('*')):
                yield cv.imread(str(path))
        self.frames = gen(dir)

    def set_init_bbox(self, bbox: (int, int, int, int)):
        self.init_bbox = bbox

    def _init_tracker(self, init_bbox):
        if init_bbox is None:
            frame = next(self.frames, None)
            assert frame is not None, 'Video has no frames'
            init_bbox = cv.selectROI('Select object', frame, False)
            cv.destroyWindow('Select object')
            # Ensure result length to match input frames.
            self.result.append(init_bbox)
        assert self.tracker.init(frame, init_bbox)

    def _process_frame(self, frame):
        ok, bbox = self.tracker.update(frame)

        # Draw
        if ok:
            p1 = (int(bbox[0]), int(bbox[1]))
            p2 = (int(bbox[0] + bbox[2]), int(bbox[1] + bbox[3]))
            cv.rectangle(frame, p1, p2, (255, 0, 0), 2, 1)
        else:
            cv.putText(frame, "Tracking failure detected", (100, 80), cv.FONT_HERSHEY_SIMPLEX, 0.75, (0, 0, 255), 2)

        cv.imshow('Tracking', frame)
        return bbox if ok else None

    def run(self, delay_ms, init_bbox = None):
        assert delay_ms > 0
        self._init_tracker(init_bbox)
        for frame in self.frames:
            bbox = self._process_frame(frame)
            self.result.append(bbox)
            if cv.waitKey(delay_ms) == VK_ESC:
                raise InterruptedError('User canceled')

if __name__ == '__main__':
    main()
