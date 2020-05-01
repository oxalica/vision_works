import sys
from pathlib import Path
import cv2 as cv

VK_ESC = 27

GOTURN_FILES = Path(__file__).with_name('goturn-files')
GOTURL_PROTOTXT = Path(__file__).with_name('goturn.prototxt')
GOTURN_CAFFEMODEL = Path(__file__).with_name('goturn.caffemodel')
FRAME_DELAY_MS = 10

TRACKERS = {
    'tld': cv.TrackerTLD_create,
    'kcf': cv.TrackerKCF_create,
    'goturn': cv.TrackerGOTURN_create,
}

cv.setUseOptimized(True)

def main():
    args = sys.argv[1:]
    if args == ['init-goturn']:
        init_goturn_model()

    elif len(args) == 2 and args[0] in TRACKERS:
        if args[0] == 'goturn':
            assert GOTURN_CAFFEMODEL.exists(), 'Run with argument `init-goturn` to initialize GOTURN model first'

        tracker = TRACKERS[args[0]]()
        path = Path(args[1])
        runner = TrackerRunner(tracker)
        if path.is_dir():
            runner.open_image_dir(path)
        else:
            video = cv.VideoCapture(str(path))
            assert video.isOpened(), 'Cannot open video file'
            runner.open_video(video)
        runner.run()

    else:
        print(f'''
Usage:
python3 . init-goturn
    Initialize GOTURN tracker.

python3 . <tld|kcf|goturn> <video_path|image_dir>
    Run the specific tracker on the given video or images.
''')
        exit(1)

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
        self.init_bbox = None

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

    def _init_tracker(self):
        if self.init_bbox is None:
            frame = next(self.frames, None)
            assert frame is not None, 'Video has no frames'
            self.init_bbox = cv.selectROI('Select object', frame, False)
            cv.destroyWindow('Select object')
        assert self.tracker.init(frame, self.init_bbox)

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

    def run(self):
        self._init_tracker()
        for frame in self.frames:
            self._process_frame(frame)
            if FRAME_DELAY_MS != 0 and cv.waitKey(FRAME_DELAY_MS) == VK_ESC:
                raise InterruptedError('User canceled')

if __name__ == '__main__':
    main()
